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

    /// Attempt bounded repair (up to 3 iterations) for missing critical artifacts.
    pub fn attempt_repair(gen_root: &Path, max_attempts: usize) -> Result<bool, std::io::Error> {
        let mut attempt = 0;
        while attempt < max_attempts {
            attempt += 1;
            let issues = Self::inspect_workspace(gen_root);
            if issues.is_empty() {
                return Ok(true);
            }

            for issue in &issues {
                if issue.file == "app/globals.css" {
                    let path = gen_root.join("app/globals.css");
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(
                        &path,
                        "@import \"tailwindcss\";\n\n:root {\n  color-scheme: light dark;\n}\n",
                    )?;
                } else if issue.file == "app/layout.tsx" {
                    let path = gen_root.join("app/layout.tsx");
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    let layout_code = r#"import type { Metadata } from 'next';
import './globals.css';

export const metadata: Metadata = {
  title: 'Sayn Application',
  description: 'Built with Sayn and Next.js',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="antialiased min-h-screen bg-neutral-50 dark:bg-neutral-950 text-neutral-900 dark:text-neutral-50">
        {children}
      </body>
    </html>
  );
}
"#;
                    fs::write(&path, layout_code)?;
                } else if issue.file == "app/page.tsx" {
                    let path = gen_root.join("app/page.tsx");
                    if let Some(parent) = path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    let page_code = r#"export default function HomePage() {
  return (
    <main className="container mx-auto px-4 py-16">
      <h1 className="text-4xl font-bold tracking-tight">Sayn Application</h1>
      <p className="mt-4 text-neutral-600 dark:text-neutral-400">
        Engineered with Sayn instruction compiler.
      </p>
    </main>
  );
}
"#;
                    fs::write(&path, page_code)?;
                }
            }
        }

        // Final check
        let remaining = Self::inspect_workspace(gen_root);
        Ok(remaining.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_inspect_and_repair() {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Initially empty: issues exist
        let issues = BoundedRepair::inspect_workspace(root);
        assert!(!issues.is_empty());

        // Create package.json
        fs::write(root.join("package.json"), r#"{"name": "test"}"#).unwrap();

        // Run repair
        let fixed = BoundedRepair::attempt_repair(root, 3).unwrap();
        assert!(fixed);

        // Check that artifacts now exist
        assert!(root.join("app/layout.tsx").exists());
        assert!(root.join("app/page.tsx").exists());
        assert!(root.join("app/globals.css").exists());
    }
}
