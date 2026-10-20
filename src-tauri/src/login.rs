//! 界面内登录（OAuth PKCE + 手动回调）：
//! 1. `start` 生成 verifier/state，返回授权链接；
//! 2. 用户在浏览器授权后，回调页显示 `code#state`，粘贴回界面；
//! 3. `complete` 校验 state、换取令牌、拉取 profile，写入账号库。
//!
//! 不经过 CLI，不影响当前 CLI 登录的账号；请求走统一代理。

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use sha2::{Digest, Sha256};

use crate::anthropic::{self, AUTHORIZE_URL, CLIENT_ID, LOGIN_SCOPES, MANUAL_REDIRECT_URL};
use crate::error::{AppError, AppResult};
use crate::fsutil::now_ms;

/// 授权链接有效期；超时后需重新生成
const SESSION_TTL_MS: i64 = 15 * 60 * 1000;

#[derive(Clone)]
pub struct LoginSession {
    pub verifier: String,
    pub state: String,
    pub url: String,
    pub started_at: i64,
}

fn random_b64(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    rand::thread_rng().fill_bytes(&mut buf);
    URL_SAFE_NO_PAD.encode(buf)
}

fn challenge_of(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn authorize_url(challenge: &str, state: &str) -> String {
    let scope = LOGIN_SCOPES.join(" ");
    let params = [
        ("code", "true"),
        ("client_id", CLIENT_ID),
        ("response_type", "code"),
        ("redirect_uri", MANUAL_REDIRECT_URL),
        ("scope", scope.as_str()),
        ("code_challenge", challenge),
        ("code_challenge_method", "S256"),
        ("state", state),
    ];
    let mut url = reqwest::Url::parse(AUTHORIZE_URL).expect("AUTHORIZE_URL 是合法常量");
    url.query_pairs_mut().extend_pairs(params);
    url.to_string()
}

pub fn new_session() -> LoginSession {
    let verifier = random_b64(32);
    let state = random_b64(32);
    let url = authorize_url(&challenge_of(&verifier), &state);
    LoginSession {
        verifier,
        state,
        url,
        started_at: now_ms(),
    }
}

/// 解析用户粘贴的 `code#state`，并校验 state 与本次会话一致。
pub fn parse_pasted(input: &str, session: &LoginSession) -> AppResult<String> {
    let input = input.trim();
    let (code, state) = input
        .split_once('#')
        .filter(|(c, s)| !c.is_empty() && !s.is_empty())
        .ok_or_else(|| {
            AppError::Invalid("授权码格式不对：应为回调页显示的完整内容（含 # 号）".into())
        })?;
    if state != session.state {
        return Err(AppError::Invalid(
            "授权码与当前登录链接不匹配，请用本窗口最新生成的链接重新授权".into(),
        ));
    }
    if now_ms() - session.started_at > SESSION_TTL_MS {
        return Err(AppError::Invalid("登录链接已过期，请重新生成".into()));
    }
    Ok(code.to_string())
}

/// 换取令牌并构造 CLI 格式的凭据与账号信息。
pub async fn complete(
    client: &anthropic::Client,
    session: &LoginSession,
    pasted: &str,
) -> AppResult<(serde_json::Value, serde_json::Value)> {
    let code = parse_pasted(pasted, session)?;
    let token_resp = client
        .exchange_code(&code, &session.state, &session.verifier)
        .await?;
    let access = token_resp
        .get("access_token")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| AppError::Http("令牌响应缺少 access_token".into()))?;
    let profile = client.fetch_profile(access).await?;
    let oauth_value = anthropic::build_oauth(&token_resp, &profile, now_ms())?;
    Ok((oauth_value, anthropic::build_account(&profile)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_matches_rfc7636_example() {
        // RFC 7636 附录 B
        assert_eq!(
            challenge_of("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn url_contains_required_params() {
        let s = new_session();
        let url = reqwest::Url::parse(&s.url).unwrap();
        let q: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(q["redirect_uri"], MANUAL_REDIRECT_URL);
        assert_eq!(q["code_challenge"], challenge_of(&s.verifier));
        assert_eq!(q["state"], s.state);
        assert!(q["scope"].contains("user:inference"));
    }

    #[test]
    fn parses_and_validates_pasted_code() {
        let s = new_session();
        assert_eq!(
            parse_pasted(&format!("  abc#{}\n", s.state), &s).unwrap(),
            "abc"
        );
        assert!(parse_pasted("abc", &s).is_err());
        assert!(parse_pasted("abc#wrong", &s).is_err());
        let expired = LoginSession {
            started_at: 0,
            ..new_session()
        };
        assert!(parse_pasted(&format!("abc#{}", expired.state), &expired).is_err());
    }
}
