use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

use crate::error::AppResult;

/// 先写临时文件再 rename，避免写一半时 CLI 读到残缺 JSON。
pub fn write_atomic(path: &Path, bytes: &[u8]) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("claude-auto.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn write_json_atomic(path: &Path, value: &Value) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    write_atomic(path, &bytes)
}

/// 读取 JSON 对象；文件不存在返回空对象。
pub fn read_json_object(path: &Path) -> AppResult<Map<String, Value>> {
    match fs::read(path) {
        Ok(bytes) => match serde_json::from_slice::<Value>(&bytes)? {
            Value::Object(m) => Ok(m),
            _ => Ok(Map::new()),
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Map::new()),
        Err(e) => Err(e.into()),
    }
}

pub fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}
