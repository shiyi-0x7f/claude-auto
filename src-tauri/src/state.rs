//! 运行时状态与给前端的快照 DTO。

use std::collections::{HashMap, VecDeque};

use serde::Serialize;

use crate::anthropic::Usage;
use crate::cli_files::oauth;
use crate::login::LoginSession;
use crate::policy::Exhaustion;
use crate::settings::{Settings, SettingsStore};
use crate::vault::Vault;

const HISTORY_LIMIT: usize = 50;

#[derive(Debug, Clone, Default)]
pub struct AccountStatus {
    pub usage: Option<Usage>,
    pub error: Option<String>,
    pub needs_relogin: bool,
    pub checked_at: Option<i64>,
}

/// 激活账号达到阈值后的切换建议（半自动模式等待用户确认）。
#[derive(Debug, Clone, PartialEq)]
pub struct Suggestion {
    pub from_id: String,
    /// None 表示所有账号均已耗尽
    pub to_id: Option<String>,
    pub reason: Exhaustion,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchRecord {
    pub at: i64,
    pub from_label: Option<String>,
    pub to_label: String,
    pub trigger: String,
}

pub struct Inner {
    pub vault: Vault,
    pub settings: SettingsStore,
    pub statuses: HashMap<String, AccountStatus>,
    /// 当前 CLI 登录的账号在库中的 id；CLI 未登录或登录的是未纳管账号时为 None
    pub active_id: Option<String>,
    pub cli_logged_in: bool,
    pub pending: Option<Suggestion>,
    /// 最近一次已发通知的建议，避免每轮轮询重复弹通知
    pub notified: Option<Suggestion>,
    pub history: VecDeque<SwitchRecord>,
    pub login: Option<LoginSession>,
    pub last_error: Option<String>,
    pub last_poll_at: Option<i64>,
}

impl Inner {
    pub fn new(vault: Vault, settings: SettingsStore) -> Self {
        Self {
            vault,
            settings,
            statuses: HashMap::new(),
            active_id: None,
            cli_logged_in: false,
            pending: None,
            notified: None,
            history: VecDeque::new(),
            login: None,
            last_error: None,
            last_poll_at: None,
        }
    }

    pub fn label_of(&self, id: &str) -> String {
        self.vault
            .get(id)
            .map(|a| a.label.clone())
            .unwrap_or_else(|_| "已删除账号".into())
    }

    /// 系统级文本（通知/托盘）用的账号名：隐私模式下打码。
    pub fn display(&self, label: &str) -> String {
        if self.settings.value.privacy_mode {
            crate::privacy::mask(label)
        } else {
            label.to_string()
        }
    }

    pub fn push_history(&mut self, rec: SwitchRecord) {
        self.history.push_front(rec);
        self.history.truncate(HISTORY_LIMIT);
    }

    pub fn snapshot(&self, cli_config_dir: String, proxy: Option<String>) -> Snapshot {
        let accounts = self
            .vault
            .data
            .accounts
            .iter()
            .map(|a| {
                let st = self.statuses.get(&a.id).cloned().unwrap_or_default();
                AccountView {
                    id: a.id.clone(),
                    label: a.label.clone(),
                    email: a.email().map(String::from),
                    plan: oauth::subscription_type(&a.oauth).map(String::from),
                    is_active: self.active_id.as_deref() == Some(a.id.as_str()),
                    token_expires_at: oauth::expires_at(&a.oauth),
                    usage: st.usage,
                    error: st.error,
                    needs_relogin: st.needs_relogin,
                    checked_at: st.checked_at,
                }
            })
            .collect();
        Snapshot {
            accounts,
            active_id: self.active_id.clone(),
            cli_logged_in: self.cli_logged_in,
            cli_config_dir,
            settings: self.settings.value.clone(),
            pending: self.pending.as_ref().map(|p| PendingView {
                from_id: p.from_id.clone(),
                from_label: self.label_of(&p.from_id),
                to_id: p.to_id.clone(),
                to_label: p.to_id.as_deref().map(|id| self.label_of(id)),
                reason: p.reason,
                reason_label: p.reason.label().into(),
            }),
            history: self.history.iter().cloned().collect(),
            login_url: self.login.as_ref().map(|s| s.url.clone()),
            proxy,
            last_error: self.last_error.clone(),
            last_poll_at: self.last_poll_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub id: String,
    pub label: String,
    pub email: Option<String>,
    pub plan: Option<String>,
    pub is_active: bool,
    pub token_expires_at: Option<i64>,
    pub usage: Option<Usage>,
    pub error: Option<String>,
    pub needs_relogin: bool,
    pub checked_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingView {
    pub from_id: String,
    pub from_label: String,
    pub to_id: Option<String>,
    pub to_label: Option<String>,
    pub reason: Exhaustion,
    pub reason_label: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub accounts: Vec<AccountView>,
    pub active_id: Option<String>,
    pub cli_logged_in: bool,
    pub cli_config_dir: String,
    pub settings: Settings,
    pub pending: Option<PendingView>,
    pub history: Vec<SwitchRecord>,
    /// 进行中的登录授权链接
    pub login_url: Option<String>,
    /// 当前生效的代理（已隐去凭据），None 为直连
    pub proxy: Option<String>,
    pub last_error: Option<String>,
    pub last_poll_at: Option<i64>,
}
