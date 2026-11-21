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
    ServerReject { error: String, description: String },

    #[error("Invalid credentials state: {0}")]
    InvalidState(String),

    #[error("Nous Portal authentication must be renewed: {0}")]
    ReauthenticationRequired(String),
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
    fn rejected(status: reqwest::StatusCode, body: &str) -> AuthError {
        let provider_error = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(|item| item.as_str())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| "request_rejected".into());
        AuthError::ServerReject {
            error: format!("HTTP {status}"),
            description: format!("provider error: {provider_error}"),
        }
    }

    fn validate_token_response(
        response: NousTokenResponse,
    ) -> Result<NousTokenResponse, AuthError> {
        if response.access_token.trim().is_empty() {
            return Err(AuthError::InvalidState(
                "provider returned an empty access token".into(),
            ));
        }
        Ok(response)
    }

    /// Request a new device code from Nous Portal.
    pub fn request_device_code(portal_url: Option<&str>) -> Result<DeviceCodeResponse, AuthError> {
        let base_url = portal_url
            .unwrap_or(DEFAULT_NOUS_PORTAL_URL)
            .trim_end_matches('/');
        let url = format!("{}/api/oauth/device/code", base_url);

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()?;

        let mut params = HashMap::new();
        params.insert("client_id", DEFAULT_NOUS_CLIENT_ID);
        params.insert("scope", DEFAULT_NOUS_SCOPE);

        let res = client.post(&url).form(&params).send()?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().unwrap_or_default();
            return Err(Self::rejected(status, &body));
        }

        let device_data: DeviceCodeResponse = res.json()?;
        if device_data.device_code.trim().is_empty()
            || device_data.user_code.trim().is_empty()
            || device_data.verification_uri.trim().is_empty()
        {
            return Err(AuthError::InvalidState(
                "provider returned an incomplete device authorization response".into(),
            ));
        }
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
        let base_url = portal_url
            .unwrap_or(DEFAULT_NOUS_PORTAL_URL)
            .trim_end_matches('/');
        let url = format!("{}/api/oauth/token", base_url);

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()?;

        let effective_interval = interval.clamp(1, 30);
        let start = Instant::now();
        let timeout_dur = Duration::from_secs(expires_in.max(30));

        loop {
            let elapsed = start.elapsed();
            if elapsed >= timeout_dur {
                return Err(AuthError::Timeout(
                    "Timed out waiting for device authorization. Please retry 'sayn auth nous'."
                        .to_string(),
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
                return Self::validate_token_response(token_data);
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
        let base_url = portal_url
            .unwrap_or(DEFAULT_NOUS_PORTAL_URL)
            .trim_end_matches('/');
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
            return Err(Self::rejected(status, &body));
        }

        let token_data: NousTokenResponse = res.json()?;
        Self::validate_token_response(token_data)
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
        Self::save_auth_state_to(&Self::auth_file_path(), state)
    }

    pub fn save_auth_state_to(path: &PathBuf, state: &NousAuthState) -> Result<(), AuthError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(state)?;
        let temporary = path.with_extension(
            path.extension()
                .and_then(|extension| extension.to_str())
                .map(|extension| format!("{extension}.tmp"))
                .unwrap_or_else(|| "tmp".into()),
        );
        fs::write(&temporary, json)?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))?;
        }

        fs::rename(&temporary, path)?;

        Ok(())
    }

    /// Load Nous authentication state (reads ~/.sayn/auth.json, fallback ~/.hermes/auth.json).
    pub fn load_auth_state() -> Result<Option<NousAuthState>, AuthError> {
        let sayn_path = Self::auth_file_path();
        let hermes_path = Self::hermes_auth_file_path();
        Self::load_auth_state_from(&sayn_path, &hermes_path)
    }

    pub fn load_auth_state_from(
        sayn_path: &PathBuf,
        hermes_path: &PathBuf,
    ) -> Result<Option<NousAuthState>, AuthError> {
        if sayn_path.exists() {
            let content = fs::read_to_string(sayn_path)?;
            let state = serde_json::from_str::<NousAuthState>(&content)
                .map_err(|error| AuthError::InvalidState(error.to_string()))?;
            return Ok(Some(state));
        }

        // Upstream fallback: check ~/.hermes/auth.json
        if hermes_path.exists()
            && let Ok(content) = fs::read_to_string(hermes_path)
            && let Ok(val) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(nous) = val.get("providers").and_then(|p| p.get("nous"))
            && let Some(access_token) = nous.get("access_token").and_then(|t| t.as_str())
        {
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
            let expires_in = nous
                .get("expires_in")
                .and_then(|e| e.as_u64())
                .unwrap_or(3600);
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
        match Self::valid_access_token() {
            Ok(token) => Ok(Some(token)),
            Err(AuthError::ReauthenticationRequired(message)) if message == "not authenticated" => {
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    pub fn valid_access_token() -> Result<String, AuthError> {
        Self::valid_access_token_from(&Self::auth_file_path(), &Self::hermes_auth_file_path())
    }

    pub fn valid_access_token_from(
        sayn_path: &PathBuf,
        hermes_path: &PathBuf,
    ) -> Result<String, AuthError> {
        let mut state = Self::load_auth_state_from(sayn_path, hermes_path)?
            .ok_or_else(|| AuthError::ReauthenticationRequired("not authenticated".into()))?;
        if !state.is_expired() {
            return Ok(state.access_token);
        }

        let refresh = state.refresh_token.clone().ok_or_else(|| {
            AuthError::ReauthenticationRequired("the access token expired".into())
        })?;
        let new_token = Self::refresh_token(Some(&state.portal_base_url), &refresh)
            .map_err(|_| AuthError::ReauthenticationRequired("refresh was rejected".into()))?;
        state.access_token = new_token.access_token;
        if let Some(rotated_refresh) = new_token.refresh_token {
            state.refresh_token = Some(rotated_refresh);
        }
        let now = chrono::Utc::now();
        let expires_in = new_token.expires_in.unwrap_or(3600);
        state.obtained_at = now.to_rfc3339();
        state.expires_in = expires_in;
        state.expires_at = (now + chrono::Duration::seconds(expires_in as i64)).to_rfc3339();
        Self::save_auth_state_to(sayn_path, &state)?;
        Ok(state.access_token)
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
            Command::new("cmd")
                .args(["/c", "start", url])
                .spawn()
                .is_ok()
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
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use tempfile::tempdir;

    fn serve(responses: Vec<(&'static str, &'static str)>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let handle = thread::spawn(move || {
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0_u8; 4096];
                let _ = stream.read(&mut request);
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
            }
        });
        (format!("http://{address}"), handle)
    }

    fn expired_state(portal_base_url: String) -> NousAuthState {
        let past = (chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
        NousAuthState {
            portal_base_url,
            inference_base_url: DEFAULT_NOUS_INFERENCE_URL.into(),
            client_id: DEFAULT_NOUS_CLIENT_ID.into(),
            scope: DEFAULT_NOUS_SCOPE.into(),
            token_type: "Bearer".into(),
            access_token: "expired-access".into(),
            refresh_token: Some("old-refresh".into()),
            obtained_at: past.clone(),
            expires_at: past,
            expires_in: 3600,
            free_tier: false,
        }
    }

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

    #[test]
    fn device_authorization_uses_the_official_response_contract() {
        let body = r#"{"device_code":"device-secret","user_code":"ABCD-EFGH","verification_uri":"https://portal.example/device","verification_uri_complete":"https://portal.example/device?user_code=ABCD-EFGH","expires_in":300,"interval":5}"#;
        let (portal, server) = serve(vec![("200 OK", body)]);

        let response = NousAuthManager::request_device_code(Some(&portal)).unwrap();
        server.join().unwrap();

        assert_eq!(response.user_code, "ABCD-EFGH");
        assert_eq!(response.expires_in, 300);
        assert_eq!(response.interval, 5);
    }

    #[test]
    fn polling_retries_pending_authorization_before_returning_tokens() {
        let pending = r#"{"error":"authorization_pending"}"#;
        let token = r#"{"access_token":"approved-access","refresh_token":"approved-refresh","token_type":"Bearer","expires_in":3600,"scope":"inference:invoke"}"#;
        let (portal, server) = serve(vec![("400 Bad Request", pending), ("200 OK", token)]);
        let polls = std::sync::atomic::AtomicUsize::new(0);

        let response =
            NousAuthManager::poll_device_token(Some(&portal), "device-secret", 30, 0, |_| {
                polls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            })
            .unwrap();
        server.join().unwrap();

        assert_eq!(response.access_token, "approved-access");
        assert_eq!(polls.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    #[test]
    fn refresh_rotates_tokens_and_persists_the_new_state() {
        let body = r#"{"access_token":"new-access","refresh_token":"new-refresh","token_type":"Bearer","expires_in":7200,"scope":"inference:invoke"}"#;
        let (portal, server) = serve(vec![("200 OK", body)]);
        let dir = tempdir().unwrap();
        let sayn_path = dir.path().join("sayn-auth.json");
        let hermes_path = dir.path().join("hermes-auth.json");
        NousAuthManager::save_auth_state_to(&sayn_path, &expired_state(portal)).unwrap();

        let token = NousAuthManager::valid_access_token_from(&sayn_path, &hermes_path).unwrap();
        server.join().unwrap();

        assert_eq!(token, "new-access");
        let saved: NousAuthState =
            serde_json::from_str(&fs::read_to_string(sayn_path).unwrap()).unwrap();
        assert_eq!(saved.access_token, "new-access");
        assert_eq!(saved.refresh_token.as_deref(), Some("new-refresh"));
        assert_eq!(saved.expires_in, 7200);
    }

    #[test]
    fn rejected_refresh_never_returns_the_expired_access_token_or_secret_body() {
        let body =
            r#"{"error":"invalid_grant","error_description":"refresh token old-refresh rejected"}"#;
        let (portal, server) = serve(vec![("401 Unauthorized", body)]);
        let dir = tempdir().unwrap();
        let sayn_path = dir.path().join("sayn-auth.json");
        let hermes_path = dir.path().join("hermes-auth.json");
        NousAuthManager::save_auth_state_to(&sayn_path, &expired_state(portal)).unwrap();

        let error = NousAuthManager::valid_access_token_from(&sayn_path, &hermes_path).unwrap_err();
        server.join().unwrap();

        assert!(matches!(error, AuthError::ReauthenticationRequired(_)));
        assert!(!error.to_string().contains("old-refresh"));
        assert!(!error.to_string().contains("expired-access"));
    }

    #[test]
    fn hermes_credentials_are_imported_without_modifying_the_hermes_store() {
        let dir = tempdir().unwrap();
        let sayn_path = dir.path().join("sayn-auth.json");
        let hermes_path = dir.path().join("hermes-auth.json");
        let original = r#"{"providers":{"nous":{"access_token":"hermes-access","refresh_token":"hermes-refresh","expires_in":3600,"portal_base_url":"https://portal.nousresearch.com","inference_base_url":"https://inference-api.nousresearch.com/v1"}}}"#;
        fs::write(&hermes_path, original).unwrap();

        let imported = NousAuthManager::load_auth_state_from(&sayn_path, &hermes_path)
            .unwrap()
            .unwrap();

        assert_eq!(imported.access_token, "hermes-access");
        assert_eq!(fs::read_to_string(&hermes_path).unwrap(), original);
        assert!(!sayn_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn saved_credentials_are_replaced_atomically_with_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let path = dir.path().join("auth.json");
        let state = expired_state(DEFAULT_NOUS_PORTAL_URL.into());

        NousAuthManager::save_auth_state_to(&path, &state).unwrap();

        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(!dir.path().join("auth.json.tmp").exists());
    }
}
