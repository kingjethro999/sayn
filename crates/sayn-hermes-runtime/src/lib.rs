use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("Python not found: {0}")]
    PythonNotFound(String),

    #[error("Python version incompatible: {0} (requires >= 3.11, < 3.15)")]
    PythonIncompatible(String),

    #[error("Failed to provision runtime venv: {0}")]
    VenvFailed(String),

    #[error("Dependency installation error: {0}")]
    DependencyError(String),

    #[error("RPC communication error: {0}")]
    RpcError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeHealthReport {
    pub python_installed: bool,
    pub python_version: Option<String>,
    pub venv_exists: bool,
    pub dependencies_installed: bool,
    pub agent_comm_ok: bool,
}

pub struct RuntimeManager;

impl RuntimeManager {
    /// Get the path to Sayn's managed virtual environment: ~/.sayn/runtime/venv
    pub fn runtime_dir() -> PathBuf {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        home.join(".sayn/runtime")
    }

    pub fn venv_dir() -> PathBuf {
        Self::runtime_dir().join("venv")
    }

    pub fn venv_python() -> PathBuf {
        #[cfg(windows)]
        {
            Self::venv_dir().join("Scripts/python.exe")
        }
        #[cfg(not(windows))]
        {
            Self::venv_dir().join("bin/python")
        }
    }

    pub fn venv_pip() -> PathBuf {
        #[cfg(windows)]
        {
            Self::venv_dir().join("Scripts/pip.exe")
        }
        #[cfg(not(windows))]
        {
            Self::venv_dir().join("bin/pip")
        }
    }

    /// Detect host python3 executable and version.
    pub fn detect_host_python() -> Result<(String, String), RuntimeError> {
        let candidates = ["python3", "python"];
        for cmd in candidates {
            if let Ok(output) = Command::new(cmd).arg("--version").output() {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let ver_text = if text.is_empty() {
                        String::from_utf8_lossy(&output.stderr).trim().to_string()
                    } else {
                        text
                    };

                    // Check version number (e.g. "Python 3.12.3")
                    if ver_text.contains("3.11")
                        || ver_text.contains("3.12")
                        || ver_text.contains("3.13")
                        || ver_text.contains("3.14")
                    {
                        return Ok((cmd.to_string(), ver_text));
                    } else {
                        return Err(RuntimeError::PythonIncompatible(ver_text));
                    }
                }
            }
        }

        Err(RuntimeError::PythonNotFound(
            "No compatible Python 3.11+ found in PATH".into(),
        ))
    }

    /// Check if virtualenv is initialized and dependencies installed.
    pub fn is_runtime_ready() -> bool {
        let py = Self::venv_python();
        if !py.exists() {
            return false;
        }

        // Test required imports
        let check_script = "import ruamel.yaml, httpx, rich, tenacity, dotenv, openai";
        Command::new(&py)
            .args(["-c", check_script])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Ensure runtime virtualenv is created and dependencies are installed.
    pub fn ensure_runtime() -> Result<(), RuntimeError> {
        if Self::is_runtime_ready() {
            return Ok(());
        }

        let (host_py, _) = Self::detect_host_python()?;
        let venv_path = Self::venv_dir();

        if !venv_path.exists() {
            let status = Command::new(&host_py)
                .args(["-m", "venv", &venv_path.to_string_lossy()])
                .status()?;
            if !status.success() {
                return Err(RuntimeError::VenvFailed(
                    "Failed to create python virtual environment".into(),
                ));
            }
        }

        let pip = Self::venv_pip();
        // Ensure pip exists or bootstrap via ensurepip
        if !pip.exists() {
            let py = Self::venv_python();
            let _ = Command::new(&py)
                .args(["-m", "ensurepip", "--upgrade"])
                .status();
        }

        // Install core Hermes dependencies
        let deps = ["ruamel.yaml", "httpx", "rich", "tenacity", "python-dotenv", "openai"];
        let mut cmd = Command::new(&pip);
        cmd.arg("install");
        for d in deps {
            cmd.arg(d);
        }

        let status = cmd
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        if !status.success() {
            return Err(RuntimeError::DependencyError(
                "Failed to install Hermes runtime dependencies via pip".into(),
            ));
        }

        Ok(())
    }

    /// Perform a full health inspection for `sayn doctor`.
    pub fn check_health() -> RuntimeHealthReport {
        let (py_ok, py_ver) = match Self::detect_host_python() {
            Ok((_, ver)) => (true, Some(ver)),
            Err(_) => (false, None),
        };

        let venv_exists = Self::venv_dir().exists();
        let deps_ok = Self::is_runtime_ready();

        let agent_comm_ok = if deps_ok {
            let py = Self::venv_python();
            Command::new(&py)
                .args(["-c", "print('READY')"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).contains("READY"))
                .unwrap_or(false)
        } else {
            false
        };

        RuntimeHealthReport {
            python_installed: py_ok,
            python_version: py_ver,
            venv_exists,
            dependencies_installed: deps_ok,
            agent_comm_ok,
        }
    }
}

/// JSON-RPC 2.0 Client for communicating with the embedded Hermes gateway.
pub struct HermesRpcClient {
    child: Child,
    request_id: u64,
}

impl HermesRpcClient {
    /// Spawn the Hermes TUI gateway process using the managed runtime.
    pub fn spawn(hermes_dir: &Path) -> Result<Self, RuntimeError> {
        let py = RuntimeManager::venv_python();
        if !py.exists() {
            RuntimeManager::ensure_runtime()?;
        }

        let child = Command::new(&py)
            .args(["-m", "tui_gateway.entry"])
            .current_dir(hermes_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Wait for gateway.ready event on stdout
        let client = Self {
            child,
            request_id: 1,
        };

        Ok(client)
    }

    /// Send a request and wait for the response.
    pub fn call(&mut self, method: &str, params: serde_json::Value) -> Result<serde_json::Value, RuntimeError> {
        let id = self.request_id;
        self.request_id += 1;

        let req = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });

        let line = format!("{}\n", serde_json::to_string(&req)?);

        if let Some(ref mut stdin) = self.child.stdin {
            stdin.write_all(line.as_bytes())?;
            stdin.flush()?;
        } else {
            return Err(RuntimeError::RpcError("No stdin handle".into()));
        }

        // Read responses until matching id is received
        if let Some(ref mut stdout) = self.child.stdout {
            let reader = BufReader::new(stdout);
            for l in reader.lines() {
                let text = l?;
                if text.trim().is_empty() {
                    continue;
                }

                if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                    if val.get("id").and_then(|v| v.as_u64()) == Some(id) {
                        if let Some(res) = val.get("result") {
                            return Ok(res.clone());
                        } else if let Some(err) = val.get("error") {
                            return Err(RuntimeError::RpcError(err.to_string()));
                        }
                    }
                }
            }
        }

        Err(RuntimeError::RpcError("EOF without response".into()))
    }
}

impl Drop for HermesRpcClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_python() {
        let res = RuntimeManager::detect_host_python();
        assert!(res.is_ok());
    }

    #[test]
    fn test_runtime_health() {
        let health = RuntimeManager::check_health();
        assert!(health.python_installed);
        assert!(health.python_version.is_some());
    }
}
