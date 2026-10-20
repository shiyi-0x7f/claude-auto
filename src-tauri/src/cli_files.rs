//! Claude Code CLI 的本地凭据文件读写。
//!
//! - `<config>/.credentials.json` 的 `claudeAiOauth`：OAuth 令牌（同文件其他键如 `mcpOAuth` 原样保留）
//! - `.claude.json` 的 `oauthAccount`：账号元信息（邮箱/组织），CLI 用来展示当前账号
//!
//! 配置目录：设置了 `CLAUDE_CONFIG_DIR` 时两者都在该目录下；否则凭据在 `~/.claude/`，
//! `.claude.json` 在用户主目录。

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::error::AppResult;
use crate::fsutil::{read_json_object, write_json_atomic};

const OAUTH_KEY: &str = "claudeAiOauth";
const ACCOUNT_KEY: &str = "oauthAccount";

#[derive(Debug, Clone)]
pub struct CliPaths {
    pub config_dir: PathBuf,
    pub credentials: PathBuf,
    pub global_config: PathBuf,
}

impl CliPaths {
    pub fn resolve(home: &Path) -> Self {
        match std::env::var_os("CLAUDE_CONFIG_DIR").filter(|v| !v.is_empty()) {
            Some(dir) => Self::for_config_dir(PathBuf::from(dir)),
            None => {
                let config_dir = home.join(".claude");
                Self {
                    credentials: config_dir.join(".credentials.json"),
                    global_config: home.join(".claude.json"),
                    config_dir,
                }
            }
        }
    }

    /// 显式配置目录（`CLAUDE_CONFIG_DIR` 语义），也用于隔离登录的临时目录。
    pub fn for_config_dir(dir: PathBuf) -> Self {
        Self {
            credentials: dir.join(".credentials.json"),
            global_config: dir.join(".claude.json"),
            config_dir: dir,
        }
    }

    pub fn read_oauth(&self) -> AppResult<Option<Value>> {
        let map = read_json_object(&self.credentials)?;
        Ok(map.get(OAUTH_KEY).filter(|v| v.is_object()).cloned())
    }

    pub fn write_oauth(&self, oauth: &Value) -> AppResult<()> {
        let mut map = read_json_object(&self.credentials)?;
        map.insert(OAUTH_KEY.to_string(), oauth.clone());
        write_json_atomic(&self.credentials, &Value::Object(map))
    }

    pub fn read_account(&self) -> AppResult<Option<Value>> {
        let map = read_json_object(&self.global_config)?;
        Ok(map.get(ACCOUNT_KEY).filter(|v| v.is_object()).cloned())
    }

    /// 只替换 `oauthAccount` 键，其余配置原样保留；`None` 时删除该键。
    pub fn write_account(&self, account: Option<&Value>) -> AppResult<()> {
        let mut map = read_json_object(&self.global_config)?;
        match account {
            Some(a) => {
                map.insert(ACCOUNT_KEY.to_string(), a.clone());
            }
            None => {
                map.shift_remove(ACCOUNT_KEY);
            }
        }
        write_json_atomic(&self.global_config, &Value::Object(map))
    }
}

/// `claudeAiOauth` 字段访问器（保持原始 JSON，避免 CLI 新增字段被丢弃）。
pub mod oauth {
    use serde_json::Value;

    pub fn access_token(v: &Value) -> Option<&str> {
        v.get("accessToken").and_then(Value::as_str)
    }
    pub fn refresh_token(v: &Value) -> Option<&str> {
        v.get("refreshToken").and_then(Value::as_str)
    }
    pub fn expires_at(v: &Value) -> Option<i64> {
        v.get("expiresAt").and_then(Value::as_i64)
    }
    pub fn refresh_expires_at(v: &Value) -> Option<i64> {
        v.get("refreshTokenExpiresAt").and_then(Value::as_i64)
    }
    pub fn subscription_type(v: &Value) -> Option<&str> {
        v.get("subscriptionType").and_then(Value::as_str)
    }
    pub fn scopes(v: &Value) -> Vec<String> {
        v.get("scopes")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|s| s.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// `oauthAccount` 字段访问器。
pub mod account {
    use serde_json::Value;

    pub fn uuid(v: &Value) -> Option<&str> {
        v.get("accountUuid").and_then(Value::as_str)
    }
    pub fn email(v: &Value) -> Option<&str> {
        v.get("emailAddress").and_then(Value::as_str)
    }
    pub fn subscription_created_at(v: &Value) -> Option<&str> {
        v.get("subscriptionCreatedAt").and_then(Value::as_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn write_oauth_keeps_other_keys() {
        let dir = tempfile::tempdir().unwrap();
        let p = CliPaths::for_config_dir(dir.path().to_path_buf());
        std::fs::write(
            &p.credentials,
            r#"{"mcpOAuth":{"x":1},"claudeAiOauth":{"accessToken":"old"}}"#,
        )
        .unwrap();

        p.write_oauth(&json!({"accessToken": "new", "refreshToken": "r"}))
            .unwrap();

        let map = read_json_object(&p.credentials).unwrap();
        assert_eq!(map["mcpOAuth"], json!({"x": 1}));
        assert_eq!(oauth::access_token(&map["claudeAiOauth"]), Some("new"));
        // 键序保持：mcpOAuth 仍在前
        assert_eq!(map.keys().next().map(String::as_str), Some("mcpOAuth"));
    }

    #[test]
    fn write_account_patches_only_its_key() {
        let dir = tempfile::tempdir().unwrap();
        let p = CliPaths::for_config_dir(dir.path().to_path_buf());
        std::fs::write(
            &p.global_config,
            r#"{"numStartups":3,"oauthAccount":{"emailAddress":"a@x"}}"#,
        )
        .unwrap();

        p.write_account(Some(&json!({"emailAddress": "b@x"})))
            .unwrap();
        let acc = p.read_account().unwrap().unwrap();
        assert_eq!(account::email(&acc), Some("b@x"));
        assert_eq!(
            read_json_object(&p.global_config).unwrap()["numStartups"],
            json!(3)
        );

        p.write_account(None).unwrap();
        assert!(p.read_account().unwrap().is_none());
    }

    #[test]
    fn missing_files_read_as_none() {
        let dir = tempfile::tempdir().unwrap();
        let p = CliPaths::for_config_dir(dir.path().join("nope"));
        assert!(p.read_oauth().unwrap().is_none());
        assert!(p.read_account().unwrap().is_none());
    }
}
