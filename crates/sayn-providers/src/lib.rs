pub mod error;
pub mod mock;
pub mod nous;
pub mod openai;
pub mod provider;

pub use error::ProviderError;
pub use mock::MockProvider;
pub use nous::{NOUS_INFERENCE_BASE_URL, NOUS_PROVIDER_NAME, NousProvider};
pub use openai::{OpenAiProvider, OpenAiProviderConfig};
pub use provider::{GenerationProvider, GenerationRequest};
pub use sayn_ir::{GeneratedOperation, GenerationResponse};

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> GenerationRequest {
        GenerationRequest {
            prompt: "Build portfolio".to_string(),
            target: "web".to_string(),
            instructions: "Create portfolio page".to_string(),
            max_tokens: Some(1000),
            temperature: Some(0.7),
        }
    }

    #[test]
    fn mock_provider_returns_a_valid_structured_response() {
        let response = MockProvider::default().generate(&request()).unwrap();
        assert!(response.validate().is_ok());
        assert!(
            response
                .operations
                .iter()
                .any(|operation| operation.path() == "app/page.tsx")
        );
    }

    #[test]
    fn openai_provider_without_a_key_fails_before_any_request() {
        let provider = OpenAiProvider::new(OpenAiProviderConfig {
            base_url: "http://127.0.0.1:9".to_string(),
            model: "gpt-4o".to_string(),
            api_key: None,
        });
        let error = provider.generate(&request()).unwrap_err();
        assert!(matches!(error, ProviderError::Authentication(_)));
    }
}
