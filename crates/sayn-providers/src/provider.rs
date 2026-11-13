use crate::error::ProviderError;
use sayn_ir::GenerationResponse;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub prompt: String,
    pub target: String,
    pub instructions: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
}

/// A source of structured generation proposals.
///
/// Implementations return a schema-validated [`GenerationResponse`] or an
/// error. They never write files and never substitute canned output when the
/// upstream model is unavailable.
pub trait GenerationProvider: Send + Sync {
    /// Human-readable provider name for diagnostics, without credentials.
    fn name(&self) -> &str;

    /// The model identifier this provider will request.
    fn model(&self) -> &str;

    fn generate(&self, request: &GenerationRequest) -> Result<GenerationResponse, ProviderError>;
}
