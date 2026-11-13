//! Versioned structured generation response.
//!
//! A model's reply is an untrusted proposal. This module turns the raw reply
//! text into a typed, structurally validated list of operations, or refuses it.
//! Nothing here touches the filesystem: confinement against the real output
//! root still happens in the executor before any write.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Component, Path};
use thiserror::Error;

/// The only generation response schema version this engine accepts.
pub const GENERATION_SCHEMA_VERSION: u32 = 1;

/// Upper bound on operations in one response.
pub const MAX_GENERATED_OPERATIONS: usize = 500;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ResponseSchemaError {
    #[error("the response is not a JSON object matching the generation schema: {0}")]
    Malformed(String),

    #[error("unsupported schema_version {found}; this engine accepts version {expected}")]
    UnsupportedVersion { found: u32, expected: u32 },

    #[error("the response contained no file operations")]
    EmptyOperations,

    #[error("the response contained {0} operations, above the limit of {MAX_GENERATED_OPERATIONS}")]
    TooManyOperations(usize),

    #[error("operation path '{0}' is not a relative path inside the generated project")]
    UnsafePath(String),

    #[error("more than one operation targets '{0}'")]
    DuplicateTarget(String),
}

/// One file-level operation proposed by a model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum GeneratedOperation {
    Create { path: String, content: String },
    Modify { path: String, content: String },
    Delete { path: String },
    Retain { path: String },
}

impl GeneratedOperation {
    pub fn path(&self) -> &str {
        match self {
            Self::Create { path, .. }
            | Self::Modify { path, .. }
            | Self::Delete { path }
            | Self::Retain { path } => path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationResponse {
    pub schema_version: u32,
    pub operations: Vec<GeneratedOperation>,
    #[serde(default)]
    pub acceptance_checks: Vec<String>,
    #[serde(default)]
    pub summary: String,
}

impl GenerationResponse {
    /// Parse and validate assistant text. Accepts a bare JSON object or one
    /// wrapped in a Markdown code fence or surrounding prose.
    pub fn parse(text: &str) -> Result<Self, ResponseSchemaError> {
        let trimmed = text.trim();
        let response: Self = match serde_json::from_str(trimmed) {
            Ok(response) => response,
            Err(direct_error) => {
                let embedded = embedded_object(trimmed)
                    .ok_or_else(|| ResponseSchemaError::Malformed(direct_error.to_string()))?;
                serde_json::from_str(embedded)
                    .map_err(|error| ResponseSchemaError::Malformed(error.to_string()))?
            }
        };
        response.validate()?;
        Ok(response)
    }

    /// Structural validation that does not depend on the local filesystem.
    pub fn validate(&self) -> Result<(), ResponseSchemaError> {
        if self.schema_version != GENERATION_SCHEMA_VERSION {
            return Err(ResponseSchemaError::UnsupportedVersion {
                found: self.schema_version,
                expected: GENERATION_SCHEMA_VERSION,
            });
        }
        if self.operations.is_empty() {
            return Err(ResponseSchemaError::EmptyOperations);
        }
        if self.operations.len() > MAX_GENERATED_OPERATIONS {
            return Err(ResponseSchemaError::TooManyOperations(
                self.operations.len(),
            ));
        }

        let mut seen = HashSet::new();
        for operation in &self.operations {
            let path = operation.path();
            let normalized = normalize_relative(path)
                .ok_or_else(|| ResponseSchemaError::UnsafePath(path.to_string()))?;
            // Lowercased so two spellings cannot collide on case-insensitive filesystems.
            if !seen.insert(normalized.to_lowercase()) {
                return Err(ResponseSchemaError::DuplicateTarget(path.to_string()));
            }
        }
        Ok(())
    }
}

/// The outermost `{ ... }` span of the text, if any.
fn embedded_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    (end > start).then(|| &text[start..=end])
}

/// Normalize a model-supplied path to `a/b/c`, or `None` if it is empty,
/// absolute, drive-prefixed, or contains a parent-directory component.
fn normalize_relative(path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed.contains('\0') {
        return None;
    }
    let unified = trimmed.replace('\\', "/");
    if unified.starts_with('/') || unified.split('/').next()?.contains(':') {
        return None;
    }

    let mut parts = Vec::new();
    for component in Path::new(&unified).components() {
        match component {
            Component::Normal(part) => parts.push(part.to_str()?.to_string()),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"{"schema_version":1,"operations":[{"kind":"create","path":"app/page.tsx","content":"x"}],"acceptance_checks":["build"],"summary":"ok"}"#;

    #[test]
    fn parses_a_bare_object() {
        let response = GenerationResponse::parse(VALID).unwrap();
        assert_eq!(response.operations.len(), 1);
        assert_eq!(response.acceptance_checks, vec!["build"]);
    }

    #[test]
    fn parses_an_object_inside_a_code_fence() {
        let fenced = format!("Here you go:\n```json\n{VALID}\n```\n");
        assert!(GenerationResponse::parse(&fenced).is_ok());
    }

    #[test]
    fn rejects_text_that_is_not_json() {
        assert!(matches!(
            GenerationResponse::parse("I cannot help with that."),
            Err(ResponseSchemaError::Malformed(_))
        ));
    }

    #[test]
    fn rejects_unknown_operation_kinds() {
        let text = r#"{"schema_version":1,"operations":[{"kind":"shell","path":"a","content":"rm -rf /"}]}"#;
        assert!(matches!(
            GenerationResponse::parse(text),
            Err(ResponseSchemaError::Malformed(_))
        ));
    }

    #[test]
    fn rejects_unknown_operation_fields() {
        let text =
            r#"{"schema_version":1,"operations":[{"kind":"delete","path":"a","command":"x"}]}"#;
        assert!(matches!(
            GenerationResponse::parse(text),
            Err(ResponseSchemaError::Malformed(_))
        ));
    }

    #[test]
    fn rejects_other_schema_versions() {
        let text = r#"{"schema_version":2,"operations":[{"kind":"retain","path":"a"}]}"#;
        assert_eq!(
            GenerationResponse::parse(text),
            Err(ResponseSchemaError::UnsupportedVersion {
                found: 2,
                expected: 1
            })
        );
    }

    #[test]
    fn rejects_empty_operations() {
        let text = r#"{"schema_version":1,"operations":[]}"#;
        assert_eq!(
            GenerationResponse::parse(text),
            Err(ResponseSchemaError::EmptyOperations)
        );
    }

    #[test]
    fn rejects_escaping_and_absolute_paths() {
        for path in [
            "../outside.txt",
            "app/../../secret",
            "/etc/passwd",
            "C:\\Windows\\x",
            "..\\up",
            "",
        ] {
            let text = serde_json::json!({
                "schema_version": 1,
                "operations": [{"kind": "create", "path": path, "content": ""}]
            })
            .to_string();
            assert!(
                matches!(
                    GenerationResponse::parse(&text),
                    Err(ResponseSchemaError::UnsafePath(_))
                ),
                "path {path:?} should be rejected"
            );
        }
    }

    #[test]
    fn rejects_duplicate_targets_across_spellings() {
        let text = r#"{"schema_version":1,"operations":[{"kind":"create","path":"app/page.tsx","content":"a"},{"kind":"modify","path":"./App/page.tsx","content":"b"}]}"#;
        assert!(matches!(
            GenerationResponse::parse(text),
            Err(ResponseSchemaError::DuplicateTarget(_))
        ));
    }
}
