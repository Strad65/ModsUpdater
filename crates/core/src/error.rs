use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("API error: {0}")]
    Api(#[from] ApiError),

    #[error("Scan error: {0}")]
    Scan(String),

    #[error("Update error: {0}")]
    Update(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Hash verification failed for {filename}: expected {expected}, got {actual}")]
    HashMismatch {
        filename: String,
        expected: String,
        actual: String,
    },
}

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Rate limited. Retry after {retry_after:?}")]
    RateLimited { retry_after: Option<std::time::Duration> },

    #[error("API returned error {status}: {message}")]
    ApiError { status: u16, message: String },

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Request failed: {0}")]
    RequestFailed(String),
}

pub type CoreResult<T> = Result<T, CoreError>;
