use sayn_security::confinement::confine_path;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum OperationError {
    #[error("Security violation: path '{0}' attempts escape outside allowed root")]
    PathEscape(String),

    #[error("Invalid package name '{0}': contains unsafe characters")]
    UnsafePackageName(String),

    #[error("File operation error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum FileOperation {
    CreateFile {
        path: String,
        content: String,
    },
    UpdateFile {
        path: String,
        content: String,
    },
    DeleteGeneratedFile {
        path: String,
    },
    InstallDependency {
        package_name: String,
        #[serde(default)]
        dev: bool,
    },
    ExecuteValidation {
        command: String,
    },
    ReportDiagnostic {
        message: String,
        severity: String,
        file: Option<String>,
        line: Option<usize>,
    },
}

pub struct OperationValidator;

impl OperationValidator {
    /// Validates a relative file path against directory escape.
    pub fn validate_path(output_root: &Path, rel_path: &str) -> Result<PathBuf, OperationError> {
        let trimmed = rel_path.trim();
        if trimmed.is_empty() {
            return Err(OperationError::PathEscape(rel_path.to_string()));
        }

        match confine_path(output_root, Path::new(trimmed)) {
            Ok(p) => Ok(p),
            Err(_) => Err(OperationError::PathEscape(rel_path.to_string())),
        }
    }

    /// Validates npm/pnpm package names to prevent arbitrary command injection.
    pub fn validate_package_name(pkg: &str) -> Result<(), OperationError> {
        let trimmed = pkg.trim();
        if trimmed.is_empty() {
            return Err(OperationError::UnsafePackageName(pkg.to_string()));
        }

        // Rejection of shell metacharacters
        for ch in [' ', ';', '&', '|', '`', '$', '(', ')', '<', '>', '\n', '\r'] {
            if trimmed.contains(ch) {
                return Err(OperationError::UnsafePackageName(pkg.to_string()));
            }
        }

        Ok(())
    }

    /// Applies a validated file operation to the target workspace.
    pub fn apply(output_root: &Path, op: &FileOperation) -> Result<Option<String>, OperationError> {
        match op {
            FileOperation::CreateFile { path, content }
            | FileOperation::UpdateFile { path, content } => {
                let target = Self::validate_path(output_root, path)?;
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&target, content)?;
                Ok(Some(format!("Wrote {}", path)))
            }
            FileOperation::DeleteGeneratedFile { path } => {
                let target = Self::validate_path(output_root, path)?;
                if target.exists() {
                    if target.is_dir() {
                        fs::remove_dir_all(&target)?;
                    } else {
                        fs::remove_file(&target)?;
                    }
                    Ok(Some(format!("Deleted {}", path)))
                } else {
                    Ok(None)
                }
            }
            FileOperation::InstallDependency { package_name, dev } => {
                Self::validate_package_name(package_name)?;
                let flag = if *dev { "-D" } else { "" };
                Ok(Some(
                    format!("Requested dependency {} {}", package_name, flag)
                        .trim()
                        .to_string(),
                ))
            }
            FileOperation::ExecuteValidation { command } => {
                Ok(Some(format!("Validation queued: {}", command)))
            }
            FileOperation::ReportDiagnostic {
                message, severity, ..
            } => Ok(Some(format!("[{}] {}", severity.to_uppercase(), message))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_operation_path_confinement() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Valid relative paths
        assert!(OperationValidator::validate_path(root, "app/page.tsx").is_ok());
        assert!(OperationValidator::validate_path(root, "components/Header.tsx").is_ok());

        // Escape attempts
        assert!(OperationValidator::validate_path(root, "../escaping.txt").is_err());
        assert!(OperationValidator::validate_path(root, "/etc/passwd").is_err());
        assert!(OperationValidator::validate_path(root, "app/../../secret.key").is_err());
    }

    #[test]
    fn test_validate_package_name() {
        assert!(OperationValidator::validate_package_name("lucide-react").is_ok());
        assert!(OperationValidator::validate_package_name("@tailwindcss/postcss").is_ok());

        // Malicious names
        assert!(OperationValidator::validate_package_name("lucide-react; rm -rf /").is_err());
        assert!(OperationValidator::validate_package_name("pkg | sh").is_err());
        assert!(OperationValidator::validate_package_name("pkg`id`").is_err());
    }

    #[test]
    fn test_apply_create_file() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        let op = FileOperation::CreateFile {
            path: "test/hello.txt".into(),
            content: "Hello Sayn".into(),
        };

        let res = OperationValidator::apply(root, &op);
        assert!(res.is_ok());
        assert_eq!(
            fs::read_to_string(root.join("test/hello.txt")).unwrap(),
            "Hello Sayn"
        );
    }
}
