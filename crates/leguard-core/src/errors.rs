use thiserror::Error;

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("dataset path does not exist: {0}")]
    MissingPath(String),
    #[error("dataset path is not a directory: {0}")]
    NotDirectory(String),
}
