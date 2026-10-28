pub mod error;
pub mod mock;
pub mod openai;
pub mod provider;

pub use error::ProviderError;
pub use mock::MockProvider;
pub use openai::{OpenAiProvider, OpenAiProviderConfig};
pub use provider::{GeneratedFile, GenerationRequest, GenerationResponse, Provider};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_provider() {
        let provider = MockProvider::default();
        let request = GenerationRequest {
            prompt: "Build portfolio".to_string(),
            target: "web".to_string(),
            instructions: "Create portfolio page".to_string(),
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        let response = provider.generate(&request).expect("Mock generation failed");
        assert_eq!(response.files.len(), 2);
        assert!(
            response
                .files
                .iter()
                .any(|f| f.relative_path.to_str() == Some("app/page.tsx"))
        );
        assert!(response.output_tokens > 0);
    }

    #[test]
    fn test_openai_provider_missing_key() {
        let config = OpenAiProviderConfig {
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o".to_string(),
            api_key: None,
        };
        let provider = OpenAiProvider::new(config);
        let request = GenerationRequest {
            prompt: "Test".to_string(),
            target: "web".to_string(),
            instructions: "Test".to_string(),
            max_tokens: None,
            temperature: None,
        };
        let result = provider.generate(&request);
        assert!(result.is_err());
    }
}
