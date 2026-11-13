pub mod operations;
pub mod repair;
pub mod runner;
pub mod session;
pub mod task;

pub use operations::{FileOperation, OperationError, OperationValidator};
pub use repair::{BoundedRepair, DiagnosticIssue};
pub use runner::{CandidateCheck, ExecutionResult, ExecutorError, HermesExecutor};
pub use session::{ExecutionSession, SessionManager, SessionStatus};
pub use task::{GenerationTask, TaskInstruction};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn generation_task_prompt_contains_the_user_source() {
        let dir = tempdir().unwrap();
        let proj_root = dir.path().to_path_buf();
        let out_root = proj_root.join(".sayn/generated");

        let task = GenerationTask {
            project_id: "test-proj-123".into(),
            run_id: "test-run-456".into(),
            project_root: proj_root.clone(),
            output_root: out_root.clone(),
            target: "web".into(),
            framework: "nextjs".into(),
            model: "hermes-3-llama-3.1-8b".into(),
            provider: "nous".into(),
            instructions: vec![TaskInstruction {
                file_path: "app/page.sayn".into(),
                raw_content: "Create a portfolio website with light and dark mode.".into(),
                title: Some("Portfolio".into()),
                summary: None,
            }],
            routes: vec!["/".into()],
            components: vec!["Header".into(), "Footer".into()],
            dependencies: vec!["clsx".into()],
            styling_guidelines: vec!["Tailwind CSS v4".into()],
            constraints: vec!["TypeScript".into()],
            existing_files: Vec::new(),
            source_hashes: HashMap::new(),
        };

        let prompt = task.format_prompt();
        assert!(prompt.contains("Create a portfolio website with light and dark mode."));
        assert!(prompt.contains("app/page.sayn"));
    }
}
