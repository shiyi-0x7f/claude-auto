use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::fsutil::write_atomic;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SwitchMode {
    /// 达到阈值时提示，用户一键确认后切换
    Semi,
    /// 达到阈值时直接切换并通知
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProxyMode {
    /// 读取 CLI settings.json 的 env 代理（HTTPS_PROXY/HTTP_PROXY/NO_PROXY），没有则用进程环境变量
    Cli,
    /// 使用 `proxy_url`
    Custom,
    /// 直连
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub mode: SwitchMode,
    /// 5 小时窗口已用百分比达到该值视为耗尽
    pub five_hour_threshold: f64,
    /// 周窗口已用百分比达到该值视为耗尽
    pub seven_day_threshold: f64,
    pub poll_secs: u64,
    pub proxy_mode: ProxyMode,
    pub proxy_url: String,
    /// 隐私模式：界面、托盘、通知中遮挡账号名（防录屏）
    pub privacy_mode: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: SwitchMode::Semi,
            five_hour_threshold: 95.0,
            seven_day_threshold: 95.0,
            poll_secs: 180,
            proxy_mode: ProxyMode::Cli,
            proxy_url: String::new(),
            privacy_mode: true,
        }
    }
}

pub const MIN_POLL_SECS: u64 = 60;
pub const MAX_POLL_SECS: u64 = 3600;

impl Settings {
    pub fn validate(&self) -> AppResult<()> {
        for (name, v) in [
            ("5 小时阈值", self.five_hour_threshold),
            ("周阈值", self.seven_day_threshold),
        ] {
            if !(1.0..=100.0).contains(&v) {
                return Err(AppError::Invalid(format!("{name}须在 1~100 之间")));
            }
        }
        if !(MIN_POLL_SECS..=MAX_POLL_SECS).contains(&self.poll_secs) {
            return Err(AppError::Invalid(format!(
                "轮询间隔须在 {MIN_POLL_SECS}~{MAX_POLL_SECS} 秒之间"
            )));
        }
        if self.proxy_mode == ProxyMode::Custom {
            reqwest::Proxy::all(self.proxy_url.trim())
                .map_err(|e| AppError::Invalid(format!("代理地址无效: {e}")))?;
        }
        Ok(())
    }
}

pub struct SettingsStore {
    path: PathBuf,
    pub value: Settings,
}

impl SettingsStore {
    pub fn load(dir: &Path) -> AppResult<Self> {
        let path = dir.join("settings.json");
        let value = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self { path, value })
    }

    pub fn set(&mut self, value: Settings) -> AppResult<()> {
        value.validate()?;
        write_atomic(&self.path, &serde_json::to_vec_pretty(&value)?)?;
        self.value = value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_out_of_range() {
        let s = Settings {
            five_hour_threshold: 0.0,
            ..Settings::default()
        };
        assert!(s.validate().is_err());
        let s = Settings {
            poll_secs: 5,
            ..Settings::default()
        };
        assert!(s.validate().is_err());
        assert!(Settings::default().validate().is_ok());
    }

    #[test]
    fn partial_file_uses_defaults() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.json"), r#"{"mode":"auto"}"#).unwrap();
        let s = SettingsStore::load(dir.path()).unwrap();
        assert_eq!(s.value.mode, SwitchMode::Auto);
        assert_eq!(s.value.poll_secs, 180);
    }
}
