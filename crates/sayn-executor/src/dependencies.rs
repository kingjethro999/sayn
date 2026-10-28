use std::path::Path;
use std::process::{Command, Stdio};

/// Manages Node.js dependencies through pnpm (preferred) or npm (fallback).
pub struct DependencyManager;

impl DependencyManager {
    pub fn is_node_installed() -> bool {
        Command::new("node")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn is_pnpm_installed() -> bool {
        Command::new("pnpm")
            .arg("--version")
            .env("COREPACK_ENABLE_DOWNLOAD_PROMPT", "0")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub fn is_npm_installed() -> bool {
        Command::new("npm")
            .arg("--version")
            .env("COREPACK_ENABLE_DOWNLOAD_PROMPT", "0")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Returns the best available package manager command ("pnpm" or "npm").
    pub fn detect_package_manager() -> Option<&'static str> {
        if Self::is_pnpm_installed() {
            Some("pnpm")
        } else if Self::is_npm_installed() {
            Some("npm")
        } else {
            None
        }
    }

    /// Install dependencies in the generated project directory.
    /// Uses pnpm by default; falls back to npm.
    pub fn install_dependencies(gen_root: &Path) -> std::io::Result<bool> {
        let pm = match Self::detect_package_manager() {
            Some(pm) => pm,
            None => return Ok(false),
        };

        let status = Command::new(pm)
            .arg("install")
            .current_dir(gen_root)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()?;

        Ok(status.success())
    }

    /// Run a script defined in the generated project's package.json.
    /// E.g. `run_script(gen_root, "dev")` runs `pnpm run dev`.
    pub fn run_script(gen_root: &Path, script: &str, extra_args: &[String]) -> std::io::Result<bool> {
        let pm = match Self::detect_package_manager() {
            Some(pm) => pm,
            None => return Ok(false),
        };

        let mut cmd = Command::new(pm);
        cmd.arg("run").arg(script);

        // For pnpm/npm, pass extra args after "--"
        if !extra_args.is_empty() {
            cmd.arg("--");
            for arg in extra_args {
                cmd.arg(arg);
            }
        }

        cmd.current_dir(gen_root)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        let status = cmd.status()?;
        Ok(status.success())
    }

    /// List scripts from a package.json file.
    pub fn list_scripts(gen_root: &Path) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
        let pkg_path = gen_root.join("package.json");
        if !pkg_path.exists() {
            return Err("No package.json found in generated project".into());
        }

        let content = std::fs::read_to_string(&pkg_path)?;
        let parsed: serde_json::Value = serde_json::from_str(&content)?;

        let mut scripts = Vec::new();
        if let Some(obj) = parsed.get("scripts").and_then(|s| s.as_object()) {
            for (key, val) in obj {
                if let Some(cmd) = val.as_str() {
                    scripts.push((key.clone(), cmd.to_string()));
                }
            }
        }

        Ok(scripts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_detection() {
        let _ = DependencyManager::is_node_installed();
    }

    #[test]
    fn test_package_manager_detection() {
        // Should detect at least one package manager on dev machines
        let pm = DependencyManager::detect_package_manager();
        // This is a soft test; CI may not have pnpm
        if pm.is_some() {
            assert!(pm == Some("pnpm") || pm == Some("npm"));
        }
    }
}
