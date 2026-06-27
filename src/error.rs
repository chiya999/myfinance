/// 统一错误类型
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// 数据库操作错误
    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),

    /// 文件 I/O 错误
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    /// JSON 序列化错误
    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),

    /// 资源未找到
    #[error("未找到: {0}")]
    NotFound(String),

    /// 输入校验错误
    #[error("校验错误: {0}")]
    Validation(String),

    /// 内部逻辑错误
    #[error("内部错误: {0}")]
    Internal(String),
}

/// 统一 Result 别名
pub type AppResult<T> = Result<T, AppError>;
