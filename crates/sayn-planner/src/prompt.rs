use sayn_project::ProjectConfig;
use sayn_source::SourceDocument;

pub struct PromptBuilder;

impl PromptBuilder {
    pub fn build_generation_prompt(
        config: &ProjectConfig,
        docs: &[SourceDocument],
    ) -> String {
        let mut prompt = String::new();

        prompt.push_str("# Sayn Compiler Generation Specification\n\n");
        prompt.push_str(&format!("Project Name: {}\n", config.sayn.name));
        prompt.push_str(&format!("Target Framework: {} ({})\n\n", config.sayn.target, config.web.framework));

        prompt.push_str("## Core Invariants\n");
        prompt.push_str("- Generate real, accessible, responsive TypeScript React components for Next.js App Router.\n");
        prompt.push_str("- Never invent personal credentials, API keys, or private accounts.\n");
        prompt.push_str("- Confine all outputs to the generated project directory.\n");
        prompt.push_str("- Respect light/dark mode and responsive design best practices.\n\n");

        prompt.push_str("## Human-Authored Instructions (.sayn)\n\n");
        for doc in docs {
            prompt.push_str(&format!("### Source: {}\n", doc.file_path.display()));
            // Sanitize potential sensitive keys before prompt construction
            let all_text = doc.all_text();
            let sanitized = Self::sanitize_content(&all_text);
            prompt.push_str(&sanitized);
            prompt.push_str("\n\n");
        }

        prompt.push_str("## Desired Generation Output\n");
        prompt.push_str("Generate complete file operations including `package.json`, `tsconfig.json`, `app/layout.tsx`, `app/page.tsx`, and supporting components.\n");

        prompt
    }

    fn sanitize_content(content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut sanitized = Vec::new();

        for line in lines {
            let lower = line.to_lowercase();
            if lower.contains("api_key") || lower.contains("secret_key") || lower.contains("private_key") {
                sanitized.push("[REDACTED: potential secret parameter]");
            } else {
                sanitized.push(line);
            }
        }

        sanitized.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sayn_source::parse_sayn_source;
    use std::path::Path;

    #[test]
    fn test_prompt_construction_and_redaction() {
        use sayn_core::Target;
        let config = ProjectConfig::new("portfolio", Target::Web);
        let doc = parse_sayn_source(
            Path::new("app/page.sayn"),
            "Create portfolio.\nAPI_KEY=sk-1234567890abcdef\nEnsure dark mode works.",
        );

        let prompt = PromptBuilder::build_generation_prompt(&config, &[doc]);
        assert!(prompt.contains("Project Name:"));
        assert!(prompt.contains("[REDACTED: potential secret parameter]"));
        assert!(!prompt.contains("sk-1234567890abcdef"));
    }
}
