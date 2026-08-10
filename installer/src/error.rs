use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("zip error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("invalid server address: {0}")]
    InvalidServer(String),

    #[error("invalid locale: {0}")]
    InvalidLocale(String),

    #[error("checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("file not found: {0}")]
    NotFound(PathBuf),

    #[error("cancelled by user")]
    Cancelled,

    #[error("{0}")]
    Msg(String),
}

impl From<&str> for Error {
    fn from(s: &str) -> Self {
        Error::Msg(s.to_string())
    }
}

impl From<String> for Error {
    fn from(s: String) -> Self {
        Error::Msg(s)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
