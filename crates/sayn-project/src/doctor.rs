use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticCheck {
    pub name: String,
    pub passed: bool,
    pub details: String,
    pub recommendation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub checks: Vec<DiagnosticCheck>,
    pub all_passed: bool,
}

pub fn run_diagnostics() -> DoctorReport {
    let mut checks = Vec::new();

    // 1. Check Node.js
    match Command::new("node").arg("--version").output() {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            checks.push(DiagnosticCheck {
                name: "Node.js".to_string(),
                passed: true,
                details: format!("Found {}", ver),
                recommendation: None,
            });
        }
        _ => {
            checks.push(DiagnosticCheck {
                name: "Node.js".to_string(),
                passed: false,
                details: "Not found in PATH".to_string(),
                recommendation: Some("Install Node.js 18+ (https://nodejs.org)".to_string()),
            });
        }
    }

    // 2. Check Package Manager (npm, pnpm, yarn, or bun)
    let pkg_managers = ["npm", "pnpm", "yarn", "bun"];
    let mut found_pkg = None;
    for pm in pkg_managers {
        if let Some(out) = Command::new(pm)
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.success())
        {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            found_pkg = Some((pm, ver));
            break;
        }
    }

    if let Some((pm, ver)) = found_pkg {
        checks.push(DiagnosticCheck {
            name: "Package Manager".to_string(),
            passed: true,
            details: format!("Found {} {}", pm, ver),
            recommendation: None,
        });
    } else {
        checks.push(DiagnosticCheck {
            name: "Package Manager".to_string(),
            passed: false,
            details: "No package manager (npm, pnpm, yarn, bun) detected".to_string(),
            recommendation: Some("Install npm, pnpm, or yarn".to_string()),
        });
    }

    // 3. Check Git
    match Command::new("git").arg("--version").output() {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            checks.push(DiagnosticCheck {
                name: "Git".to_string(),
                passed: true,
                details: format!("Found {}", ver),
                recommendation: None,
            });
        }
        _ => {
            checks.push(DiagnosticCheck {
                name: "Git".to_string(),
                passed: false,
                details: "Not found in PATH".to_string(),
                recommendation: Some("Install git version control".to_string()),
            });
        }
    }

    // 4. Check Rust toolchain
    match Command::new("rustc").arg("--version").output() {
        Ok(out) if out.status.success() => {
            let ver = String::from_utf8_lossy(&out.stdout).trim().to_string();
            checks.push(DiagnosticCheck {
                name: "Rust Toolchain".to_string(),
                passed: true,
                details: format!("Found {}", ver),
                recommendation: None,
            });
        }
        _ => {
            checks.push(DiagnosticCheck {
                name: "Rust Toolchain".to_string(),
                passed: true,
                details: "Optional for project runtime, present for compiler development"
                    .to_string(),
                recommendation: None,
            });
        }
    }

    let all_passed = checks.iter().all(|c| c.passed);

    DoctorReport { checks, all_passed }
}
