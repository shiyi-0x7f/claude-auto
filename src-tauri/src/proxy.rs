//! 代理解析：本工具的所有请求（额度、刷新、登录）都走同一套代理。
//!
//! 从桌面启动时进程里没有 CLI settings.json 的 env，所以默认直接读取该文件。

use std::path::Path;

use serde_json::Value;

use crate::fsutil::read_json_object;
use crate::settings::{ProxyMode, Settings};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyConf {
    pub url: String,
    pub no_proxy: Option<String>,
}

fn env_get<'a>(env: &'a serde_json::Map<String, Value>, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|k| env.get(*k).and_then(Value::as_str))
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

/// 读取 CLI `settings.json` 中 env 段的代理配置。
pub fn from_cli_settings(config_dir: &Path) -> Option<ProxyConf> {
    let map = read_json_object(&config_dir.join("settings.json")).ok()?;
    let env = map.get("env")?.as_object()?;
    let url = env_get(
        env,
        &["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"],
    )?;
    Some(ProxyConf {
        url: url.to_string(),
        no_proxy: env_get(env, &["NO_PROXY", "no_proxy"]).map(String::from),
    })
}

fn from_process_env() -> Option<ProxyConf> {
    let get = |keys: &[&str]| {
        keys.iter()
            .find_map(|k| std::env::var(k).ok())
            .filter(|s| !s.trim().is_empty())
    };
    Some(ProxyConf {
        url: get(&["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy"])?,
        no_proxy: get(&["NO_PROXY", "no_proxy"]),
    })
}

/// 按设置解析最终代理；None 表示直连。
pub fn resolve(settings: &Settings, cli_config_dir: &Path) -> Option<ProxyConf> {
    match settings.proxy_mode {
        ProxyMode::None => None,
        ProxyMode::Custom => Some(ProxyConf {
            url: settings.proxy_url.trim().to_string(),
            no_proxy: None,
        })
        .filter(|p| !p.url.is_empty()),
        ProxyMode::Cli => from_cli_settings(cli_config_dir).or_else(from_process_env),
    }
}

/// 展示/日志用：隐去 URL 中的账号密码。
pub fn redact(url: &str) -> String {
    match (url.find("://"), url.rfind('@')) {
        (Some(scheme_end), Some(at)) if at > scheme_end => {
            format!("{}***@{}", &url[..scheme_end + 3], &url[at + 1..])
        }
        _ => url.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_credentials() {
        assert_eq!(redact("http://u:p@1.2.3.4:80"), "http://***@1.2.3.4:80");
        assert_eq!(redact("http://1.2.3.4:80"), "http://1.2.3.4:80");
    }

    #[test]
    fn reads_cli_settings_env() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.json"),
            r#"{"env":{"HTTP_PROXY":"http://a:1","HTTPS_PROXY":"http://b:2","NO_PROXY":"localhost"}}"#,
        )
        .unwrap();
        let p = from_cli_settings(dir.path()).unwrap();
        assert_eq!(p.url, "http://b:2");
        assert_eq!(p.no_proxy.as_deref(), Some("localhost"));
    }

    #[test]
    fn modes() {
        let dir = tempfile::tempdir().unwrap();
        let mut s = Settings {
            proxy_mode: ProxyMode::None,
            ..Settings::default()
        };
        assert!(resolve(&s, dir.path()).is_none());
        s.proxy_mode = ProxyMode::Custom;
        s.proxy_url = " http://x:1 ".into();
        assert_eq!(resolve(&s, dir.path()).unwrap().url, "http://x:1");
    }
}
