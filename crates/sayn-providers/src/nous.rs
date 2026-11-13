use crate::error::ProviderError;
use crate::openai::chat_completion;
use crate::provider::{GenerationProvider, GenerationRequest};
use sayn_ir::GenerationResponse;

pub const NOUS_PROVIDER_NAME: &str = "Nous Portal";
pub const NOUS_INFERENCE_BASE_URL: &str = "https://inference-api.nousresearch.com/v1";

/// Nous Portal inference through its OpenAI-compatible endpoint.
///
/// The caller supplies an already valid access token (OAuth or API key);
/// this type does not read or refresh stored credentials.
pub struct NousProvider {
    base_url: String,
    token: String,
    model: String,
}

impl NousProvider {
    pub fn new(
        base_url: impl Into<String>,
        token: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            token: token.into(),
            model: model.into(),
        }
    }

    /// Build from a `NOUS_API_KEY` environment credential, if present.
    pub fn from_env(model: &str) -> Option<Self> {
        let token = std::env::var("NOUS_API_KEY")
            .ok()
            .filter(|key| !key.trim().is_empty())?;
        Some(Self::new(NOUS_INFERENCE_BASE_URL, token, model))
    }

    pub fn name(&self) -> &str {
        NOUS_PROVIDER_NAME
    }
}

impl GenerationProvider for NousProvider {
    fn name(&self) -> &str {
        NOUS_PROVIDER_NAME
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn generate(&self, request: &GenerationRequest) -> Result<GenerationResponse, ProviderError> {
        chat_completion(&self.base_url, &self.token, &self.model, request)
    }
}

#[cfg(test)]
mod tests {
    use super::NousProvider;
    use crate::error::ProviderError;
    use crate::provider::{GenerationProvider, GenerationRequest};
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn request() -> GenerationRequest {
        GenerationRequest {
            prompt: "Generate the app".into(),
            target: "web".into(),
            instructions: "Create a portfolio".into(),
            max_tokens: None,
            temperature: None,
        }
    }

    /// Serve one canned HTTP response and return the captured request text.
    fn serve(status: &'static str, body: String) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = [0_u8; 16384];
            let count = stream.read(&mut bytes).unwrap();
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
            String::from_utf8_lossy(&bytes[..count]).into_owned()
        });
        (format!("http://{address}/v1"), handle)
    }

    fn completion(content: &str) -> String {
        serde_json::json!({"choices": [{"message": {"content": content}}]}).to_string()
    }

    #[test]
    fn identifies_as_nous_portal() {
        let provider = NousProvider::new(
            "https://inference-api.nousresearch.com/v1",
            "token",
            "model",
        );
        assert_eq!(provider.name(), "Nous Portal");
    }

    #[test]
    fn sends_the_token_and_model_to_chat_completions() {
        let content = r#"{"schema_version":1,"operations":[{"kind":"create","path":"app/page.tsx","content":"x"}]}"#;
        let (base_url, server) = serve("200 OK", completion(content));
        let provider = NousProvider::new(base_url, "nous-secret", "Hermes-4-405B");

        let result = GenerationProvider::generate(&provider, &request()).unwrap();
        let request_text = server.join().unwrap();

        assert_eq!(result.operations.len(), 1);
        assert!(request_text.starts_with("POST /v1/chat/completions"));
        assert!(
            request_text
                .to_lowercase()
                .contains("authorization: bearer nous-secret")
        );
        assert!(request_text.contains("Hermes-4-405B"));
    }

    #[test]
    fn classifies_rate_limits_and_server_errors() {
        let (base_url, server) = serve("429 Too Many Requests", "{}".into());
        let provider = NousProvider::new(base_url, "nous-secret", "model");
        let error = GenerationProvider::generate(&provider, &request()).unwrap_err();
        server.join().unwrap();
        assert!(matches!(error, ProviderError::RateLimit(_)));

        let (base_url, server) = serve(
            "500 Internal Server Error",
            r#"{"error":"nous-secret"}"#.into(),
        );
        let provider = NousProvider::new(base_url, "nous-secret", "model");
        let error = GenerationProvider::generate(&provider, &request()).unwrap_err();
        server.join().unwrap();
        assert!(matches!(error, ProviderError::General(_)));
        assert!(!error.to_string().contains("nous-secret"));
    }

    #[test]
    fn rejects_unknown_operation_kinds_and_invalid_schema() {
        for content in [
            r#"{"schema_version":1,"operations":[{"kind":"run_shell","path":"a","content":"x"}]}"#,
            r#"{"operations":"not a list"}"#,
            "Sorry, I can't do that.",
        ] {
            let (base_url, server) = serve("200 OK", completion(content));
            let provider = NousProvider::new(base_url, "token", "model");
            let error = GenerationProvider::generate(&provider, &request()).unwrap_err();
            server.join().unwrap();
            assert!(
                matches!(error, ProviderError::InvalidResponse(_)),
                "content {content:?} should be an invalid response"
            );
        }
    }
}
