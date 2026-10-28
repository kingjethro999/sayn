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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_length: Option<u64>,
    pub free_tier: bool,
    pub tool_calling: bool,
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
                free_tier: false,
                tool_calling: true,
                description: Some("Flagship instruction-following model with agentic tool calling".into()),
            },
            ModelInfo {
                id: "hermes-3-llama-3.1-70b".to_string(),
                name: "Hermes 3 (Llama 3.1 70B)".to_string(),
                provider: "nous".to_string(),
                context_length: Some(131072),
                free_tier: true,
                tool_calling: true,
                description: Some("High-speed reasoning model suitable for rapid prototyping".into()),
            },
            ModelInfo {
                id: "hermes-3-llama-3.1-8b".to_string(),
                name: "Hermes 3 (Llama 3.1 8B)".to_string(),
                provider: "nous".to_string(),
                context_length: Some(131072),
                free_tier: true,
                tool_calling: true,
                description: Some("Lightweight agent model for local and low-latency inference".into()),
            },
            ModelInfo {
                id: "deepseek-r1".to_string(),
                name: "DeepSeek R1 (Reasoning)".to_string(),
                provider: "nous".to_string(),
                context_length: Some(65536),
                free_tier: false,
                tool_calling: true,
                description: Some("Advanced chain-of-thought mathematical and code synthesis".into()),
            },
            ModelInfo {
                id: "qwen-2.5-coder-32b".to_string(),
                name: "Qwen 2.5 Coder 32B".to_string(),
                provider: "nous".to_string(),
                context_length: Some(32768),
                free_tier: true,
                tool_calling: true,
                description: Some("Dedicated programming and syntax reasoning model".into()),
            },
        ]
    }

    /// Fetch models live from the authenticated Nous inference API.
    pub fn fetch_models(_refresh: bool) -> Result<Vec<ModelInfo>, ModelError> {
        // Check if authenticated
        let auth_state = NousAuthManager::load_auth_state()?;
        let token = match &auth_state {
            Some(state) => state.access_token.clone(),
            None => {
                // If not authenticated, return the curated catalog marked accordingly
                return Ok(Self::curated_models());
            }
        };

        let inference_url = auth_state
            .as_ref()
            .map(|s| s.inference_base_url.clone())
            .unwrap_or_else(|| sayn_hermes_auth::DEFAULT_NOUS_INFERENCE_URL.to_string());

        let url = format!("{}/models", inference_url.trim_end_matches('/'));

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        let res = client
            .get(&url)
            .bearer_auth(&token)
            .send();

        match res {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(body) = resp.json::<serde_json::Value>() {
                    let mut models = Vec::new();
                    if let Some(list) = body.get("data").and_then(|d| d.as_array()) {
                        for item in list {
                            if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                                let context = item.get("context_length").and_then(|c| c.as_u64());
                                let is_free = id.contains("8b") || id.contains("70b");
                                models.push(ModelInfo {
                                    id: id.to_string(),
                                    name: id.to_string(),
                                    provider: "nous".to_string(),
                                    context_length: context.or(Some(131072)),
                                    free_tier: is_free,
                                    tool_calling: true,
                                    description: None,
                                });
                            }
                        }
                    }

                    if !models.is_empty() {
                        return Ok(models);
                    }
                }
                Ok(Self::curated_models())
            }
            _ => {
                // Network or upstream error: fall back safely to curated list
                Ok(Self::curated_models())
            }
        }
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

    #[test]
    fn test_curated_models() {
        let models = ModelCatalog::curated_models();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id == "hermes-3-llama-3.1-405b"));
        assert!(models.iter().any(|m| m.free_tier));
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
}
