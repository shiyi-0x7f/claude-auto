//! 到期信息：登录有效期（refresh token）与订阅续费日。
//!
//! 订阅到期日没有官方接口（CLI 与 profile 均不提供续费周期，苹果内购由 App Store 管理），
//! 因此采用「手动设置优先，否则按订阅开始日按月推算」。

use chrono::{DateTime, Months, TimeZone, Utc};
use serde::Serialize;

use crate::cli_files::{account, oauth};
use crate::vault::Account;

/// 登录有效期剩余天数低于该值时提醒
pub const RELOGIN_WARN_DAYS: i64 = 7;
/// 订阅剩余天数低于该值时提醒
pub const RENEWAL_WARN_DAYS: i64 = 3;
const DAY_MS: i64 = 86_400_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RenewalSource {
    /// 用户手动设置
    Manual,
    /// 按订阅开始日按月推算
    Estimated,
}

/// 订阅开始日之后、晚于 now 的第一个月度周年日。
/// 每次都从开始日加 n 个月（而非逐月累加），避免 31 号经过 2 月后日期漂移。
pub fn next_monthly_renewal(start: DateTime<Utc>, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    (1..=1200u32)
        .map_while(|n| start.checked_add_months(Months::new(n)))
        .find(|t| *t > now)
}

/// 订阅续费/到期日（毫秒）与来源。
pub fn renewal_of(acc: &Account, now_ms: i64) -> Option<(i64, RenewalSource)> {
    if let Some(manual) = acc.subscription_expires_override {
        return Some((manual, RenewalSource::Manual));
    }
    let start = acc
        .oauth_account
        .as_ref()
        .and_then(account::subscription_created_at)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())?
        .with_timezone(&Utc);
    let now = Utc.timestamp_millis_opt(now_ms).single()?;
    next_monthly_renewal(start, now).map(|t| (t.timestamp_millis(), RenewalSource::Estimated))
}

pub fn refresh_expires_at(acc: &Account) -> Option<i64> {
    oauth::refresh_expires_at(&acc.oauth)
}

/// 剩余整天数（向下取整，已过期为负）。
pub fn days_left(at: i64, now_ms: i64) -> i64 {
    (at - now_ms).div_euclid(DAY_MS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn utc(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn account(created: Option<&str>, manual: Option<i64>) -> Account {
        Account {
            id: "a".into(),
            label: "a".into(),
            oauth: json!({"accessToken": "x", "refreshToken": "y", "refreshTokenExpiresAt": 42}),
            oauth_account: created.map(|c| json!({"subscriptionCreatedAt": c})),
            added_at: 0,
            subscription_expires_override: manual,
        }
    }

    #[test]
    fn monthly_renewal_after_now() {
        let start = utc("2026-10-20T14:23:31Z");
        assert_eq!(
            next_monthly_renewal(start, utc("2026-10-21T00:00:00Z")),
            Some(utc("2026-11-20T14:23:31Z"))
        );
        // 恰好在周年日之后
        assert_eq!(
            next_monthly_renewal(start, utc("2026-11-20T15:00:00Z")),
            Some(utc("2026-12-20T14:23:31Z"))
        );
    }

    #[test]
    fn month_end_does_not_drift() {
        let start = utc("2026-01-31T10:00:00Z");
        assert_eq!(
            next_monthly_renewal(start, utc("2026-02-15T00:00:00Z")),
            Some(utc("2026-02-28T10:00:00Z"))
        );
        // 3 月回到 31 号，而不是停在 28 号
        assert_eq!(
            next_monthly_renewal(start, utc("2026-03-01T00:00:00Z")),
            Some(utc("2026-03-31T10:00:00Z"))
        );
    }

    #[test]
    fn manual_override_wins() {
        let now = utc("2026-10-21T00:00:00Z").timestamp_millis();
        let acc = account(Some("2026-10-20T14:23:31Z"), Some(123));
        assert_eq!(renewal_of(&acc, now), Some((123, RenewalSource::Manual)));
        let acc = account(Some("2026-10-20T14:23:31Z"), None);
        assert_eq!(
            renewal_of(&acc, now),
            Some((
                utc("2026-11-20T14:23:31Z").timestamp_millis(),
                RenewalSource::Estimated
            ))
        );
        assert_eq!(renewal_of(&account(None, None), now), None);
    }

    #[test]
    fn days_left_floors() {
        assert_eq!(days_left(DAY_MS * 3 + 5, 0), 3);
        assert_eq!(days_left(DAY_MS - 1, 0), 0);
        assert_eq!(days_left(0, 1), -1);
        assert_eq!(refresh_expires_at(&account(None, None)), Some(42));
    }
}
