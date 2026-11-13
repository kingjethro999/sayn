use crate::repair::BoundedRepair;
use crate::session::{ExecutionSession, SessionManager, SessionStatus};
use crate::task::GenerationTask;
use colored::Colorize;
use sayn_executor::Materializer;
use sayn_hermes_auth::NousAuthManager;
use sayn_hermes_models::ModelCatalog;
use sayn_ir::GenerationPlan;
use sayn_providers::{
    GenerationProvider, GenerationRequest, NousProvider, OpenAiProvider, ProviderError,
};
use sayn_state::Manifest;
use std::path::Path;
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

    #[error("Generated files were not applied: {0}")]
    Materialization(#[from] sayn_state::TransactionError),

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
    /// Whether this run wrote `package.json`, so dependencies need reinstalling.
    pub package_manifest_changed: bool,
    /// Digests of all managed generated files after this run, for the manifest.
    pub generated_hashes: std::collections::HashMap<String, String>,
}

pub struct HermesExecutor;

/// A check run on the complete candidate application before it replaces the
/// current one. An `Err` aborts the run and leaves the output untouched.
pub type CandidateCheck<'a> = &'a dyn Fn(&Path) -> Result<(), String>;

impl HermesExecutor {
    /// Execute a generation task against the user's configured provider.
    pub fn execute(
        task: &GenerationTask,
        plan: &GenerationPlan,
    ) -> Result<ExecutionResult, ExecutorError> {
        Self::execute_validated(task, plan, &|_| Ok(()))
    }

    /// Like [`Self::execute`], with an additional caller-supplied check (for
    /// example a framework build) on the candidate application. The run is
    /// recorded as running, then completed or failed.
    pub fn execute_validated(
        task: &GenerationTask,
        _plan: &GenerationPlan,
        framework_check: CandidateCheck<'_>,
    ) -> Result<ExecutionResult, ExecutorError> {
        let model = if task.model.is_empty() {
            ModelCatalog::get_active_model()
        } else {
            task.model.clone()
        };
        Self::recorded(task, &model, |session| {
            let provider = Self::resolve_provider(&model)?;
            Self::run(task, provider.as_deref(), framework_check, session)
        })
    }

    #[cfg(test)]
    fn execute_with_provider(
        task: &GenerationTask,
        _plan: &GenerationPlan,
        provider: Option<&dyn GenerationProvider>,
    ) -> Result<ExecutionResult, ExecutorError> {
        Self::recorded(task, &task.model, |session| {
            Self::run(task, provider, &|_| Ok(()), session)
        })
    }

    /// Persist the run's state around `body`: `Running` before it starts, then
    /// `Completed` or `Failed`. A run that is killed stays `Running` on disk,
    /// which later commands report as interrupted.
    fn recorded<F>(
        task: &GenerationTask,
        model: &str,
        body: F,
    ) -> Result<ExecutionResult, ExecutorError>
    where
        F: FnOnce(&mut ExecutionSession) -> Result<ExecutionResult, ExecutorError>,
    {
        let mut session = ExecutionSession {
            session_id: format!("sess-{}", task.run_id),
            project_id: task.project_id.clone(),
            run_id: task.run_id.clone(),
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: None,
            model: model.to_string(),
            provider: "unresolved".into(),
            status: SessionStatus::Running,
            source_hashes: task.source_hashes.clone(),
            applied_operations: Vec::new(),
            operations_count: 0,
            notes: None,
        };
        let _ = SessionManager::save(&task.project_root, &session);

        let outcome = body(&mut session);

        session.completed_at = Some(chrono::Utc::now().to_rfc3339());
        match &outcome {
            Ok(_) => {
                session.status = SessionStatus::Completed;
                session.notes = Some(format!("Generated via {}", session.provider));
            }
            Err(error) => {
                session.status = SessionStatus::Failed;
                // Executor errors never carry credentials or response bodies.
                session.notes = Some(error.to_string());
            }
        }
        let _ = SessionManager::save(&task.project_root, &session);
        outcome
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

    fn run(
        task: &GenerationTask,
        provider: Option<&dyn GenerationProvider>,
        framework_check: CandidateCheck<'_>,
        session: &mut ExecutionSession,
    ) -> Result<ExecutionResult, ExecutorError> {
        let provider = provider.ok_or_else(|| {
            ExecutorError::ProviderUnavailable(
                "run 'sayn auth nous' or configure a supported BYOK credential".into(),
            )
        })?;
        let provider_name = provider.name().to_string();
        let active_model = provider.model().to_string();
        session.provider = provider_name.clone();
        session.model = active_model.clone();

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

        println!(
            "  {} Received {} model-generated file operations",
            "✔".green(),
            response.operations.len()
        );

        // Stage, validate the complete candidate, then commit. Any failure
        // leaves the existing generated application exactly as it was.
        let previous = Manifest::load(&task.project_root).map_err(|error| {
            ExecutorError::Execution(format!(
                "the generation manifest could not be read ({error}); \
                 no files were changed"
            ))
        })?;
        let report = Materializer::materialize_response(
            &task.project_root,
            &task.output_root,
            &task.run_id,
            &response,
            previous.as_ref(),
            |candidate| {
                Self::validate_candidate(candidate)?;
                framework_check(candidate)
            },
        )?;
        let applied_ops: Vec<String> = report
            .written
            .iter()
            .map(|path| format!("Wrote {path}"))
            .chain(report.deleted.iter().map(|path| format!("Deleted {path}")))
            .collect();
        session.operations_count = applied_ops.len();
        session.applied_operations = applied_ops;

        Ok(ExecutionResult {
            run_id: task.run_id.clone(),
            model_used: active_model,
            provider_used: provider_name,
            operations_applied: session.operations_count,
            package_manifest_changed: report.written.iter().any(|path| path == "package.json"),
            generated_hashes: report.generated_hashes,
        })
    }

    /// Structural checks on the candidate application before it replaces the
    /// current one. Missing pieces are reported, never filled in.
    fn validate_candidate(candidate: &Path) -> Result<(), String> {
        let problems: Vec<String> = BoundedRepair::inspect_workspace(candidate)
            .into_iter()
            .filter(|issue| issue.severity == "error")
            .map(|issue| issue.description)
            .collect();
        if problems.is_empty() {
            Ok(())
        } else {
            Err(problems.join("; "))
        }
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
        assert_eq!(
            SessionManager::load_latest(dir.path()).unwrap().status,
            SessionStatus::Failed
        );
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
        assert_eq!(
            SessionManager::load_latest(dir.path()).unwrap().status,
            SessionStatus::Failed
        );
    }

    const FULL_APP: &str = r#"{"schema_version":1,"operations":[
        {"kind":"create","path":"package.json","content":"{\"name\":\"app\"}"},
        {"kind":"create","path":"app/layout.tsx","content":"export default function L({children}){return children}"},
        {"kind":"create","path":"app/page.tsx","content":"export default function Page(){ return null }"},
        {"kind":"create","path":"app/globals.css","content":"@import \"tailwindcss\";"},
        {"kind":"retain","path":"next.config.ts"}],"summary":"app"}"#;

    fn run(dir: &std::path::Path, content: &str) -> Result<ExecutionResult, ExecutorError> {
        let (base_url, server) = completion_server("200 OK", completion(content));
        let (task, plan) = task_and_plan(dir);
        let provider = provider_at(base_url);
        let result = HermesExecutor::execute_with_provider(&task, &plan, Some(&provider));
        server.join().unwrap();
        result
    }

    #[test]
    fn structured_response_is_written_and_recorded() {
        let dir = tempdir().unwrap();
        let output = dir.path().join(".sayn/generated");

        let result = run(dir.path(), FULL_APP).unwrap();

        assert_eq!(result.operations_applied, 4);
        assert_eq!(result.model_used, "test-model");
        assert!(output.join("app/page.tsx").exists());
        assert!(
            result
                .generated_hashes
                .contains_key(".sayn/generated/app/page.tsx")
        );
        let session = SessionManager::load_latest(dir.path()).unwrap();
        assert_eq!(session.status, SessionStatus::Completed);
        assert!(!dir.path().join(".sayn/staging/run-1").exists());
    }

    #[test]
    fn response_with_an_escaping_path_writes_nothing() {
        let content = r#"{"schema_version":1,"operations":[{"kind":"create","path":"app/page.tsx","content":"ok"},{"kind":"create","path":"../../escape.txt","content":"bad"}]}"#;
        let dir = tempdir().unwrap();

        let error = run(dir.path(), content).unwrap_err();

        assert!(matches!(error, ExecutorError::ProviderSchema(_)));
        assert!(!dir.path().join(".sayn/generated/app/page.tsx").exists());
        assert!(!dir.path().join("escape.txt").exists());
        assert_eq!(
            SessionManager::load_latest(dir.path()).unwrap().status,
            SessionStatus::Failed
        );
    }

    #[test]
    fn incomplete_application_is_rejected_without_canned_filler() {
        let content = r#"{"schema_version":1,"operations":[{"kind":"create","path":"app/page.tsx","content":"export default function Page(){ return null }"}]}"#;
        let dir = tempdir().unwrap();
        let output = dir.path().join(".sayn/generated");

        let error = run(dir.path(), content).unwrap_err();

        assert!(matches!(
            error,
            ExecutorError::Materialization(sayn_state::TransactionError::Validation(_))
        ));
        assert!(!output.join("app/page.tsx").exists());
        assert!(!output.join("app/layout.tsx").exists());
        assert!(!output.join("package.json").exists());
        assert_eq!(
            SessionManager::load_latest(dir.path()).unwrap().status,
            SessionStatus::Failed
        );
    }

    #[test]
    fn hand_edited_output_survives_a_regeneration_attempt() {
        let dir = tempdir().unwrap();
        let output = dir.path().join(".sayn/generated");
        let first = run(dir.path(), FULL_APP).unwrap();
        let mut manifest = Manifest::new("project-1", "1");
        manifest.generated_hashes = first.generated_hashes;
        manifest.save(dir.path()).unwrap();
        std::fs::write(output.join("app/page.tsx"), "my hand edit").unwrap();

        let error = run(dir.path(), FULL_APP).unwrap_err();

        assert!(matches!(
            error,
            ExecutorError::Materialization(sayn_state::TransactionError::ManualEditConflict { .. })
        ));
        assert_eq!(
            std::fs::read_to_string(output.join("app/page.tsx")).unwrap(),
            "my hand edit"
        );
    }
}
