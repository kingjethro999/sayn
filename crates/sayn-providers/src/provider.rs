use crate::error::ProviderError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    pub prompt: String,
    pub target: String,
    pub instructions: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFile {
    pub relative_path: PathBuf,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResponse {
    pub files: Vec<GeneratedFile>,
    pub summary: String,
    pub input_tokens: usize,
    pub output_tokens: usize,
}

pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn generate(&self, request: &GenerationRequest) -> Result<GenerationResponse, ProviderError>;
}
