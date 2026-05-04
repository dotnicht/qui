use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdminError {
    #[error("socket I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("qubesd exception ({exc_type}): {message}")]
    QubesDException { exc_type: String, message: String },

    #[error("connection lost")]
    ConnectionLost,

    #[error("parse error: {0}")]
    Parse(String),
}

pub type AdminResult<T> = Result<T, AdminError>;
