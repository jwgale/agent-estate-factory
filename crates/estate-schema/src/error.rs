use thiserror::Error;

#[derive(Debug, Error)]
pub enum EstateError {
    #[error("failed to parse estate: {0}")]
    Parse(String),
    #[error("invalid estate:\n{}", .0.join("\n"))]
    Invalid(Vec<String>),
    #[error("io error at {path}: {message}")]
    Io { path: String, message: String },
    #[error("{0}")]
    Other(String),
}

impl EstateError {
    pub fn errors(&self) -> Vec<String> {
        match self {
            EstateError::Invalid(v) => v.clone(),
            other => vec![other.to_string()],
        }
    }
}
