use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Invalid model response: {0}")]
    InvalidResponse(String),

    #[error("Provider timeout after {0} seconds")]
    Timeout(u64),

    #[error("General provider error: {0}")]
    General(String),
}
