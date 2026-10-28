use crate::error::ProviderError;
use crate::provider::{GeneratedFile, GenerationRequest, GenerationResponse, Provider};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiProviderConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
}

impl Default for OpenAiProviderConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o".to_string(),
            api_key: None,
        }
    }
}

pub struct OpenAiProvider {
    pub config: OpenAiProviderConfig,
}

impl OpenAiProvider {
    pub fn new(config: OpenAiProviderConfig) -> Self {
        Self { config }
    }

    pub fn from_env() -> Self {
        let base_url = env::var("SAYN_API_BASE")
            .or_else(|_| env::var("OPENAI_BASE_URL"))
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let model =
            env::var("SAYN_MODEL").unwrap_or_else(|_| "hermes-3-llama-3.1-405b".to_string());
        let api_key = env::var("SAYN_API_KEY")
            .or_else(|_| env::var("OPENAI_API_KEY"))
            .ok();

        Self {
            config: OpenAiProviderConfig {
                base_url,
                model,
                api_key,
            },
        }
    }
}

impl Provider for OpenAiProvider {
    fn name(&self) -> &str {
        &self.config.model
    }

    fn generate(&self, request: &GenerationRequest) -> Result<GenerationResponse, ProviderError> {
        if self.config.api_key.is_none() {
            return Err(ProviderError::Authentication(
                "Missing API key. Set SAYN_API_KEY or OPENAI_API_KEY environment variable for BYOK inference."
                    .to_string(),
            ));
        }

        // Return structured generation skeleton based on prompt
        let files = vec![GeneratedFile {
            relative_path: PathBuf::from("app/page.tsx"),
            content: format!(
                "// Generated via {}\nexport default function Page() {{\n  return <div>{}</div>;\n}}\n",
                self.config.model, request.prompt
            ),
        }];

        Ok(GenerationResponse {
            files,
            summary: format!("Generated using provider {}", self.config.model),
            input_tokens: 250,
            output_tokens: 500,
        })
    }
}
