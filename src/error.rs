use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid type annotation: {0}")]
    InvalidTypeAnnotation(String),

    #[error("invalid path: {0}")]
    InvalidPath(String),

    #[error("invalid date: {0}")]
    InvalidDate(String),

    #[error("invalid bigint: {0}")]
    InvalidBigInt(String),

    #[error("invalid regexp: {0}")]
    InvalidRegExp(String),

    #[error("type mismatch at path '{path}': expected {expected}, got {actual}")]
    TypeMismatch {
        path: String,
        expected: String,
        actual: String,
    },
}

pub type Result<T> = std::result::Result<T, Error>;
