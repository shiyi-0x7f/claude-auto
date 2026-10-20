//! 业务编排：CLI 同步 → 轮询额度 → 评估策略 → 切换凭据。
//!
//! 锁约定：`op` 串行化所有改动类操作（轮询/切换/导入），网络请求期间持有它但不持有 `inner`；
//! `inner` 只做短暂的内存读写，前端读快照不会被网络阻塞。

use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use serde_json::Value;
use tauri::{AppHandle, Emitter};
use tauri_plugin_notification::NotificationExt;
use tokio::sync::Notify;

use crate::anthropic::{Client, Usage};
use crate::cli_files::{oauth, CliPaths};
use crate::error::{AppError, AppResult};
use crate::fsutil::now_ms;
use crate::login;
use crate::policy::{self, Exhaustion, Thresholds};
use crate::proxy;
use crate::settings::{Settings, SwitchMode};
use crate::state::{AccountStatus, Inner, Snapshot, Suggestion, SwitchRecord};

/// 非激活账号令牌剩余有效期低于此值时由本工具提前刷新
const REFRESH_MARGIN_MS: i64 = 10 * 60 * 1000;
pub const SNAPSHOT_EVENT: &str = "snapshot";

pub struct Service {
    inner: Mutex<Inner>,
    op: tokio::sync::Mutex<()>,
    /// HTTP 客户端与其生效代理（已脱敏），代理设置变更时整体重建
    net: Mutex<(Client, Option<String>)>,
    pub paths: CliPaths,
    pub wake: Notify,
    app: AppHandle,
}

fn build_net(settings: &Settings, paths: &CliPaths) -> AppResult<(Client, Option<String>)> {
    let conf = proxy::resolve(settings, &paths.config_dir);
    let label = conf.as_ref().map(|p| proxy::redact(&p.url));
    tracing::info!(
        proxy = label.as_deref().unwrap_or("直连"),
        "HTTP 客户端代理"
    );
    Ok((Client::new(conf.as_ref())?, label))
}

enum Trigger {
    Manual,
    Auto(Exhaustion),
    Confirmed(Exhaustion),
}

impl Trigger {
    fn label(&self) -> String {
        match self {
            Trigger::Manual => "手动切换".into(),
            Trigger::Auto(r) => format!("自动 · {}", r.label()),
            Trigger::Confirmed(r) => format!("确认切换 · {}", r.label()),
        }
    }
}

impl Service {
    pub fn new(app: AppHandle, inner: Inner, paths: CliPaths) -> AppResult<Self> {
        // 代理配置有误时退回直连启动，错误在界面上可见，用户可改设置修复
        let net = build_net(&inner.settings.value, &paths).unwrap_or_else(|e| {
            tracing::error!("代理配置无效，暂用直连: {e}");
            (Client::new(None).expect("直连客户端构建不会失败"), None)
        });
        Ok(Self {
            inner: Mutex::new(inner),
            op: tokio::sync::Mutex::new(()),
            net: Mutex::new(net),
            paths,
            wake: Notify::new(),
            app,
        })
    }

    pub fn lock(&self) -> MutexGuard<'_, Inner> {
        // 持锁代码不含 panic 路径；万一中毒仍取回数据继续服务
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn net(&self) -> (Client, Option<String>) {
        self.net.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn client(&self) -> Client {
        self.net().0
    }

    pub fn snapshot(&self) -> Snapshot {
        let proxy = self.net().1;
        self.lock()
            .snapshot(self.paths.config_dir.display().to_string(), proxy)
    }

    pub fn emit(&self) {
        let snap = self.snapshot();
        crate::tray::update_tooltip(&self.app, &snap);
        if let Err(e) = self.app.emit(SNAPSHOT_EVENT, snap) {
            tracing::warn!("推送快照失败: {e}");
        }
    }

    pub fn notify(&self, title: &str, body: &str) {
        if let Err(e) = self
            .app
            .notification()
            .builder()
            .title(title)
            .body(body)
            .show()
        {
            tracing::warn!("系统通知失败: {e}");
        }
    }

    /// 读取 CLI 当前凭据，确定激活账号，并把 CLI 刷新后的令牌同步回库。
    fn sync_from_cli(&self) -> AppResult<()> {
        let cli_oauth = self.paths.read_oauth()?;
        let cli_account = self.paths.read_account()?;
        let mut g = self.lock();
        g.cli_logged_in = cli_oauth.is_some();
        let Some(cli_oauth) = cli_oauth else {
            g.active_id = None;
            return Ok(());
        };
        let Some(id) = g
            .vault
            .find_matching(&cli_oauth, cli_account.as_ref())
            .map(|a| a.id.clone())
        else {
            g.active_id = None;
            return Ok(());
        };
        let acc = g.vault.get_mut(&id)?;
        let mut changed = false;
        if acc.oauth != cli_oauth {
            acc.oauth = cli_oauth;
            changed = true;
        }
        if cli_account.is_some() && acc.oauth_account != cli_account {
            acc.oauth_account = cli_account;
            changed = true;
        }
        if changed {
            g.vault.save()?;
            tracing::info!("已同步 CLI 刷新后的凭据到账号库");
        }
        g.active_id = Some(id);
        Ok(())
    }

    /// 非激活账号：令牌临近过期时刷新并落库。激活账号的令牌归 CLI 所有，绝不在此刷新。
    async fn ensure_fresh(&self, id: &str, current: &Value) -> AppResult<Value> {
        let expires = oauth::expires_at(current).unwrap_or(0);
        if expires - now_ms() > REFRESH_MARGIN_MS {
            return Ok(current.clone());
        }
        tracing::info!(account = id, "刷新非激活账号令牌");
        let next = self.client().refresh(current).await?;
        let mut g = self.lock();
        g.vault.get_mut(id)?.oauth = next.clone();
        g.vault.save()?;
        Ok(next)
    }

    async fn query_one(&self, id: &str, oauth_value: &Value, is_active: bool) -> AppResult<Usage> {
        let fresh = if is_active {
            oauth_value.clone()
        } else {
            self.ensure_fresh(id, oauth_value).await?
        };
        let token =
            oauth::access_token(&fresh).ok_or_else(|| AppError::Auth("缺少 accessToken".into()))?;
        self.client().fetch_usage(token).await
    }

    pub async fn poll(&self) {
        let _op = self.op.lock().await;
        if let Err(e) = self.poll_locked().await {
            tracing::error!("轮询失败: {e}");
            self.lock().last_error = Some(e.to_string());
        }
        self.emit();
    }

    async fn poll_locked(&self) -> AppResult<()> {
        self.sync_from_cli()?;
        let jobs: Vec<(String, Value, bool)> = {
            let g = self.lock();
            g.vault
                .data
                .accounts
                .iter()
                .map(|a| {
                    (
                        a.id.clone(),
                        a.oauth.clone(),
                        g.active_id.as_deref() == Some(&a.id),
                    )
                })
                .collect()
        };
        for (id, oauth_value, is_active) in jobs {
            let result = self.query_one(&id, &oauth_value, is_active).await;
            let mut g = self.lock();
            let st = g.statuses.entry(id.clone()).or_default();
            st.checked_at = Some(now_ms());
            match result {
                Ok(u) => {
                    *st = AccountStatus {
                        usage: Some(u),
                        error: None,
                        needs_relogin: false,
                        checked_at: st.checked_at,
                    };
                }
                Err(e) => {
                    tracing::warn!(account = %id, "查询额度失败: {e}");
                    st.needs_relogin = matches!(e, AppError::Auth(_)) && !is_active;
                    st.error = Some(e.to_string());
                    // 保留上次成功的额度数据，前端用 checkedAt/error 提示可能过期
                }
            }
        }
        {
            let mut g = self.lock();
            g.last_poll_at = Some(now_ms());
            g.last_error = None;
        }
        self.evaluate().await?;
        crate::alerts::check(self);
        Ok(())
    }

    /// 根据激活账号额度生成/清除切换建议；自动模式直接执行切换。
    async fn evaluate(&self) -> AppResult<()> {
        let now = now_ms();
        let decision = {
            let g = self.lock();
            let s = &g.settings.value;
            let th = Thresholds {
                five_hour: s.five_hour_threshold,
                seven_day: s.seven_day_threshold,
            };
            g.active_id.as_ref().and_then(|active| {
                let usage = g.statuses.get(active)?.usage.as_ref()?;
                let reason = policy::exhaustion(usage, th, now)?;
                let candidates = g.statuses.iter().filter_map(|(id, st)| {
                    let ok = id != active && !st.needs_relogin && st.error.is_none();
                    ok.then_some((id.as_str(), st.usage.as_ref()?))
                });
                let to_id = policy::pick_target(candidates, th, now).map(String::from);
                Some((
                    Suggestion {
                        from_id: active.clone(),
                        to_id,
                        reason,
                    },
                    s.mode,
                ))
            })
        };

        let Some((sugg, mode)) = decision else {
            let mut g = self.lock();
            g.pending = None;
            g.notified = None;
            return Ok(());
        };

        if let (SwitchMode::Auto, Some(to)) = (mode, sugg.to_id.clone()) {
            return self.switch_locked(&to, Trigger::Auto(sugg.reason)).await;
        }

        let first_time = {
            let mut g = self.lock();
            g.pending = Some(sugg.clone());
            let first = g.notified.as_ref() != Some(&sugg);
            g.notified = Some(sugg.clone());
            first
        };
        if first_time {
            let (from, to) = {
                let g = self.lock();
                (
                    g.display(&g.label_of(&sugg.from_id)),
                    sugg.to_id.as_deref().map(|id| g.display(&g.label_of(id))),
                )
            };
            match to {
                Some(to) => self.notify(
                    &format!("{} 已达{}", from, sugg.reason.label()),
                    &format!("建议切换到 {to}，打开 Claude Auto 或托盘菜单确认"),
                ),
                None => self.notify(
                    &format!("{} 已达{}", from, sugg.reason.label()),
                    "所有账号额度均已耗尽，等待窗口重置",
                ),
            }
        }
        Ok(())
    }

    pub async fn switch_to(&self, id: &str) -> AppResult<()> {
        let _op = self.op.lock().await;
        let result = self.switch_locked(id, Trigger::Manual).await;
        self.emit();
        result
    }

    /// 执行当前切换建议（半自动模式下用户确认）。
    pub async fn apply_pending(&self) -> AppResult<()> {
        let _op = self.op.lock().await;
        let pending = self.lock().pending.clone();
        let Some(Suggestion {
            to_id: Some(to),
            reason,
            ..
        }) = pending
        else {
            return Err(AppError::Invalid("当前没有可执行的切换建议".into()));
        };
        let result = self.switch_locked(&to, Trigger::Confirmed(reason)).await;
        self.emit();
        result
    }

    pub fn dismiss_pending(&self) {
        // 只清 pending，保留 notified：同一建议不会再次弹通知，状态变化后才会重新提示
        self.lock().pending = None;
        self.emit();
    }

    async fn switch_locked(&self, to: &str, trigger: Trigger) -> AppResult<()> {
        // 先把 CLI 里正在用的令牌（可能刚被 CLI 刷新过）收回库里，再交出新账号
        self.sync_from_cli()?;
        let (from, target) = {
            let g = self.lock();
            if g.active_id.as_deref() == Some(to) {
                return Err(AppError::Invalid("该账号已是当前账号".into()));
            }
            (g.active_id.clone(), g.vault.get(to)?.clone())
        };
        let fresh = self.ensure_fresh(to, &target.oauth).await?;
        self.paths.write_oauth(&fresh)?;
        self.paths.write_account(target.oauth_account.as_ref())?;

        let mut g = self.lock();
        let from_label = from.as_deref().map(|id| g.label_of(id));
        let shown = (
            from_label
                .as_deref()
                .map_or_else(|| "未纳管账号".into(), |l| g.display(l)),
            g.display(&target.label),
        );
        g.active_id = Some(to.to_string());
        g.pending = None;
        g.notified = None;
        g.push_history(SwitchRecord {
            at: now_ms(),
            from_label,
            to_label: target.label.clone(),
            trigger: trigger.label(),
        });
        drop(g);
        tracing::info!(to = %target.label, trigger = %trigger.label(), "已切换凭据");
        if !matches!(trigger, Trigger::Manual) {
            self.notify(
                "已切换 Claude 账号",
                &format!("{} → {}（{}）", shown.0, shown.1, trigger.label()),
            );
        }
        Ok(())
    }

    /// 把 CLI 当前登录的账号纳入管理。
    pub async fn import_current(&self) -> AppResult<String> {
        let _op = self.op.lock().await;
        let oauth_value = self
            .paths
            .read_oauth()?
            .ok_or_else(|| AppError::NotFound("CLI 当前未登录 Claude 订阅账号".into()))?;
        let account = self.paths.read_account()?;
        let id = {
            let mut g = self.lock();
            let id = g.vault.upsert(oauth_value, account)?;
            g.vault.save()?;
            id
        };
        self.sync_from_cli()?;
        self.emit();
        self.wake.notify_one();
        Ok(id)
    }

    /// 生成新的授权链接（覆盖之前未完成的登录）。
    pub fn login_start(&self) -> String {
        let session = login::new_session();
        let url = session.url.clone();
        self.lock().login = Some(session);
        self.emit();
        url
    }

    pub fn login_cancel(&self) {
        self.lock().login = None;
        self.emit();
    }

    /// 用粘贴的 `code#state` 完成登录并加入账号库，返回账号显示名。
    pub async fn login_complete(&self, pasted: &str) -> AppResult<String> {
        let session = self
            .lock()
            .login
            .clone()
            .ok_or_else(|| AppError::Invalid("没有进行中的登录，请先生成登录链接".into()))?;
        let (oauth_value, account) = login::complete(&self.client(), &session, pasted).await?;

        let _op = self.op.lock().await;
        let label = {
            let mut g = self.lock();
            let id = g.vault.upsert(oauth_value, Some(account))?;
            g.vault.save()?;
            g.login = None;
            g.label_of(&id)
        };
        tracing::info!(account = %label, "已通过界面登录添加账号");
        self.emit();
        self.wake.notify_one();
        Ok(label)
    }

    pub async fn remove(&self, id: &str) -> AppResult<()> {
        let _op = self.op.lock().await;
        {
            let mut g = self.lock();
            g.vault.remove(id)?;
            g.vault.save()?;
            g.statuses.remove(id);
            if g.active_id.as_deref() == Some(id) {
                g.active_id = None;
            }
            if g.pending
                .as_ref()
                .is_some_and(|p| p.from_id == id || p.to_id.as_deref() == Some(id))
            {
                g.pending = None;
            }
        }
        self.emit();
        Ok(())
    }

    /// 手动设置订阅到期日（毫秒）；None 恢复为按月推算。
    pub fn set_subscription_expiry(&self, id: &str, at: Option<i64>) -> AppResult<()> {
        {
            let mut g = self.lock();
            g.vault.get_mut(id)?.subscription_expires_override = at;
            g.vault.save()?;
        }
        self.emit();
        Ok(())
    }

    pub fn rename(&self, id: &str, label: &str) -> AppResult<()> {
        let label = label.trim();
        if label.is_empty() || label.chars().count() > 40 {
            return Err(AppError::Invalid("名称须为 1~40 个字符".into()));
        }
        {
            let mut g = self.lock();
            g.vault.get_mut(id)?.label = label.to_string();
            g.vault.save()?;
        }
        self.emit();
        Ok(())
    }

    pub fn update_settings(&self, s: Settings) -> AppResult<()> {
        s.validate()?;
        // 先按新设置构建客户端，代理不可用时不落盘
        let net = build_net(&s, &self.paths)?;
        self.lock().settings.set(s)?;
        *self.net.lock().unwrap_or_else(|e| e.into_inner()) = net;
        self.emit();
        self.wake.notify_one();
        Ok(())
    }

    pub fn poll_interval(&self) -> Duration {
        Duration::from_secs(self.lock().settings.value.poll_secs)
    }
}
