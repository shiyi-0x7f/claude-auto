//! Anthropic OAuth 相关接口。
//!
//! ⚠️ 均为 Claude Code CLI 内部使用的非公开接口（逆向自 CLI 2.1.304，2026-10-20 实测可用），
//! 可能随 CLI 升级变化。详见 dev_docs/architecture.md「外部接口」。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cli_files::oauth;
use crate::error::{AppError, AppResult};
use crate::fsutil::now_ms;
use crate::proxy::ProxyConf;

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
const PROFILE_URL: &str = "https://api.anthropic.com/api/oauth/profile";
const TOKEN_URL: &str = "https://platform.claude.com/v1/oauth/token";
pub const AUTHORIZE_URL: &str = "https://claude.com/cai/oauth/authorize";
/// 手动回调页：授权后页面显示 `code#state` 供用户复制
pub const MANUAL_REDIRECT_URL: &str = "https://platform.claude.com/oauth/code/callback";
pub const CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
/// 与 CLI 订阅登录申请的作用域一致
pub const LOGIN_SCOPES: &[&str] = &[
    "org:create_api_key",
    "user:profile",
    "user:inference",
    "user:sessions:claude_code",
    "user:mcp_servers",
    "user:file_upload",
    "user:plugins",
];
const OAUTH_BETA: &str = "oauth-2025-04-20";

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageWindow {
    /// 已用百分比 0~100
    pub utilization: f64,
    /// 窗口重置时间（毫秒时间戳）
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
}

#[derive(Deserialize)]
struct RawWindow {
    utilization: Option<f64>,
    resets_at: Option<String>,
}

#[derive(Deserialize)]
struct RawUsage {
    five_hour: Option<RawWindow>,
    seven_day: Option<RawWindow>,
}

fn parse_window(raw: Option<RawWindow>) -> Option<UsageWindow> {
    let raw = raw?;
    Some(UsageWindow {
        utilization: raw.utilization?,
        resets_at: raw
            .resets_at
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
            .map(|d| d.timestamp_millis()),
    })
}

pub fn parse_usage(body: &str) -> AppResult<Usage> {
    let raw: RawUsage = serde_json::from_str(body)?;
    Ok(Usage {
        five_hour: parse_window(raw.five_hour),
        seven_day: parse_window(raw.seven_day),
    })
}

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
}

impl Client {
    /// `proxy` 为 None 时强制直连（不读进程环境变量，代理来源统一由 proxy::resolve 决定）。
    pub fn new(proxy: Option<&ProxyConf>) -> AppResult<Self> {
        let mut builder = reqwest::Client::builder()
            .user_agent(concat!("claude-auto/", env!("CARGO_PKG_VERSION")))
            .timeout(std::time::Duration::from_secs(30))
            .no_proxy();
        if let Some(p) = proxy {
            let mut px = reqwest::Proxy::all(&p.url)
                .map_err(|e| AppError::Invalid(format!("代理地址无效: {e}")))?;
            if let Some(np) = &p.no_proxy {
                px = px.no_proxy(reqwest::NoProxy::from_string(np));
            }
            builder = builder.proxy(px);
        }
        Ok(Self {
            http: builder.build()?,
        })
    }

    /// 用授权码换取令牌（手动回调流程），返回原始令牌响应。
    pub async fn exchange_code(&self, code: &str, state: &str, verifier: &str) -> AppResult<Value> {
        let body = serde_json::json!({
            "grant_type": "authorization_code",
            "code": code,
            "redirect_uri": MANUAL_REDIRECT_URL,
            "client_id": CLIENT_ID,
            "code_verifier": verifier,
            "state": state,
        });
        let resp = self.http.post(TOKEN_URL).json(&body).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(AppError::Auth(format!(
                "授权码换取令牌失败 {status}: {}",
                truncate(&text)
            )));
        }
        Ok(serde_json::from_str(&text)?)
    }

    pub async fn fetch_profile(&self, access_token: &str) -> AppResult<Value> {
        let resp = self
            .http
            .get(PROFILE_URL)
            .bearer_auth(access_token)
            .header("anthropic-beta", OAUTH_BETA)
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(AppError::Http(format!(
                "获取账号信息失败 {status}: {}",
                truncate(&text)
            )));
        }
        Ok(serde_json::from_str(&text)?)
    }

    pub async fn fetch_usage(&self, access_token: &str) -> AppResult<Usage> {
        let resp = self
            .http
            .get(USAGE_URL)
            .bearer_auth(access_token)
            .header("anthropic-beta", OAUTH_BETA)
            .send()
            .await?;
        let status = resp.status();
        let body = resp.text().await?;
        if status == 401 || status == 403 {
            return Err(AppError::Auth(format!("额度接口返回 {status}")));
        }
        if !status.is_success() {
            return Err(AppError::Http(format!(
                "额度接口返回 {status}: {}",
                truncate(&body)
            )));
        }
        parse_usage(&body)
    }

    /// 刷新令牌，返回更新后的 `claudeAiOauth`（保留原有其他字段）。
    pub async fn refresh(&self, current: &Value) -> AppResult<Value> {
        let refresh_token = oauth::refresh_token(current)
            .ok_or_else(|| AppError::Auth("缺少 refreshToken".into()))?;
        let body = serde_json::json!({
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
            "client_id": CLIENT_ID,
            "scope": oauth::scopes(current).join(" "),
        });
        let resp = self.http.post(TOKEN_URL).json(&body).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if status == 400 || status == 401 || status == 403 {
            return Err(AppError::Auth(format!(
                "刷新令牌被拒 {status}: {}",
                truncate(&text)
            )));
        }
        if !status.is_success() {
            return Err(AppError::Http(format!(
                "刷新令牌返回 {status}: {}",
                truncate(&text)
            )));
        }
        apply_refresh(current, &serde_json::from_str(&text)?, now_ms())
    }
}

/// 把标准 OAuth 刷新响应合并进原 `claudeAiOauth`。
pub fn apply_refresh(current: &Value, resp: &Value, now: i64) -> AppResult<Value> {
    let access = resp
        .get("access_token")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::Http("刷新响应缺少 access_token".into()))?;
    let mut next = current.clone();
    let obj = next
        .as_object_mut()
        .ok_or_else(|| AppError::Invalid("claudeAiOauth 不是对象".into()))?;
    obj.insert("accessToken".into(), access.into());
    if let Some(rt) = resp.get("refresh_token").and_then(Value::as_str) {
        obj.insert("refreshToken".into(), rt.into());
    }
    if let Some(secs) = resp.get("expires_in").and_then(Value::as_i64) {
        obj.insert("expiresAt".into(), (now + secs * 1000).into());
    }
    if let Some(scope) = resp.get("scope").and_then(Value::as_str) {
        let scopes: Vec<Value> = scope.split_whitespace().map(Value::from).collect();
        obj.insert("scopes".into(), scopes.into());
    }
    Ok(next)
}

fn str_at<'a>(v: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter().try_fold(v, |acc, k| acc.get(k))?.as_str()
}

/// organization_type → CLI 的 subscriptionType（claude_pro → pro）。
fn subscription_type(profile: &Value) -> Option<String> {
    str_at(profile, &["organization", "organization_type"])
        .map(|t| t.strip_prefix("claude_").unwrap_or(t).to_string())
}

/// 令牌响应 + profile → CLI `.credentials.json` 的 `claudeAiOauth`。
pub fn build_oauth(token_resp: &Value, profile: &Value, now: i64) -> AppResult<Value> {
    let skeleton = serde_json::json!({
        "accessToken": "",
        "refreshToken": Value::Null,
        "expiresAt": Value::Null,
        "scopes": LOGIN_SCOPES,
        "subscriptionType": subscription_type(profile),
        "rateLimitTier": str_at(profile, &["organization", "rate_limit_tier"]),
    });
    let oauth_value = apply_refresh(&skeleton, token_resp, now)?;
    if oauth::refresh_token(&oauth_value).is_none() {
        return Err(AppError::Auth("令牌响应缺少 refresh_token".into()));
    }
    Ok(oauth_value)
}

/// profile → CLI `.claude.json` 的 `oauthAccount`（字段名与 CLI 写入的一致）。
pub fn build_account(profile: &Value) -> Value {
    let acc = |k: &str| profile.get("account").and_then(|a| a.get(k)).cloned();
    let org = |k: &str| profile.get("organization").and_then(|o| o.get(k)).cloned();
    let mut m = serde_json::Map::new();
    let mut put = |key: &str, v: Option<Value>| {
        if let Some(v) = v {
            m.insert(key.into(), v);
        }
    };
    put("accountUuid", acc("uuid"));
    put("emailAddress", acc("email"));
    put("organizationUuid", org("uuid"));
    put("hasExtraUsageEnabled", org("has_extra_usage_enabled"));
    put("billingType", org("billing_type"));
    put("accountCreatedAt", acc("created_at"));
    put("subscriptionCreatedAt", org("subscription_created_at"));
    put("displayName", acc("display_name"));
    put("fullName", acc("full_name"));
    put("organizationName", org("name"));
    put("organizationType", org("organization_type"));
    put("organizationRateLimitTier", org("rate_limit_tier"));
    Value::Object(m)
}

/// 错误响应体可能很长，截断后写进错误消息；响应体不含令牌。
fn truncate(s: &str) -> String {
    s.chars().take(300).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn profile() -> Value {
        // 2026-10-20 实测 /api/oauth/profile 结构（标识已替换）
        json!({
            "account": {"uuid": "acc-1", "email": "a@x.com", "display_name": "A", "full_name": "A B",
                        "created_at": "2026-10-20T11:55:36Z"},
            "organization": {"uuid": "org-1", "name": "A's Org", "organization_type": "claude_pro",
                             "billing_type": "apple_subscription", "rate_limit_tier": "default_claude_ai",
                             "has_extra_usage_enabled": false}
        })
    }

    #[test]
    fn builds_cli_shapes_from_login() {
        let token = json!({"access_token": "at", "refresh_token": "rt", "expires_in": 28800,
                           "scope": "user:inference user:profile"});
        let o = build_oauth(&token, &profile(), 1000).unwrap();
        assert_eq!(oauth::access_token(&o), Some("at"));
        assert_eq!(oauth::refresh_token(&o), Some("rt"));
        assert_eq!(oauth::expires_at(&o), Some(1000 + 28_800_000));
        assert_eq!(oauth::subscription_type(&o), Some("pro"));
        assert_eq!(o["rateLimitTier"], "default_claude_ai");

        let a = build_account(&profile());
        assert_eq!(crate::cli_files::account::uuid(&a), Some("acc-1"));
        assert_eq!(crate::cli_files::account::email(&a), Some("a@x.com"));
        assert_eq!(a["organizationType"], "claude_pro");
    }

    #[test]
    fn build_oauth_requires_refresh_token() {
        assert!(build_oauth(&json!({"access_token": "at"}), &profile(), 0).is_err());
    }

    #[test]
    fn parses_real_usage_shape() {
        // 2026-10-20 实测响应（节选）
        let body = r#"{"five_hour":{"utilization":12.0,"resets_at":"2026-10-20T19:20:00.432190+00:00","limit_dollars":null},
            "seven_day":{"utilization":2.0,"resets_at":"2026-10-21T13:00:00.432208+00:00"},
            "seven_day_opus":null,"extra_usage":{"is_enabled":false}}"#;
        let u = parse_usage(body).unwrap();
        let fh = u.five_hour.unwrap();
        assert_eq!(fh.utilization, 12.0);
        assert_eq!(
            fh.resets_at,
            Some(
                chrono::DateTime::parse_from_rfc3339("2026-10-20T19:20:00.432190+00:00")
                    .unwrap()
                    .timestamp_millis()
            )
        );
        assert_eq!(u.seven_day.unwrap().utilization, 2.0);
    }

    #[test]
    fn null_windows_become_none() {
        let u = parse_usage(r#"{"five_hour":null,"seven_day":{"utilization":null}}"#).unwrap();
        assert_eq!(u, Usage::default());
    }

    #[test]
    fn apply_refresh_merges_and_keeps_extra_fields() {
        let cur = json!({"accessToken":"a","refreshToken":"r","expiresAt":1,"subscriptionType":"max","scopes":["x"]});
        let resp = json!({"access_token":"a2","refresh_token":"r2","expires_in":3600,"scope":"user:inference user:profile"});
        let next = apply_refresh(&cur, &resp, 1000).unwrap();
        assert_eq!(next["accessToken"], "a2");
        assert_eq!(next["refreshToken"], "r2");
        assert_eq!(next["expiresAt"], 1000 + 3_600_000);
        assert_eq!(next["subscriptionType"], "max");
        assert_eq!(next["scopes"], json!(["user:inference", "user:profile"]));
    }

    #[test]
    fn apply_refresh_without_rotation_keeps_old_refresh_token() {
        let cur = json!({"accessToken":"a","refreshToken":"r"});
        let next = apply_refresh(&cur, &json!({"access_token":"a2"}), 0).unwrap();
        assert_eq!(next["refreshToken"], "r");
    }
}
