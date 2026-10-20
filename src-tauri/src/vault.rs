//! 账号库：本工具持有的多账号凭据，存于应用数据目录 `accounts.json`。
//!
//! 令牌所有权规则（避免轮换式 refresh token 互相作废）：
//! - 当前激活账号的令牌归 CLI 所有：本工具只读 CLI 文件并同步回库，绝不自行刷新；
//! - 非激活账号的令牌归本工具所有：过期前由本工具刷新。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cli_files::{account, oauth};
use crate::error::{AppError, AppResult};
use crate::fsutil::{now_ms, write_atomic};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub label: String,
    /// CLI `.credentials.json` 中 `claudeAiOauth` 的原样副本
    pub oauth: Value,
    /// CLI `.claude.json` 中 `oauthAccount` 的原样副本（切换时一并写回）
    pub oauth_account: Option<Value>,
    pub added_at: i64,
}

impl Account {
    pub fn email(&self) -> Option<&str> {
        self.oauth_account.as_ref().and_then(account::email)
    }

    /// 是否与给定 CLI 凭据属于同一账号：优先比 accountUuid，缺失时退回比 refresh token。
    pub fn matches(&self, cli_oauth: &Value, cli_account: Option<&Value>) -> bool {
        let mine = self.oauth_account.as_ref().and_then(account::uuid);
        let theirs = cli_account.and_then(account::uuid);
        if let (Some(a), Some(b)) = (mine, theirs) {
            return a == b;
        }
        match (
            oauth::refresh_token(&self.oauth),
            oauth::refresh_token(cli_oauth),
        ) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultData {
    pub accounts: Vec<Account>,
}

pub struct Vault {
    path: PathBuf,
    pub data: VaultData,
}

impl Vault {
    pub fn load(dir: &Path) -> AppResult<Self> {
        let path = dir.join("accounts.json");
        let data = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => VaultData::default(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self { path, data })
    }

    pub fn save(&self) -> AppResult<()> {
        write_atomic(&self.path, &serde_json::to_vec_pretty(&self.data)?)
    }

    pub fn get(&self, id: &str) -> AppResult<&Account> {
        self.data
            .accounts
            .iter()
            .find(|a| a.id == id)
            .ok_or_else(|| AppError::NotFound(format!("账号 {id}")))
    }

    pub fn get_mut(&mut self, id: &str) -> AppResult<&mut Account> {
        self.data
            .accounts
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| AppError::NotFound(format!("账号 {id}")))
    }

    pub fn find_matching(
        &self,
        cli_oauth: &Value,
        cli_account: Option<&Value>,
    ) -> Option<&Account> {
        self.data
            .accounts
            .iter()
            .find(|a| a.matches(cli_oauth, cli_account))
    }

    /// 新增或更新（同一账号重复导入时覆盖凭据），返回账号 id。
    pub fn upsert(&mut self, oauth: Value, oauth_account: Option<Value>) -> AppResult<String> {
        if oauth::access_token(&oauth).is_none() || oauth::refresh_token(&oauth).is_none() {
            return Err(AppError::Invalid(
                "凭据缺少 accessToken/refreshToken".into(),
            ));
        }
        if let Some(existing) = self
            .data
            .accounts
            .iter_mut()
            .find(|a| a.matches(&oauth, oauth_account.as_ref()))
        {
            existing.oauth = oauth;
            if oauth_account.is_some() {
                existing.oauth_account = oauth_account;
            }
            return Ok(existing.id.clone());
        }
        let label = oauth_account
            .as_ref()
            .and_then(account::email)
            .unwrap_or("未命名账号")
            .to_string();
        let id = uuid::Uuid::new_v4().to_string();
        self.data.accounts.push(Account {
            id: id.clone(),
            label,
            oauth,
            oauth_account,
            added_at: now_ms(),
        });
        Ok(id)
    }

    pub fn remove(&mut self, id: &str) -> AppResult<()> {
        let before = self.data.accounts.len();
        self.data.accounts.retain(|a| a.id != id);
        if self.data.accounts.len() == before {
            return Err(AppError::NotFound(format!("账号 {id}")));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tokens(rt: &str) -> Value {
        json!({"accessToken": "a", "refreshToken": rt, "expiresAt": 1})
    }

    #[test]
    fn upsert_dedups_by_account_uuid() {
        let dir = tempfile::tempdir().unwrap();
        let mut v = Vault::load(dir.path()).unwrap();
        let acc = json!({"accountUuid": "u1", "emailAddress": "a@x"});
        let id1 = v.upsert(tokens("r1"), Some(acc.clone())).unwrap();
        // CLI 刷新后 refresh token 变了，但仍是同一账号
        let id2 = v.upsert(tokens("r2"), Some(acc)).unwrap();
        assert_eq!(id1, id2);
        assert_eq!(v.data.accounts.len(), 1);
        assert_eq!(oauth::refresh_token(&v.data.accounts[0].oauth), Some("r2"));
        assert_eq!(v.data.accounts[0].label, "a@x");
    }

    #[test]
    fn matches_falls_back_to_refresh_token() {
        let dir = tempfile::tempdir().unwrap();
        let mut v = Vault::load(dir.path()).unwrap();
        v.upsert(tokens("r1"), None).unwrap();
        assert!(v.find_matching(&tokens("r1"), None).is_some());
        assert!(v.find_matching(&tokens("other"), None).is_none());
    }

    #[test]
    fn save_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let mut v = Vault::load(dir.path()).unwrap();
        v.upsert(tokens("r1"), None).unwrap();
        v.save().unwrap();
        let v2 = Vault::load(dir.path()).unwrap();
        assert_eq!(v2.data.accounts.len(), 1);
    }

    #[test]
    fn rejects_incomplete_tokens() {
        let dir = tempfile::tempdir().unwrap();
        let mut v = Vault::load(dir.path()).unwrap();
        assert!(v.upsert(json!({"accessToken": "a"}), None).is_err());
    }
}
