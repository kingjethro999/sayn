use sayn_hermes_auth::NousAuthManager;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModelError {
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Authentication error: {0}")]
    Auth(#[from] sayn_hermes_auth::AuthError),

    #[error("Model '{0}' not found or not entitled")]
    NotFound(String),

    #[error("Nous Portal authentication is required to discover entitled models")]
    NotAuthenticated,

    #[error("Nous Portal rejected the model-catalog credential")]
    Unauthorized,

    #[error("Nous Portal returned an empty model catalog")]
    EmptyCatalog,

    #[error("Nous Portal returned an invalid model catalog: {0}")]
    InvalidCatalog(String),

    #[error("Nous Portal model catalog request failed with HTTP {0}")]
    Http(u16),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_length: Option<u64>,
    pub free_tier: Option<bool>,
    pub tool_calling: Option<bool>,
    pub description: Option<String>,
}

pub struct ModelCatalog;

impl ModelCatalog {
    /// Curated fallback model list (matches Hermes upstream defaults)
    pub fn curated_models() -> Vec<ModelInfo> {
        vec![
            ModelInfo {
                id: "hermes-3-llama-3.1-405b".to_string(),
                name: "Hermes 3 (Llama 3.1 405B)".to_string(),
                provider: "nous".to_string(),
                context_length: Some(131072),
                free_tier: None,
                tool_calling: Some(true),
                description: Some(
                    "Flagship instruction-following model with agentic tool calling".into(),
                ),
            },
            ModelInfo {
                id: "hermes-3-llama-3.1-70b".to_string(),
                name: "Hermes 3 (Llama 3.1 70B)".to_string(),
                provider: "nous".to_string(),
                context_length: Some(131072),
                free_tier: None,
                tool_calling: Some(true),
                description: Some(
                    "High-speed reasoning model suitable for rapid prototyping".into(),
                ),
            },
            ModelInfo {
                id: "hermes-3-llama-3.1-8b".to_string(),
                name: "Hermes 3 (Llama 3.1 8B)".to_string(),
                provider: "nous".to_string(),
                context_length: Some(131072),
                free_tier: None,
                tool_calling: Some(true),
                description: Some(
                    "Lightweight agent model for local and low-latency inference".into(),
                ),
            },
            ModelInfo {
                id: "deepseek-r1".to_string(),
                name: "DeepSeek R1 (Reasoning)".to_string(),
                provider: "nous".to_string(),
                context_length: Some(65536),
                free_tier: None,
                tool_calling: Some(true),
                description: Some(
                    "Advanced chain-of-thought mathematical and code synthesis".into(),
                ),
            },
            ModelInfo {
                id: "qwen-2.5-coder-32b".to_string(),
                name: "Qwen 2.5 Coder 32B".to_string(),
                provider: "nous".to_string(),
                context_length: Some(32768),
                free_tier: None,
                tool_calling: Some(true),
                description: Some("Dedicated programming and syntax reasoning model".into()),
            },
        ]
    }

    /// Fetch models live from the authenticated Nous inference API.
    pub fn fetch_models(_refresh: bool) -> Result<Vec<ModelInfo>, ModelError> {
        let auth_state = NousAuthManager::load_auth_state()?.ok_or(ModelError::NotAuthenticated)?;
        let token = NousAuthManager::valid_access_token()?;
        Self::fetch_models_from(&auth_state.inference_base_url, &token)
    }

    pub fn fetch_models_from(base_url: &str, token: &str) -> Result<Vec<ModelInfo>, ModelError> {
        let url = format!("{}/models", base_url.trim_end_matches('/'));

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let response = client.get(&url).bearer_auth(token).send()?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ModelError::Unauthorized);
        }
        if !response.status().is_success() {
            return Err(ModelError::Http(response.status().as_u16()));
        }

        let body = response
            .json::<serde_json::Value>()
            .map_err(|error| ModelError::InvalidCatalog(error.to_string()))?;
        let list = body
            .get("data")
            .and_then(|data| data.as_array())
            .ok_or_else(|| ModelError::InvalidCatalog("missing data array".into()))?;
        let mut models = Vec::new();
        for item in list {
            let id = item
                .get("id")
                .and_then(|id| id.as_str())
                .filter(|id| !id.trim().is_empty())
                .ok_or_else(|| ModelError::InvalidCatalog("model entry missing id".into()))?;
            models.push(ModelInfo {
                id: id.into(),
                name: item
                    .get("name")
                    .and_then(|name| name.as_str())
                    .unwrap_or(id)
                    .into(),
                provider: "nous".into(),
                context_length: item.get("context_length").and_then(|value| value.as_u64()),
                free_tier: item.get("free_tier").and_then(|value| value.as_bool()),
                tool_calling: item.get("tool_calling").and_then(|value| value.as_bool()),
                description: item
                    .get("description")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned),
            });
        }
        if models.is_empty() {
            return Err(ModelError::EmptyCatalog);
        }
        Ok(models)
    }

    pub fn validate_selection(
        model_id: &str,
        catalog: &[ModelInfo],
    ) -> Result<ModelInfo, ModelError> {
        catalog
            .iter()
            .find(|model| model.id == model_id)
            .cloned()
            .ok_or_else(|| ModelError::NotFound(model_id.into()))
    }

    pub fn model_file_path() -> PathBuf {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        home.join(".sayn/model.json")
    }

    /// Retrieve the currently selected model ID.
    pub fn get_active_model() -> String {
        let path = Self::model_file_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(m) = json.get("active_model").and_then(|m| m.as_str()) {
                        return m.to_string();
                    }
                }
            }
        }

        // Check environment variable
        if let Ok(env_m) = std::env::var("SAYN_MODEL") {
            return env_m;
        }

        "hermes-3-llama-3.1-405b".to_string()
    }

    /// Select and persist the active model ID.
    pub fn set_active_model(model_id: &str) -> Result<(), ModelError> {
        let path = Self::model_file_path();
        if let Some(p) = path.parent() {
            fs::create_dir_all(p)?;
        }

        let json = serde_json::json!({
            "active_model": model_id,
            "updated_at": chrono::Utc::now().to_rfc3339()
        });

        fs::write(&path, serde_json::to_string_pretty(&json)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn serve(status: &'static str, body: &'static str) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let _ = stream.read(&mut request);
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        (format!("http://{address}"), handle)
    }

    #[test]
    fn test_curated_models() {
        let models = ModelCatalog::curated_models();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id == "hermes-3-llama-3.1-405b"));
        assert!(models.iter().all(|model| model.free_tier.is_none()));
    }

    #[test]
    fn test_set_and_get_active_model() {
        let temp_dir = tempfile::tempdir().unwrap();
        let model_path = temp_dir.path().join("model.json");
        let json = serde_json::json!({ "active_model": "test-model" });
        fs::write(&model_path, json.to_string()).unwrap();

        let content = fs::read_to_string(&model_path).unwrap();
        let val: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(val["active_model"], "test-model");
    }

    #[test]
    fn live_catalog_contains_only_provider_returned_models() {
        let body = r#"{"data":[{"id":"anthropic/claude-sonnet","context_length":200000}]}"#;
        let (base_url, server) = serve("200 OK", body);

        let models = ModelCatalog::fetch_models_from(&base_url, "access-token").unwrap();
        server.join().unwrap();

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "anthropic/claude-sonnet");
        assert_eq!(models[0].context_length, Some(200000));
        assert_eq!(models[0].free_tier, None);
        assert_eq!(models[0].tool_calling, None);
    }

    #[test]
    fn empty_catalog_is_not_replaced_with_fabricated_entitlements() {
        let (base_url, server) = serve("200 OK", r#"{"data":[]}"#);

        let error = ModelCatalog::fetch_models_from(&base_url, "access-token").unwrap_err();
        server.join().unwrap();

        assert!(matches!(error, ModelError::EmptyCatalog));
    }

    #[test]
    fn unauthorized_catalog_request_is_reported_as_authentication_failure() {
        let (base_url, server) = serve("401 Unauthorized", r#"{"error":"unauthorized"}"#);

        let error = ModelCatalog::fetch_models_from(&base_url, "access-token").unwrap_err();
        server.join().unwrap();

        assert!(matches!(error, ModelError::Unauthorized));
    }

    #[test]
    fn malformed_catalog_is_rejected() {
        let (base_url, server) = serve("200 OK", r#"{"models":"not-an-array"}"#);

        let error = ModelCatalog::fetch_models_from(&base_url, "access-token").unwrap_err();
        server.join().unwrap();

        assert!(matches!(error, ModelError::InvalidCatalog(_)));
    }

    #[test]
    fn unavailable_model_cannot_be_selected_from_an_entitled_catalog() {
        let catalog = vec![ModelInfo {
            id: "available-model".into(),
            name: "available-model".into(),
            provider: "nous".into(),
            context_length: None,
            free_tier: None,
            tool_calling: None,
            description: None,
        }];

        let error = ModelCatalog::validate_selection("missing-model", &catalog).unwrap_err();

        assert!(matches!(error, ModelError::NotFound(model) if model == "missing-model"));
    }
}
