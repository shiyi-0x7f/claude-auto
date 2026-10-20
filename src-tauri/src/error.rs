use serde::{Serialize, Serializer};

/// 全局错误类型。序列化给前端时统一为 `{ code, message }`。
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("文件读写失败: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON 解析失败: {0}")]
    Json(#[from] serde_json::Error),
    #[error("网络请求失败: {0}")]
    Http(String),
    /// 令牌失效（401/403 或刷新被拒），需要重新登录该账号
    #[error("凭据失效，需要重新登录: {0}")]
    Auth(String),
    #[error("未找到: {0}")]
    NotFound(String),
    #[error("{0}")]
    Invalid(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Io(_) => "IO",
            AppError::Json(_) => "JSON",
            AppError::Http(_) => "HTTP",
            AppError::Auth(_) => "AUTH",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Invalid(_) => "INVALID",
        }
    }
}

impl From<reqwest::Error> for AppError {
    fn from(e: reqwest::Error) -> Self {
        AppError::Http(e.to_string())
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("code", self.code())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

pub type AppResult<T> = Result<T, AppError>;
