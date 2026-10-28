use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

pub const DEFAULT_NOUS_PORTAL_URL: &str = "https://portal.nousresearch.com";
pub const DEFAULT_NOUS_INFERENCE_URL: &str = "https://inference-api.nousresearch.com/v1";
pub const DEFAULT_NOUS_WELCOME_URL: &str = "https://welcome-api.nousresearch.com/v1";
pub const DEFAULT_NOUS_CLIENT_ID: &str = "hermes-cli";
pub const DEFAULT_NOUS_SCOPE: &str = "inference:invoke";
pub const DEVICE_CODE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Network request error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Device authorization timed out or expired: {0}")]
    Timeout(String),

    #[error("Authentication failed: {error} - {description}")]
    ServerReject {
        error: String,
        description: String,
    },

    #[error("Invalid credentials state: {0}")]
    InvalidState(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NousTokenResponse {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default = "default_token_type")]
    pub token_type: String,
    #[serde(default)]
    pub expires_in: Option<u64>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub inference_base_url: Option<String>,
}

fn default_token_type() -> String {
    "Bearer".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NousAuthState {
    pub portal_base_url: String,
    pub inference_base_url: String,
    pub client_id: String,
    pub scope: String,
    pub token_type: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub obtained_at: String,
    pub expires_at: String,
    pub expires_in: u64,
    pub free_tier: bool,
}

impl NousAuthState {
    pub fn is_expired(&self) -> bool {
        if let Ok(exp) = chrono::DateTime::parse_from_rfc3339(&self.expires_at) {
            // Skew by 120s before actual expiry
            chrono::Utc::now() >= (exp.with_timezone(&chrono::Utc) - chrono::Duration::seconds(120))
        } else {
            false
        }
    }
}

pub struct NousAuthManager;

impl NousAuthManager {
    /// Request a new device code from Nous Portal.
    pub fn request_device_code(portal_url: Option<&str>) -> Result<DeviceCodeResponse, AuthError> {
        let base_url = portal_url.unwrap_or(DEFAULT_NOUS_PORTAL_URL).trim_end_matches('/');
        let url = format!("{}/api/oauth/device/code", base_url);

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()?;

        let mut params = HashMap::new();
        params.insert("client_id", DEFAULT_NOUS_CLIENT_ID);
        params.insert("scope", DEFAULT_NOUS_SCOPE);

        let res = client
            .post(&url)
            .form(&params)
            .send()?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().unwrap_or_default();
            return Err(AuthError::ServerReject {
                error: format!("HTTP {}", status),
                description: body,
            });
        }

        let device_data: DeviceCodeResponse = res.json()?;
        Ok(device_data)
    }

    /// Poll Nous Portal token endpoint until approval or timeout.
    pub fn poll_device_token<F>(
        portal_url: Option<&str>,
        device_code: &str,
        expires_in: u64,
        interval: u64,
        mut on_poll: F,
    ) -> Result<NousTokenResponse, AuthError>
    where
        F: FnMut(u64), // elapsed seconds
    {
        let base_url = portal_url.unwrap_or(DEFAULT_NOUS_PORTAL_URL).trim_end_matches('/');
        let url = format!("{}/api/oauth/token", base_url);

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()?;

        let effective_interval = interval.max(1).min(30);
        let start = Instant::now();
        let timeout_dur = Duration::from_secs(expires_in.max(30));

        loop {
            let elapsed = start.elapsed();
            if elapsed >= timeout_dur {
                return Err(AuthError::Timeout(
                    "Timed out waiting for device authorization. Please retry 'sayn auth nous'.".to_string(),
                ));
            }

            on_poll(elapsed.as_secs());

            let mut params = HashMap::new();
            params.insert("client_id", DEFAULT_NOUS_CLIENT_ID);
            params.insert("grant_type", DEVICE_CODE_GRANT_TYPE);
            params.insert("device_code", device_code);

            let res = client.post(&url).form(&params).send()?;

            if res.status().is_success() {
                let token_data: NousTokenResponse = res.json()?;
                return Ok(token_data);
            }

            let status = res.status();
            let body_text = res.text().unwrap_or_default();

            if let Ok(err_json) = serde_json::from_str::<serde_json::Value>(&body_text) {
                let err = err_json.get("error").and_then(|e| e.as_str()).unwrap_or("");
                if err == "authorization_pending" {
                    thread::sleep(Duration::from_secs(effective_interval));
                    continue;
                } else if err == "slow_down" {
                    thread::sleep(Duration::from_secs(effective_interval + 2));
                    continue;
                } else {
                    let desc = err_json
                        .get("error_description")
                        .and_then(|d| d.as_str())
                        .unwrap_or("Authorization refused");
                    return Err(AuthError::ServerReject {
                        error: err.to_string(),
                        description: desc.to_string(),
                    });
                }
            } else {
                return Err(AuthError::ServerReject {
                    error: format!("HTTP {}", status),
                    description: body_text,
                });
            }
        }
    }

    /// Refresh expired Nous token.
    pub fn refresh_token(
        portal_url: Option<&str>,
        refresh_token: &str,
    ) -> Result<NousTokenResponse, AuthError> {
        let base_url = portal_url.unwrap_or(DEFAULT_NOUS_PORTAL_URL).trim_end_matches('/');
        let url = format!("{}/api/oauth/token", base_url);

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()?;

        let mut params = HashMap::new();
        params.insert("client_id", DEFAULT_NOUS_CLIENT_ID);
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", refresh_token);

        let res = client.post(&url).form(&params).send()?;
        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().unwrap_or_default();
            return Err(AuthError::ServerReject {
                error: format!("Refresh HTTP {}", status),
                description: body,
            });
        }

        let token_data: NousTokenResponse = res.json()?;
        Ok(token_data)
    }

    /// Primary auth store path in ~/.sayn/auth.json
    pub fn auth_file_path() -> PathBuf {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        home.join(".sayn/auth.json")
    }

    /// Hermes auth store path in ~/.hermes/auth.json (for upstream compatibility)
    pub fn hermes_auth_file_path() -> PathBuf {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        home.join(".hermes/auth.json")
    }

    /// Save authenticated Nous credentials with restrictive 0600 file permissions.
    pub fn save_auth_state(state: &NousAuthState) -> Result<(), AuthError> {
        let path = Self::auth_file_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(state)?;
        fs::write(&path, json)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
        }

        Ok(())
    }

    /// Load Nous authentication state (reads ~/.sayn/auth.json, fallback ~/.hermes/auth.json).
    pub fn load_auth_state() -> Result<Option<NousAuthState>, AuthError> {
        let sayn_path = Self::auth_file_path();
        if sayn_path.exists() {
            let content = fs::read_to_string(&sayn_path)?;
            if let Ok(state) = serde_json::from_str::<NousAuthState>(&content) {
                return Ok(Some(state));
            }
        }

        // Upstream fallback: check ~/.hermes/auth.json
        let hermes_path = Self::hermes_auth_file_path();
        if hermes_path.exists() {
            if let Ok(content) = fs::read_to_string(&hermes_path) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(nous) = val.get("providers").and_then(|p| p.get("nous")) {
                        if let Some(access_token) = nous.get("access_token").and_then(|t| t.as_str()) {
                            let portal = nous
                                .get("portal_base_url")
                                .and_then(|p| p.as_str())
                                .unwrap_or(DEFAULT_NOUS_PORTAL_URL);
                            let inference = nous
                                .get("inference_base_url")
                                .and_then(|p| p.as_str())
                                .unwrap_or(DEFAULT_NOUS_INFERENCE_URL);
                            let refresh = nous
                                .get("refresh_token")
                                .and_then(|r| r.as_str())
                                .map(|s| s.to_string());
                            let expires_in = nous.get("expires_in").and_then(|e| e.as_u64()).unwrap_or(3600);
                            let now = chrono::Utc::now();
                            let exp = now + chrono::Duration::seconds(expires_in as i64);

                            return Ok(Some(NousAuthState {
                                portal_base_url: portal.to_string(),
                                inference_base_url: inference.to_string(),
                                client_id: DEFAULT_NOUS_CLIENT_ID.to_string(),
                                scope: DEFAULT_NOUS_SCOPE.to_string(),
                                token_type: "Bearer".to_string(),
                                access_token: access_token.to_string(),
                                refresh_token: refresh,
                                obtained_at: now.to_rfc3339(),
                                expires_at: exp.to_rfc3339(),
                                expires_in,
                                free_tier: false,
                            }));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Clear stored Nous authentication credentials.
    pub fn clear_auth_state() -> Result<(), AuthError> {
        let sayn_path = Self::auth_file_path();
        if sayn_path.exists() {
            fs::remove_file(sayn_path)?;
        }
        Ok(())
    }

    /// Ensure active access token is valid (automatically refreshes if needed).
    pub fn get_valid_access_token() -> Result<Option<String>, AuthError> {
        if let Some(mut state) = Self::load_auth_state()? {
            if state.is_expired() {
                if let Some(ref refresh) = state.refresh_token {
                    if let Ok(new_token) = Self::refresh_token(Some(&state.portal_base_url), refresh) {
                        state.access_token = new_token.access_token;
                        if let Some(r) = new_token.refresh_token {
                            state.refresh_token = Some(r);
                        }
                        let now = chrono::Utc::now();
                        state.obtained_at = now.to_rfc3339();
                        let exp_in = new_token.expires_in.unwrap_or(3600);
                        state.expires_in = exp_in;
                        state.expires_at = (now + chrono::Duration::seconds(exp_in as i64)).to_rfc3339();
                        let _ = Self::save_auth_state(&state);
                        return Ok(Some(state.access_token));
                    }
                }
            }
            Ok(Some(state.access_token))
        } else {
            Ok(None)
        }
    }

    /// Open web browser for user verification.
    pub fn open_browser(url: &str) -> bool {
        #[cfg(target_os = "linux")]
        {
            Command::new("xdg-open").arg(url).spawn().is_ok()
        }
        #[cfg(target_os = "macos")]
        {
            Command::new("open").arg(url).spawn().is_ok()
        }
        #[cfg(target_os = "windows")]
        {
            Command::new("cmd").args(["/c", "start", url]).spawn().is_ok()
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_state_expiry_detection() {
        let past = (chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        let state = NousAuthState {
            portal_base_url: DEFAULT_NOUS_PORTAL_URL.into(),
            inference_base_url: DEFAULT_NOUS_INFERENCE_URL.into(),
            client_id: DEFAULT_NOUS_CLIENT_ID.into(),
            scope: DEFAULT_NOUS_SCOPE.into(),
            token_type: "Bearer".into(),
            access_token: "test_token".into(),
            refresh_token: None,
            obtained_at: past.clone(),
            expires_at: past,
            expires_in: 3600,
            free_tier: false,
        };
        assert!(state.is_expired());

        let future = (chrono::Utc::now() + chrono::Duration::hours(2)).to_rfc3339();
        let state2 = NousAuthState {
            expires_at: future,
            ..state
        };
        assert!(!state2.is_expired());
    }
}
