use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct DiagnosticIssue {
    pub file: String,
    pub description: String,
    pub severity: String,
}

pub struct BoundedRepair;

impl BoundedRepair {
    /// Inspect the generated output directory for structural or compilation issues.
    pub fn inspect_workspace(gen_root: &Path) -> Vec<DiagnosticIssue> {
        let mut issues = Vec::new();

        // 1. Check package.json
        let pkg_path = gen_root.join("package.json");
        if !pkg_path.exists() {
            issues.push(DiagnosticIssue {
                file: "package.json".into(),
                description: "Missing package.json in generated workspace".into(),
                severity: "error".into(),
            });
        } else if let Ok(content) = fs::read_to_string(&pkg_path) {
            if serde_json::from_str::<serde_json::Value>(&content).is_err() {
                issues.push(DiagnosticIssue {
                    file: "package.json".into(),
                    description: "Malformed package.json syntax".into(),
                    severity: "error".into(),
                });
            }
        }

        // 2. Check layout
        let layout_path = gen_root.join("app/layout.tsx");
        if !layout_path.exists() {
            issues.push(DiagnosticIssue {
                file: "app/layout.tsx".into(),
                description: "Missing root layout (app/layout.tsx)".into(),
                severity: "error".into(),
            });
        }

        // 3. Check page
        let page_path = gen_root.join("app/page.tsx");
        if !page_path.exists() {
            issues.push(DiagnosticIssue {
                file: "app/page.tsx".into(),
                description: "Missing root entry page (app/page.tsx)".into(),
                severity: "error".into(),
            });
        }

        // 4. Check Tailwind CSS globals
        let css_path = gen_root.join("app/globals.css");
        if !css_path.exists() {
            issues.push(DiagnosticIssue {
                file: "app/globals.css".into(),
                description: "Missing app/globals.css stylesheet".into(),
                severity: "error".into(),
            });
        } else if let Ok(content) = fs::read_to_string(&css_path) {
            if !content.contains("tailwindcss") {
                issues.push(DiagnosticIssue {
                    file: "app/globals.css".into(),
                    description: "globals.css is missing '@import \"tailwindcss\";'".into(),
                    severity: "warning".into(),
                });
            }
        }

        issues
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn inspection_reports_missing_artifacts_without_creating_them() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("package.json"), r#"{"name": "test"}"#).unwrap();

        let issues = BoundedRepair::inspect_workspace(root);

        assert!(issues.iter().any(|issue| issue.file == "app/layout.tsx"));
        assert!(issues.iter().any(|issue| issue.file == "app/page.tsx"));
        assert!(!root.join("app").exists());
    }
}
