use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
