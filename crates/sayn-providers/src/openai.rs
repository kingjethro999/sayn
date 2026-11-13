use crate::error::ProviderError;
use crate::provider::{GenerationProvider, GenerationRequest};
use sayn_ir::GenerationResponse;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::Read;
use std::time::Duration;

/// Upper bound on one inference round trip.
pub const REQUEST_TIMEOUT_SECS: u64 = 180;

/// Upper bound on an accepted response body.
pub const MAX_RESPONSE_BYTES: u64 = 8 * 1024 * 1024;

const SYSTEM_PROMPT: &str = "You are the code generation backend of the Sayn compiler. \
Follow the user's instructions exactly and reply with ONLY one JSON object of this shape: \
{\"schema_version\": 1, \"operations\": [{\"kind\": \"create\", \"path\": \"app/page.tsx\", \"content\": \"...\"}], \
\"acceptance_checks\": [\"...\"], \"summary\": \"...\"}. \
Allowed operation kinds: \"create\" and \"modify\" (with path and full file content), \
\"delete\" and \"retain\" (with path only). Paths are relative to the generated project root; \
never use absolute paths or '..'. Target each path at most once. Do not include any other text.";

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

/// BYOK provider for any OpenAI-compatible chat completions endpoint.
pub struct OpenAiProvider {
    pub config: OpenAiProviderConfig,
}

impl OpenAiProvider {
    pub fn new(config: OpenAiProviderConfig) -> Self {
        Self { config }
    }

    /// Build a BYOK provider from `SAYN_API_KEY` / `OPENAI_API_KEY`.
    ///
    /// Returns `None` when no key is present. `SAYN_MODEL` overrides
    /// `default_model`; `SAYN_API_BASE` / `OPENAI_BASE_URL` override the endpoint.
    pub fn from_env(default_model: &str) -> Option<Self> {
        let api_key = env::var("SAYN_API_KEY")
            .or_else(|_| env::var("OPENAI_API_KEY"))
            .ok()
            .filter(|key| !key.trim().is_empty())?;
        let base_url = env::var("SAYN_API_BASE")
            .or_else(|_| env::var("OPENAI_BASE_URL"))
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let model = env::var("SAYN_MODEL").unwrap_or_else(|_| default_model.to_string());

        Some(Self::new(OpenAiProviderConfig {
            base_url,
            model,
            api_key: Some(api_key),
        }))
    }
}

impl GenerationProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "BYOK (OpenAI-compatible)"
    }

    fn model(&self) -> &str {
        &self.config.model
    }

    fn generate(&self, request: &GenerationRequest) -> Result<GenerationResponse, ProviderError> {
        let api_key = self.config.api_key.as_deref().ok_or_else(|| {
            ProviderError::Authentication(
                "missing API key; set SAYN_API_KEY or OPENAI_API_KEY for BYOK inference".into(),
            )
        })?;
        chat_completion(&self.config.base_url, api_key, &self.config.model, request)
    }
}

/// Send one structured-generation request to an OpenAI-compatible endpoint.
///
/// Error messages never include the response body or the credential: upstream
/// error bodies can echo the token back.
pub(crate) fn chat_completion(
    base_url: &str,
    token: &str,
    model: &str,
    request: &GenerationRequest,
) -> Result<GenerationResponse, ProviderError> {
    let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let mut body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": request.prompt},
        ],
        "temperature": request.temperature.unwrap_or(0.2),
        "response_format": {"type": "json_object"},
    });
    if let Some(max_tokens) = request.max_tokens {
        body["max_tokens"] = serde_json::json!(max_tokens);
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|_| ProviderError::General("could not initialize the HTTP client".into()))?;

    let response = client
        .post(&endpoint)
        .bearer_auth(token)
        .json(&body)
        .send()
        .map_err(|error| {
            if error.is_timeout() {
                ProviderError::Timeout(REQUEST_TIMEOUT_SECS)
            } else if error.is_connect() {
                ProviderError::Network("could not connect to the inference endpoint".into())
            } else {
                ProviderError::Network("the inference request failed in transit".into())
            }
        })?;

    let status = response.status();
    if !status.is_success() {
        return Err(match status.as_u16() {
            401 | 403 => ProviderError::Authentication(format!(
                "the provider rejected the credential (HTTP {})",
                status.as_u16()
            )),
            404 => ProviderError::General(format!(
                "the endpoint or model '{model}' was not found (HTTP 404)"
            )),
            429 => ProviderError::RateLimit(
                "the provider reported a rate or quota limit (HTTP 429)".into(),
            ),
            code => ProviderError::General(format!("the provider returned HTTP {code}")),
        });
    }

    let mut bytes = Vec::new();
    response
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::TimedOut {
                ProviderError::Timeout(REQUEST_TIMEOUT_SECS)
            } else {
                ProviderError::Network("the response ended before it was complete".into())
            }
        })?;
    if bytes.len() as u64 > MAX_RESPONSE_BYTES {
        return Err(ProviderError::InvalidResponse(format!(
            "the response exceeded the {MAX_RESPONSE_BYTES}-byte limit"
        )));
    }

    let envelope: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|_| ProviderError::InvalidResponse("the response body was not JSON".into()))?;
    let choice = envelope.get("choices").and_then(|choices| choices.get(0));
    if choice
        .and_then(|choice| choice.get("finish_reason"))
        .and_then(|reason| reason.as_str())
        == Some("length")
    {
        return Err(ProviderError::InvalidResponse(
            "the model stopped at its output limit before finishing the response".into(),
        ));
    }
    let content = choice
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(|content| content.as_str())
        .filter(|content| !content.trim().is_empty())
        .ok_or_else(|| {
            ProviderError::InvalidResponse("the response had no assistant content".into())
        })?;

    GenerationResponse::parse(content)
        .map_err(|error| ProviderError::InvalidResponse(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::GenerationProvider;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn request() -> GenerationRequest {
        GenerationRequest {
            prompt: "Generate the app".into(),
            target: "web".into(),
            instructions: "Create a portfolio".into(),
            max_tokens: Some(2000),
            temperature: Some(0.2),
        }
    }

    fn serve(
        status: &'static str,
        body: &'static str,
    ) -> (String, Arc<Mutex<String>>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let captured = Arc::new(Mutex::new(String::new()));
        let captured_for_server = Arc::clone(&captured);
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut bytes = [0_u8; 16384];
            let count = stream.read(&mut bytes).unwrap();
            *captured_for_server.lock().unwrap() =
                String::from_utf8_lossy(&bytes[..count]).into_owned();
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        (format!("http://{address}/v1"), captured, handle)
    }

    #[test]
    fn sends_selected_model_and_parses_fenced_structured_operations() {
        let content = r#"```json\n{"schema_version":1,"operations":[{"kind":"create","path":"app/page.tsx","content":"export default function Page(){}"}],"acceptance_checks":["npm run build"],"summary":"Created page"}\n```"#;
        let response = serde_json::json!({
            "choices": [{"message": {"content": content}}],
            "usage": {"prompt_tokens": 12, "completion_tokens": 34}
        })
        .to_string();
        let leaked: &'static str = Box::leak(response.into_boxed_str());
        let (base_url, captured, server) = serve("200 OK", leaked);
        let provider = OpenAiProvider::new(OpenAiProviderConfig {
            base_url,
            model: "account/model-1".into(),
            api_key: Some("provider-secret".into()),
        });

        let result = GenerationProvider::generate(&provider, &request()).unwrap();
        server.join().unwrap();

        assert_eq!(result.schema_version, 1);
        assert_eq!(result.operations.len(), 1);
        assert_eq!(result.acceptance_checks, vec!["npm run build"]);
        let request_text = captured.lock().unwrap();
        assert!(request_text.contains("account/model-1"));
        assert!(request_text.contains("Bearer provider-secret"));
    }

    #[test]
    fn rejects_empty_operation_lists() {
        let content =
            r#"{"schema_version":1,"operations":[],"acceptance_checks":[],"summary":"none"}"#;
        let response = serde_json::json!({"choices":[{"message":{"content":content}}]}).to_string();
        let leaked: &'static str = Box::leak(response.into_boxed_str());
        let (base_url, _, server) = serve("200 OK", leaked);
        let provider = OpenAiProvider::new(OpenAiProviderConfig {
            base_url,
            model: "model".into(),
            api_key: Some("secret".into()),
        });

        let error = GenerationProvider::generate(&provider, &request()).unwrap_err();
        server.join().unwrap();

        assert!(matches!(error, ProviderError::InvalidResponse(_)));
    }

    #[test]
    fn classifies_unauthorized_responses_without_echoing_the_body() {
        let (base_url, _, server) = serve(
            "401 Unauthorized",
            r#"{"error":"token provider-secret rejected"}"#,
        );
        let provider = OpenAiProvider::new(OpenAiProviderConfig {
            base_url,
            model: "model".into(),
            api_key: Some("provider-secret".into()),
        });

        let error = GenerationProvider::generate(&provider, &request()).unwrap_err();
        server.join().unwrap();

        assert!(matches!(error, ProviderError::Authentication(_)));
        assert!(!error.to_string().contains("provider-secret"));
    }
}
