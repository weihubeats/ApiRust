use uuid::Uuid;

/// 统一错误类型：所有用户可见错误都必须映射到该类型，UI 层统一转换为中文提示。
#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("openapi error: {0}")]
    OpenApi(String),

    #[error("mock error: {0}")]
    Mock(String),

    #[error("test error: {0}")]
    Test(String),

    #[error("script error: {0}")]
    ScriptError(String),

    #[error("websocket error: {0}")]
    WebSocket(String),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

impl AppError {
    /// 用户可见的中文错误消息。
    pub fn user_message(&self) -> String {
        match self {
            AppError::Database(_) => "数据库操作失败".to_string(),
            AppError::Io(_) => format!("文件操作失败：{self}"),
            AppError::Http(_) => "网络请求失败".to_string(),
            AppError::Validation(msg) => msg.clone(),
            AppError::NotFound(name) => format!("未找到：{name}"),
            AppError::OpenApi(msg) => msg.clone(),
            AppError::Mock(msg) => msg.clone(),
            AppError::Test(msg) => msg.clone(),
            AppError::ScriptError(msg) => msg.clone(),
            AppError::WebSocket(msg) => msg.clone(),
            AppError::Json(_) => "JSON 解析失败".to_string(),
        }
    }
}

/// 便捷的未找到错误构造。
pub fn not_found(what: &str, id: Uuid) -> AppError {
    AppError::NotFound(format!("{what}（{id}）"))
}

pub fn validation(msg: impl Into<String>) -> AppError {
    AppError::Validation(msg.into())
}

pub fn openapi_error(msg: impl Into<String>) -> AppError {
    AppError::OpenApi(msg.into())
}

pub fn script_error(msg: impl Into<String>) -> AppError {
    AppError::ScriptError(msg.into())
}

pub fn ws_error(msg: impl Into<String>) -> AppError {
    AppError::WebSocket(msg.into())
}
