use serde::Serialize;
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, TS, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    InvalidInput,
    NotFound,
    Locked,
    Busy,
    RateLimited,
    UnsafeKey,
    Storage,
    Forbidden,
    Unsupported,
    Cancelled,
    Process,
}

#[derive(Clone, Debug, Serialize, TS, thiserror::Error)]
#[serde(rename_all = "camelCase")]
#[error("{message}")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub field: Option<String>,
}

pub type Result<T> = std::result::Result<T, AppError>;
impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            field: None,
        }
    }
    pub fn field(field: &str, message: impl Into<String>) -> Self {
        Self {
            code: ErrorCode::InvalidInput,
            message: message.into(),
            field: Some(field.into()),
        }
    }
    pub fn storage(error: impl std::fmt::Display) -> Self {
        Self::new(
            ErrorCode::Storage,
            format!("Local data could not be accessed: {error}"),
        )
    }
}
impl From<String> for AppError {
    fn from(message: String) -> Self {
        Self::new(ErrorCode::InvalidInput, message)
    }
}
impl From<&str> for AppError {
    fn from(message: &str) -> Self {
        Self::new(ErrorCode::InvalidInput, message)
    }
}
