//! `error`模块定义了`AppError`枚举，它包含了应用可能会遇到的所有错误类型。
//!
//! `AppError`枚举包含以下几种错误类型：
//! - `DatabaseError`: 数据库错误，包装了`sqlx::Error`。
//! - `InvalidUrl`: 无效的URL错误，包含了无效的URL字符串。
//! - `UrlNotFound`: URL未找到错误。
//! - `InvalidHeader`: 无效的header值错误，包装了`InvalidHeaderValue`。
//!
//! 此外，`AppError`实现了`IntoResponse` trait，可以将`AppError`转换为HTTP响应。这使得错误处理更加方便，可以直接将错误转换为对应的HTTP状态码和错误消息。

use axum::response::IntoResponse;
use http::header::InvalidHeaderValue;
use http::StatusCode;
use thiserror::Error;

/// AppError枚举，定义了应用可能会遇到的错误类型。
#[derive(Debug, Error)]
pub enum AppError {
    /// 数据库错误，包装了sqlx::Error。
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    /// 无效的URL错误，包含了无效的URL字符串。
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// URL未找到错误。
    #[error("URL not found")]
    UrlNotFound,

    /// 无效的header值错误，包装了InvalidHeaderValue。
    #[error("Invalid header value: {0}")]
    InvalidHeader(#[from] InvalidHeaderValue),
}

/// AppError的IntoResponse实现，将AppError转换为HTTP响应。
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        // 根据不同的错误类型，设置不同的HTTP状态码和错误消息。
        let (status, error_message) = match self {
            AppError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::InvalidUrl(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::UrlNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::InvalidHeader(_) => (StatusCode::BAD_REQUEST, self.to_string()),
        };
        // 将状态码和错误消息转换为HTTP响应。
        (status, error_message).into_response()
    }
}
