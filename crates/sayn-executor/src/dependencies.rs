use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Default bound on one package-manager command.
pub const TOOL_TIMEOUT: Duration = Duration::from_secs(15 * 60);

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

    /// Resolve the package manager for a project preference (`"pnpm"`,
    /// `"npm"`, or `"auto"`/empty). An explicit preference is never silently
    /// replaced by a different tool.
    pub fn resolve_package_manager(preference: &str) -> Option<&'static str> {
        match preference.trim().to_lowercase().as_str() {
            "pnpm" => Self::is_pnpm_installed().then_some("pnpm"),
            "npm" => Self::is_npm_installed().then_some("npm"),
            _ => Self::detect_package_manager(),
        }
    }

    /// Run `<package manager> <args...>` in `dir`, bounded by `timeout`.
    ///
    /// The executable and arguments are passed as an argument array, never
    /// through a shell. Returns `Err` describing the exact exit status, a
    /// timeout, or a failure to start; a nonzero exit is never success.
    pub fn run_bounded(
        package_manager: &str,
        args: &[&str],
        dir: &Path,
        timeout: Duration,
    ) -> Result<(), String> {
        let label = format!("{package_manager} {}", args.join(" "));
        let mut child = Command::new(package_manager)
            .args(args)
            .current_dir(dir)
            .env("COREPACK_ENABLE_DOWNLOAD_PROMPT", "0")
            .env("CI", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|error| format!("'{label}' could not be started: {error}"))?;

        let started = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => return Ok(()),
                Ok(Some(status)) => {
                    return Err(match status.code() {
                        Some(code) => format!("'{label}' exited with code {code}"),
                        None => format!("'{label}' was terminated by a signal"),
                    });
                }
                Ok(None) if started.elapsed() >= timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "'{label}' did not finish within {} seconds and was stopped",
                        timeout.as_secs()
                    ));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(100)),
                Err(error) => return Err(format!("'{label}' could not be monitored: {error}")),
            }
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
    pub fn run_script(
        gen_root: &Path,
        script: &str,
        extra_args: &[String],
    ) -> std::io::Result<bool> {
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
    pub fn list_scripts(
        gen_root: &Path,
    ) -> Result<Vec<(String, String)>, Box<dyn std::error::Error>> {
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

    #[cfg(unix)]
    #[test]
    fn run_bounded_reports_the_exact_exit_code() {
        let dir = std::env::temp_dir();
        assert!(DependencyManager::run_bounded("true", &[], &dir, TOOL_TIMEOUT).is_ok());

        let error = DependencyManager::run_bounded("sh", &["-c", "exit 3"], &dir, TOOL_TIMEOUT)
            .unwrap_err();
        assert!(error.contains("exited with code 3"), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn run_bounded_stops_a_command_that_exceeds_its_timeout() {
        let dir = std::env::temp_dir();
        let started = Instant::now();

        let error =
            DependencyManager::run_bounded("sleep", &["30"], &dir, Duration::from_millis(300))
                .unwrap_err();

        assert!(error.contains("did not finish"), "{error}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn run_bounded_reports_a_missing_executable() {
        let error = DependencyManager::run_bounded(
            "sayn-no-such-tool",
            &["install"],
            &std::env::temp_dir(),
            TOOL_TIMEOUT,
        )
        .unwrap_err();
        assert!(error.contains("could not be started"), "{error}");
    }

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
