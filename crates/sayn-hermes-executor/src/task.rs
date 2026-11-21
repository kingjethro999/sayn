use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInstruction {
    pub file_path: String,
    pub raw_content: String,
    pub title: Option<String>,
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationTask {
    pub project_id: String,
    pub run_id: String,
    pub project_root: PathBuf,
    pub output_root: PathBuf,
    pub target: String,
    pub framework: String,
    pub model: String,
    pub provider: String,
    pub instructions: Vec<TaskInstruction>,
    pub routes: Vec<String>,
    pub components: Vec<String>,
    pub dependencies: Vec<String>,
    pub styling_guidelines: Vec<String>,
    pub constraints: Vec<String>,
    pub existing_files: Vec<String>,
    pub source_hashes: HashMap<String, String>,
}

impl GenerationTask {
    /// Render a comprehensive system and user prompt for Hermes Agent reasoning.
    pub fn format_prompt(&self) -> String {
        let mut prompt = String::new();
        prompt.push_str("# Sayn Compiler & Hermes Agent Task\n\n");
        prompt.push_str(&format!("Project ID: {}\n", self.project_id));
        prompt.push_str(&format!("Run ID: {}\n", self.run_id));
        prompt.push_str(&format!(
            "Target Framework: {} ({})\n\n",
            self.target, self.framework
        ));

        prompt.push_str("## Core Mandates\n");
        prompt.push_str("- Generate a production-grade, functional Next.js 15 application using React 19 and Tailwind CSS v4.\n");
        prompt.push_str("- The output directory is '.sayn/generated'. Never attempt path traversal or write outside this boundary.\n");
        prompt.push_str("- Implement modern aesthetic standards: clean typography (Inter / Geist), smooth transitions, responsive layouts, accessible semantic HTML.\n");
        prompt.push_str("- Output must be represented as structured file operations.\n\n");

        prompt.push_str("## User Instructions (.sayn files)\n");
        for inst in &self.instructions {
            prompt.push_str(&format!("### Source: {}\n", inst.file_path));
            prompt.push_str("```sayn\n");
            prompt.push_str(&inst.raw_content);
            prompt.push_str("\n```\n\n");
        }

        if !self.routes.is_empty() {
            prompt.push_str("## Target Routes\n");
            for r in &self.routes {
                prompt.push_str(&format!("- {}\n", r));
            }
            prompt.push('\n');
        }

        if !self.components.is_empty() {
            prompt.push_str("## Target Components\n");
            for c in &self.components {
                prompt.push_str(&format!("- {}\n", c));
            }
            prompt.push('\n');
        }

        if !self.constraints.is_empty() {
            prompt.push_str("## Constraints & Governance\n");
            for c in &self.constraints {
                prompt.push_str(&format!("- {}\n", c));
            }
            prompt.push('\n');
        }

        prompt.push_str("## Output Format\n");
        prompt.push_str("Return one JSON object adhering to this schema (version 1):\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n  \"schema_version\": 1,\n  \"operations\": [\n    {\"kind\": \"create\", \"path\": \"app/page.tsx\", \"content\": \"...\"},\n    {\"kind\": \"modify\", \"path\": \"app/globals.css\", \"content\": \"...\"},\n    {\"kind\": \"delete\", \"path\": \"app/old.tsx\"},\n    {\"kind\": \"retain\", \"path\": \"package.json\"}\n  ],\n  \"acceptance_checks\": [\"...\"],\n  \"summary\": \"...\"\n}\n");
        prompt.push_str("```\n");
        prompt.push_str(
            "Paths are relative to the generated project root. Target each path at most once.\n",
        );

        prompt
    }
}
