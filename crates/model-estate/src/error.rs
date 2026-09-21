use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum ModelError {
    #[error("model binding '{0}' is not wired")]
    NotWired(String),
    #[error("unknown model binding '{0}'")]
    Unknown(String),
    #[error("missing credential env {0} (never bake secrets into the estate file)")]
    MissingCreds(String),
    #[error("local specialist endpoint unset; set {0} (or run model-estate mock-local)")]
    MissingEndpoint(String),
    #[error("frontier specialist endpoint unset; set CELL_FRONTIER_ENDPOINT (XAI_API_KEY is not used)")]
    MissingFrontierEndpoint,
    #[error("driver unreachable: {0}")]
    Unreachable(String),
    #[error("driver refused: {0}")]
    Refused(String),
    #[error("estate denied: {0}")]
    Denied(String),
    #[error("local driver '{0}' is a stub (Apple MLX live proof later); fail closed")]
    Stub(String),
    #[error("local driver '{0}' is experimental until Jason verifies; fail closed")]
    Experimental(String),
    #[error("{0}")]
    Other(String),
}

impl ModelError {
    /// Estate-bound local work must fail closed. These errors must not fall through to frontier.
    pub fn is_local_down(&self) -> bool {
        matches!(
            self,
            ModelError::Unreachable(_)
                | ModelError::MissingEndpoint(_)
                | ModelError::Stub(_)
                | ModelError::Experimental(_)
                | ModelError::Refused(_)
        )
    }
}

impl PartialEq for ModelError {
    fn eq(&self, other: &Self) -> bool {
        self.to_string() == other.to_string()
    }
}

impl Eq for ModelError {}
