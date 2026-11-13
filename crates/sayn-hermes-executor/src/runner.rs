use crate::operations::{FileOperation, OperationValidator};
use crate::repair::BoundedRepair;
use crate::session::{ExecutionSession, SessionManager, SessionStatus};
use crate::task::GenerationTask;
use colored::Colorize;
use sayn_hermes_auth::NousAuthManager;
use sayn_hermes_models::ModelCatalog;
use sayn_ir::{GeneratedOperation, GenerationPlan};
use sayn_providers::{
    GenerationProvider, GenerationRequest, NousProvider, OpenAiProvider, ProviderError,
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExecutorError {
    #[error("Task execution error: {0}")]
    Execution(String),

    #[error("No authenticated inference provider is available: {0}")]
    ProviderUnavailable(String),

    #[error("Inference provider request failed: {0}")]
    ProviderRequest(String),

    #[error("Inference provider returned an invalid structured response: {0}")]
    ProviderSchema(String),

    #[error("Operation error: {0}")]
    Operation(#[from] crate::operations::OperationError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub run_id: String,
    pub model_used: String,
    pub provider_used: String,
    pub operations_applied: usize,
    pub repaired: bool,
}

pub struct HermesExecutor;

impl HermesExecutor {
    /// Execute a generation task against the user's configured provider.
    pub fn execute(
        task: &GenerationTask,
        plan: &GenerationPlan,
    ) -> Result<ExecutionResult, ExecutorError> {
        let model = if task.model.is_empty() {
            ModelCatalog::get_active_model()
        } else {
            task.model.clone()
        };
        let provider = Self::resolve_provider(&model)?;
        Self::execute_with_provider(task, plan, provider.as_deref())
    }

    /// Pick the provider the user configured, in a fixed order: a stored Nous
    /// Portal login, then BYOK environment credentials. A Nous login that
    /// cannot be renewed is an error, never a silent switch to another provider.
    fn resolve_provider(model: &str) -> Result<Option<Box<dyn GenerationProvider>>, ExecutorError> {
        match NousAuthManager::load_auth_state() {
            Ok(Some(state)) => {
                let token = NousAuthManager::valid_access_token()
                    .map_err(|error| ExecutorError::ProviderUnavailable(error.to_string()))?;
                return Ok(Some(Box::new(NousProvider::new(
                    state.inference_base_url,
                    token,
                    model,
                ))));
            }
            Ok(None) => {}
            Err(error) => {
                return Err(ExecutorError::ProviderUnavailable(format!(
                    "the stored Nous Portal credentials could not be read: {error}"
                )));
            }
        }

        if let Some(provider) = OpenAiProvider::from_env(model) {
            return Ok(Some(Box::new(provider)));
        }
        if let Some(provider) = NousProvider::from_env(model) {
            return Ok(Some(Box::new(provider)));
        }
        Ok(None)
    }

    fn execute_with_provider(
        task: &GenerationTask,
        _plan: &GenerationPlan,
        provider: Option<&dyn GenerationProvider>,
    ) -> Result<ExecutionResult, ExecutorError> {
        let run_id = task.run_id.clone();
        let started_at = chrono::Utc::now().to_rfc3339();

        let provider = provider.ok_or_else(|| {
            ExecutorError::ProviderUnavailable(
                "run 'sayn auth nous' or configure a supported BYOK credential".into(),
            )
        })?;
        let provider_name = provider.name().to_string();
        let active_model = provider.model().to_string();

        println!(
            "  {} Requesting generation from {} with model {}...",
            "→".blue().bold(),
            provider_name,
            active_model.cyan()
        );

        let response = provider
            .generate(&GenerationRequest {
                prompt: task.format_prompt(),
                target: task.target.clone(),
                instructions: String::new(),
                max_tokens: None,
                temperature: Some(0.2),
            })
            .map_err(|error| match error {
                ProviderError::Authentication(message) => {
                    ExecutorError::ProviderUnavailable(message)
                }
                ProviderError::InvalidResponse(message) => ExecutorError::ProviderSchema(message),
                other => ExecutorError::ProviderRequest(other.to_string()),
            })?;

        // Confine every target before the first write, so a response with one
        // bad path changes nothing.
        let mut operations = Vec::new();
        for operation in &response.operations {
            OperationValidator::validate_path(&task.output_root, operation.path())?;
            match operation {
                GeneratedOperation::Create { path, content } => {
                    operations.push(FileOperation::CreateFile {
                        path: path.clone(),
                        content: content.clone(),
                    });
                }
                GeneratedOperation::Modify { path, content } => {
                    operations.push(FileOperation::UpdateFile {
                        path: path.clone(),
                        content: content.clone(),
                    });
                }
                GeneratedOperation::Delete { path } => {
                    operations.push(FileOperation::DeleteGeneratedFile { path: path.clone() });
                }
                GeneratedOperation::Retain { .. } => {}
            }
        }

        println!(
            "  {} Received {} model-generated file operations",
            "✔".green(),
            response.operations.len()
        );
        let mut applied_ops = Vec::new();
        for operation in &operations {
            if let Some(message) = OperationValidator::apply(&task.output_root, operation)? {
                applied_ops.push(message);
            }
        }

        // Bounded repair check
        let repaired = BoundedRepair::attempt_repair(&task.output_root, 3).unwrap_or(false);

        let completed_at = chrono::Utc::now().to_rfc3339();

        // Record persistent session
        let session = ExecutionSession {
            session_id: format!("sess-{}", run_id),
            project_id: task.project_id.clone(),
            run_id: run_id.clone(),
            started_at,
            completed_at: Some(completed_at),
            model: active_model.clone(),
            provider: provider_name.clone(),
            status: SessionStatus::Completed,
            source_hashes: task.source_hashes.clone(),
            applied_operations: applied_ops.clone(),
            operations_count: applied_ops.len(),
            notes: Some(format!("Generated via {}", provider_name)),
        };

        let _ = SessionManager::save(&task.project_root, &session);

        Ok(ExecutionResult {
            run_id,
            model_used: active_model,
            provider_used: provider_name,
            operations_applied: applied_ops.len(),
            repaired,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{SessionManager, SessionStatus};
    use crate::task::{GenerationTask, TaskInstruction};
    use sayn_core::{Framework, Target};
    use sayn_ir::{GenerationPlan, RoutePlan, TokenEstimate};
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::thread;
    use tempfile::tempdir;

    fn task_and_plan(root: &std::path::Path) -> (GenerationTask, GenerationPlan) {
        let task = GenerationTask {
            project_id: "project-1".into(),
            run_id: "run-1".into(),
            project_root: root.to_path_buf(),
            output_root: root.join(".sayn/generated"),
            target: "web".into(),
            framework: "nextjs".into(),
            model: "test-model".into(),
            provider: "none".into(),
            instructions: vec![TaskInstruction {
                file_path: "app/page.sayn".into(),
                raw_content: "Create a portfolio website.".into(),
                title: None,
                summary: None,
            }],
            routes: vec!["/".into()],
            components: Vec::new(),
            dependencies: Vec::new(),
            styling_guidelines: Vec::new(),
            constraints: Vec::new(),
            existing_files: Vec::new(),
            source_hashes: HashMap::new(),
        };

        let plan = GenerationPlan {
            project_name: "project-1".into(),
            target: Target::Web,
            framework: Framework::NextJs,
            instructions_count: 1,
            routes: vec![RoutePlan {
                route: "/".into(),
                source_instruction: PathBuf::from("app/page.sayn"),
                target_file: PathBuf::from("app/page.tsx"),
            }],
            components: Vec::new(),
            dependencies: Vec::new(),
            operations: Vec::new(),
            token_estimate: TokenEstimate::default(),
            warnings: Vec::new(),
        };

        (task, plan)
    }

    fn completion_server(status: &'static str, body: String) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 16384];
            let _ = stream.read(&mut request);
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        (format!("http://{address}"), server)
    }

    fn provider_at(base_url: String) -> OpenAiProvider {
        OpenAiProvider::new(sayn_providers::OpenAiProviderConfig {
            base_url,
            model: "test-model".into(),
            api_key: Some("secret-token".into()),
        })
    }

    fn completion(content: &str) -> String {
        serde_json::json!({"choices": [{"message": {"content": content}}]}).to_string()
    }

    #[test]
    fn missing_credentials_do_not_create_output_or_completed_session() {
        let dir = tempdir().unwrap();
        let (task, plan) = task_and_plan(dir.path());

        let error = HermesExecutor::execute_with_provider(&task, &plan, None).unwrap_err();

        assert!(matches!(error, ExecutorError::ProviderUnavailable(_)));
        assert!(!task.output_root.join("package.json").exists());
        assert!(SessionManager::load_latest(dir.path()).is_none());
    }

    #[test]
    fn upstream_failure_does_not_create_output_or_completed_session() {
        let (base_url, server) = completion_server("500 Internal Server Error", "nope".into());
        let dir = tempdir().unwrap();
        let (task, plan) = task_and_plan(dir.path());
        let provider = provider_at(base_url);

        let error =
            HermesExecutor::execute_with_provider(&task, &plan, Some(&provider)).unwrap_err();
        server.join().unwrap();

        assert!(matches!(error, ExecutorError::ProviderRequest(_)));
        assert!(!error.to_string().contains("secret-token"));
        assert!(!task.output_root.join("package.json").exists());
        assert!(SessionManager::load_latest(dir.path()).is_none());
    }

    #[test]
    fn structured_response_is_written_and_recorded() {
        let content = r#"{"schema_version":1,"operations":[{"kind":"create","path":"app/page.tsx","content":"export default function Page(){ return null }"},{"kind":"retain","path":"package.json"}],"summary":"page"}"#;
        let (base_url, server) = completion_server("200 OK", completion(content));
        let dir = tempdir().unwrap();
        let (task, plan) = task_and_plan(dir.path());
        let provider = provider_at(base_url);

        let result = HermesExecutor::execute_with_provider(&task, &plan, Some(&provider)).unwrap();
        server.join().unwrap();

        assert_eq!(result.operations_applied, 1);
        assert_eq!(result.model_used, "test-model");
        assert!(task.output_root.join("app/page.tsx").exists());
        let session = SessionManager::load_latest(dir.path()).unwrap();
        assert_eq!(session.status, SessionStatus::Completed);
    }

    #[test]
    fn response_with_an_escaping_path_writes_nothing() {
        let content = r#"{"schema_version":1,"operations":[{"kind":"create","path":"app/page.tsx","content":"ok"},{"kind":"create","path":"../../escape.txt","content":"bad"}]}"#;
        let (base_url, server) = completion_server("200 OK", completion(content));
        let dir = tempdir().unwrap();
        let (task, plan) = task_and_plan(dir.path());
        let provider = provider_at(base_url);

        let error =
            HermesExecutor::execute_with_provider(&task, &plan, Some(&provider)).unwrap_err();
        server.join().unwrap();

        assert!(matches!(error, ExecutorError::ProviderSchema(_)));
        assert!(!task.output_root.join("app/page.tsx").exists());
        assert!(!dir.path().join("escape.txt").exists());
        assert!(SessionManager::load_latest(dir.path()).is_none());
    }
}
