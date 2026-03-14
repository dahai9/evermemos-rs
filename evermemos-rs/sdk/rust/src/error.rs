use thiserror::Error;

#[derive(Debug, Error)]
pub enum EverMemOSError {
    #[error("invalid client config: {0}")]
    InvalidConfig(&'static str),
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json parse failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("evermemos returned non-success status: {status} ({message})")]
    ApiStatus { status: String, message: String },
}
