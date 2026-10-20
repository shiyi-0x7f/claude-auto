//! 到期提醒：登录有效期临近过期、订阅临近续费/到期时发系统通知。
//! 每个（账号, 到期时间）只提醒一次；到期时间变化（如刷新后顺延、手动改期）会重新判断。

use chrono::{Local, TimeZone};

use crate::expiry::{self, RenewalSource, RELOGIN_WARN_DAYS, RENEWAL_WARN_DAYS};
use crate::fsutil::now_ms;
use crate::service::Service;
use crate::state::Inner;

fn fmt_date(ms: i64) -> String {
    Local
        .timestamp_millis_opt(ms)
        .single()
        .map_or_else(|| "--".into(), |d| d.format("%m-%d").to_string())
}

/// 需要提醒的一条消息
#[derive(Debug, PartialEq)]
pub struct Alert {
    pub key: String,
    pub title: String,
    pub body: String,
}

/// 收集当前应提醒的到期事项（纯函数：只读状态，不发通知）。
pub fn collect(g: &Inner, now: i64) -> Vec<Alert> {
    let mut out = Vec::new();
    for a in &g.vault.data.accounts {
        let name = g.display(&a.label);
        if let Some(at) = expiry::refresh_expires_at(a) {
            let d = expiry::days_left(at, now);
            if d < RELOGIN_WARN_DAYS {
                out.push(Alert {
                    key: format!("relogin:{}:{at}", a.id),
                    title: if d < 0 {
                        "账号登录已过期".into()
                    } else {
                        "账号登录即将过期".into()
                    },
                    body: if d < 0 {
                        format!("{name} 需要重新登录")
                    } else {
                        format!(
                            "{name} 的登录将于 {}（{d} 天后）过期，请重新登录",
                            fmt_date(at)
                        )
                    },
                });
            }
        }
        if let Some((at, source)) = expiry::renewal_of(a, now) {
            let d = expiry::days_left(at, now);
            if (0..RENEWAL_WARN_DAYS).contains(&d) {
                let what = match source {
                    RenewalSource::Manual => "订阅到期",
                    RenewalSource::Estimated => "订阅预计续费",
                };
                out.push(Alert {
                    key: format!("renew:{}:{at}", a.id),
                    title: format!("{what}提醒"),
                    body: format!("{name} 将于 {}（{d} 天后）{what}", fmt_date(at)),
                });
            }
        }
    }
    out
}

/// 轮询后调用：发出尚未发过的提醒。
pub fn check(svc: &Service) {
    let alerts = collect(&svc.lock(), now_ms());
    for alert in alerts {
        let fresh = svc.lock().expiry_alerted.insert(alert.key.clone());
        if fresh {
            svc.notify(&alert.title, &alert.body);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SettingsStore;
    use crate::vault::Vault;
    use serde_json::json;

    const DAY: i64 = 86_400_000;

    fn inner_with(refresh_at: i64, manual_renewal: Option<i64>) -> (tempfile::TempDir, Inner) {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = Vault::load(dir.path()).unwrap();
        let id = vault
            .upsert(
                json!({"accessToken": "a", "refreshToken": "r", "refreshTokenExpiresAt": refresh_at}),
                Some(json!({"accountUuid": "u", "emailAddress": "alice@x.com"})),
            )
            .unwrap();
        vault.get_mut(&id).unwrap().subscription_expires_override = manual_renewal;
        let inner = Inner::new(vault, SettingsStore::load(dir.path()).unwrap());
        (dir, inner)
    }

    #[test]
    fn warns_when_login_or_subscription_near_expiry() {
        let (_d, g) = inner_with(5 * DAY, Some(2 * DAY + 1));
        let alerts = collect(&g, 0);
        assert_eq!(alerts.len(), 2);
        assert!(alerts[0].key.starts_with("relogin:"));
        assert_eq!(alerts[0].title, "账号登录即将过期");
        assert_eq!(alerts[1].title, "订阅到期提醒");
        // 隐私模式默认开启：通知里只有前两个字
        assert!(alerts.iter().all(|a| !a.body.contains("alice@x.com")));
        assert!(alerts[0].body.starts_with("al***"));
    }

    #[test]
    fn quiet_when_far_from_expiry() {
        let (_d, g) = inner_with(20 * DAY, Some(10 * DAY));
        assert!(collect(&g, 0).is_empty());
    }

    #[test]
    fn expired_login_says_relogin() {
        let (_d, g) = inner_with(-1, None);
        let alerts = collect(&g, 0);
        assert_eq!(alerts[0].title, "账号登录已过期");
    }
}
