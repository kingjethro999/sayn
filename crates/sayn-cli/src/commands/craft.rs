use crate::cli::CraftArgs;
use colored::Colorize;
use sayn_core::{ExitCode, Severity};
use sayn_executor::{DependencyManager, TOOL_TIMEOUT};
use sayn_hermes_executor::ExecutorError;
use sayn_planner::build_generation_plan;
use sayn_project::{ProjectConfig, find_project_root};
use sayn_security::{PermissionAction, SecurityPolicy};
use sayn_source::{discover_sayn_files, parse_sayn_source, validate_document};
use sayn_state::TransactionError;
use std::cell::Cell;
use std::env;
use std::fs;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

/// Whether dependency installation and the framework build may run, and with
/// which package manager.
enum Tooling {
    Available(&'static str),
    Unavailable(String),
}

impl Tooling {
    /// Decide once, before any inference, so a paid request is never left
    /// waiting on a prompt.
    fn resolve(policy: &SecurityPolicy, package_manager: &str) -> Self {
        let permitted = if policy.allows(PermissionAction::InstallDependencies) {
            true
        } else if policy.requires_confirmation(PermissionAction::InstallDependencies) {
            if !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
                return Self::Unavailable(
                    "installing dependencies needs approval and this session is not interactive; \
                     set install_dependencies = \"allow\" in project.toml to permit it"
                        .into(),
                );
            }
            print!(
                "  Install dependencies and run the production build check in the generated project? [y/N] "
            );
            let _ = std::io::stdout().flush();
            let mut answer = String::new();
            let _ = std::io::stdin().read_line(&mut answer);
            matches!(answer.trim().to_lowercase().as_str(), "y" | "yes")
        } else {
            false
        };
        if !permitted {
            return Self::Unavailable("dependency installation is not permitted".into());
        }
        match DependencyManager::resolve_package_manager(package_manager) {
            Some(tool) => Self::Available(tool),
            None => Self::Unavailable(
                "no usable package manager (pnpm or npm) was found on this machine".into(),
            ),
        }
    }
}

/// Map an executor failure to the exit code and advice a user needs.
fn report_generation_failure(error: &ExecutorError) -> ExitCode {
    eprintln!("{} {}", "✖".red().bold(), error);
    let (code, advice) = match error {
        ExecutorError::ProviderUnavailable(_) => (
            ExitCode::GenerationError,
            "Run 'sayn auth' to sign in, or set SAYN_API_KEY for a custom provider.",
        ),
        ExecutorError::ProviderRequest(_) => (
            ExitCode::GenerationError,
            "The provider could not complete the request. Check 'sayn doctor', then run 'sayn craft' again.",
        ),
        ExecutorError::ProviderSchema(_) => (
            ExitCode::GenerationError,
            "The model did not return a usable result. Run 'sayn craft' again or choose another model with 'sayn model'.",
        ),
        ExecutorError::Materialization(TransactionError::ManualEditConflict { .. }) => (
            ExitCode::PlanError,
            "Restore or remove the listed files (or move your edits into .sayn instructions), then run 'sayn craft' again.",
        ),
        ExecutorError::Materialization(
            TransactionError::UnsafePath(_)
            | TransactionError::UnmanagedDelete(_)
            | TransactionError::NotAFile(_),
        ) => (
            ExitCode::SecurityError,
            "The model proposed a file change Sayn does not allow. Run 'sayn craft' again.",
        ),
        ExecutorError::Materialization(TransactionError::Validation(_)) => (
            ExitCode::BuildError,
            "The generated application did not pass its checks, so it was discarded.",
        ),
        ExecutorError::Materialization(TransactionError::CommitFailed {
            rolled_back: false,
            ..
        }) => {
            eprintln!(
                "{} Some generated files may be in a mixed state. See the journal under .sayn/staging/.",
                "⚠".yellow().bold()
            );
            return ExitCode::GeneralError;
        }
        _ => (ExitCode::GeneralError, "Run 'sayn craft' again."),
    };
    eprintln!("No generated files were changed.");
    eprintln!("{advice}");
    code
}

/// Install dependencies and run the production build inside `dir`.
fn install_and_build(tool: &str, dir: &Path) -> Result<(), String> {
    println!(
        "  {} Installing dependencies with {} for validation...",
        "→".blue().bold(),
        tool
    );
    DependencyManager::run_bounded(tool, &["install"], dir, TOOL_TIMEOUT)?;
    println!(
        "  {} Running the production build check...",
        "→".blue().bold()
    );
    DependencyManager::run_bounded(tool, &["run", "build"], dir, TOOL_TIMEOUT)
}

pub fn execute(
    args: &CraftArgs,
    project_opt: Option<&PathBuf>,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let current_dir = env::current_dir()?;
    let root = match project_opt {
        Some(p) => p.clone(),
        None => match find_project_root(&current_dir) {
            Some(r) => r,
            None => {
                eprintln!(
                    "{} No Sayn project found. Run 'sayn create --web' or 'sayn init' first.",
                    "✖".red().bold()
                );
                return Ok(ExitCode::ConfigError);
            }
        },
    };

    let config_path = root.join("project.toml");
    let config = match ProjectConfig::from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} Failed to parse project.toml: {}", "✖".red().bold(), e);
            return Ok(ExitCode::ConfigError);
        }
    };

    if args.action.as_deref() == Some("resume") {
        println!("{} Resuming previous generation run...", "✦".cyan().bold());
    }

    println!(
        "{} Crafting Sayn project: {}",
        "✦".cyan().bold(),
        config.sayn.name.bold()
    );

    // =========================================================================
    // STEP 1: AUTOMATIC CHECK
    // =========================================================================
    println!(
        "\n{} [1/4] Checking project and instructions...",
        "→".blue().bold()
    );
    let files = discover_sayn_files(&root, &config.source.roots);
    if files.is_empty() {
        eprintln!(
            "{} No .sayn instruction files found in configured roots.",
            "✖".red().bold()
        );
        return Ok(ExitCode::SourceError);
    }

    let mut total_errors = 0;
    let mut parsed_docs = Vec::new();

    for file in &files {
        let content = fs::read_to_string(file)?;
        let doc = parse_sayn_source(file, &content);
        let diags = validate_document(&doc, &root);

        for diag in diags {
            if diag.severity == Severity::Error {
                total_errors += 1;
                eprintln!("{}", diag.to_string().red());
            } else if diag.severity == Severity::Warning {
                println!("{}", diag.to_string().yellow());
            }
        }
        parsed_docs.push(doc);
    }

    if total_errors > 0 {
        eprintln!(
            "{} Automatic check failed with {} error(s). Aborting craft.",
            "✖".red().bold(),
            total_errors
        );
        return Ok(ExitCode::SourceError);
    }
    println!("  {} Instructions validated successfully", "✔".green());

    // =========================================================================
    // STEP 2: AUTOMATIC PLAN
    // =========================================================================
    println!(
        "\n{} [2/4] Planning target application...",
        "→".blue().bold()
    );
    let plan = build_generation_plan(&root, &config, &parsed_docs)?;
    println!(
        "  Target:     {} ({})",
        plan.target.to_string().cyan(),
        plan.framework.to_string().cyan()
    );
    let active_model = args
        .model
        .clone()
        .unwrap_or_else(sayn_hermes_models::ModelCatalog::get_active_model);
    println!("  Routes:     {}", plan.routes.len());
    println!("  Components: {}", plan.components.len());
    println!("  Output:     {}", config.output.root.display());
    println!("  {} Plan generated", "✔".green());

    if args.dry_run {
        println!(
            "\n{} Dry run enabled: skipping file generation.",
            "✔".green().bold()
        );
        return Ok(ExitCode::Success);
    }

    // =========================================================================
    // STEP 3: GENERATE APPLICATION
    // =========================================================================
    println!(
        "\n{} [3/4] Generating application code...",
        "→".blue().bold()
    );
    let gen_root = root.join(&config.output.root);
    let tooling = Tooling::resolve(&config.to_security_policy(), &config.web.package_manager);

    // Compute current source hashes
    let mut current_source_hashes = std::collections::HashMap::new();
    for f in &files {
        let rel = f
            .strip_prefix(&root)
            .unwrap_or(f)
            .to_string_lossy()
            .replace('\\', "/");
        if let Ok(hash) = sayn_state::Manifest::compute_file_hash(f) {
            current_source_hashes.insert(rel, hash);
        }
    }

    let existing_manifest = match sayn_state::Manifest::load(&root) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!(
                "{} The generation manifest (.sayn/state/manifest.json) could not be read: {}",
                "✖".red().bold(),
                error
            );
            eprintln!("No generated files were changed. Your .sayn sources are untouched.");
            return Ok(ExitCode::GeneralError);
        }
    };
    let is_up_to_date = existing_manifest
        .as_ref()
        .map(|m| m.is_up_to_date(&current_source_hashes) && gen_root.join("package.json").exists())
        .unwrap_or(false);

    // Set when the framework build actually ran against the candidate.
    let build_checked = Cell::new(false);
    let mut dependencies_stale = !gen_root.join("node_modules").exists();

    if is_up_to_date && args.action.as_deref() != Some("force") {
        println!(
            "  {} Instruction files unchanged. Reusing verified generated code in {} (no model request made).",
            "✦".cyan().bold(),
            gen_root.display()
        );
    } else {
        // Resolve the provider and, where the provider can list them, confirm
        // the selected model before any chargeable request.
        let nous_login = match sayn_hermes_auth::NousAuthManager::load_auth_state() {
            Ok(state) => state.is_some(),
            Err(error) => {
                eprintln!(
                    "{} Stored Nous Portal credentials could not be read: {}",
                    "✖".red().bold(),
                    error
                );
                eprintln!("No generated files were changed. Run 'sayn auth' to sign in again.");
                return Ok(ExitCode::GenerationError);
            }
        };
        let provider_name = if nous_login {
            "Nous Portal"
        } else if ["SAYN_API_KEY", "OPENAI_API_KEY", "NOUS_API_KEY"]
            .iter()
            .any(|name| env::var(name).is_ok())
        {
            "BYOK (environment key)"
        } else {
            "not configured"
        };
        println!(
            "  Provider:   {}\n  Model:      {}",
            provider_name,
            active_model.cyan()
        );

        if nous_login {
            let entitled_models = match sayn_hermes_models::ModelCatalog::fetch_models(false) {
                Ok(models) => models,
                Err(error) => {
                    eprintln!(
                        "{} Could not load entitled Nous models: {}",
                        "✖".red().bold(),
                        error
                    );
                    eprintln!("No generated files were changed.");
                    return Ok(ExitCode::GenerationError);
                }
            };
            if let Err(error) = sayn_hermes_models::ModelCatalog::validate_selection(
                &active_model,
                &entitled_models,
            ) {
                eprintln!("{} {}", "✖".red().bold(), error);
                eprintln!("No generated files were changed.");
                eprintln!("Run 'sayn model list' and 'sayn model use <model-id>'.");
                return Ok(ExitCode::GenerationError);
            }
        }

        let run_id = if args.action.as_deref() == Some("resume") {
            sayn_hermes_executor::SessionManager::load_latest(&root)
                .map(|s| s.run_id)
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string())
        } else {
            uuid::Uuid::new_v4().to_string()
        };

        let mut task_instructions = Vec::new();
        for f in &files {
            let content = fs::read_to_string(f)?;
            let rel = f
                .strip_prefix(&root)
                .unwrap_or(f)
                .to_string_lossy()
                .to_string();
            task_instructions.push(sayn_hermes_executor::TaskInstruction {
                file_path: rel,
                raw_content: content,
                title: None,
                summary: None,
            });
        }

        let task = sayn_hermes_executor::GenerationTask {
            project_id: config.sayn.project_id.clone(),
            run_id,
            project_root: root.clone(),
            output_root: gen_root.clone(),
            target: config.sayn.target.to_string(),
            framework: config.web.framework.to_string(),
            model: active_model.clone(),
            provider: provider_name.to_string(),
            instructions: task_instructions,
            routes: plan.routes.iter().map(|r| r.route.clone()).collect(),
            components: plan.components.iter().map(|c| c.name.clone()).collect(),
            dependencies: plan.dependencies.iter().map(|d| d.name.clone()).collect(),
            styling_guidelines: vec!["Tailwind CSS v4".into()],
            constraints: vec!["TypeScript".into(), "Next.js App Router".into()],
            existing_files: Vec::new(),
            source_hashes: current_source_hashes.clone(),
        };

        // The candidate is installed and built in staging; only a passing
        // candidate replaces the current application.
        let framework_check = |candidate: &Path| -> Result<(), String> {
            if let Tooling::Available(tool) = &tooling {
                install_and_build(tool, candidate)?;
                build_checked.set(true);
            }
            Ok(())
        };

        let exec_res = match sayn_hermes_executor::HermesExecutor::execute_validated(
            &task,
            &plan,
            &framework_check,
        ) {
            Ok(result) => result,
            Err(error) => return Ok(report_generation_failure(&error)),
        };

        println!(
            "  {} Generation committed: {} file changes (run: {})",
            "✔".green(),
            exec_res.operations_applied,
            exec_res.run_id.dimmed()
        );
        dependencies_stale |= exec_res.package_manifest_changed;

        // Digests come from the committed run, not a rescan, so a later hand
        // edit is detected as a conflict instead of being recorded as generated.
        let mut new_manifest =
            sayn_state::Manifest::new(&config.sayn.project_id, &config.sayn.template_version);
        new_manifest.source_hashes = current_source_hashes;
        new_manifest.generated_hashes = exec_res.generated_hashes.clone();
        new_manifest.save(&root)?;

        let checkpoint_dir = root.join(".sayn/checkpoints");
        fs::create_dir_all(&checkpoint_dir)?;
        fs::write(
            checkpoint_dir.join("latest.json"),
            serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "status": "completed",
                "run_id": exec_res.run_id,
                "files_count": files.len(),
            })
            .to_string(),
        )?;
    }

    // =========================================================================
    // STEP 4: VERIFY AND RUN
    // =========================================================================
    println!(
        "\n{} [4/4] Verifying generated output...",
        "→".blue().bold()
    );
    if !gen_root.join("package.json").exists() || !gen_root.join("app/page.tsx").exists() {
        eprintln!(
            "{} Verification failed: missing generated artifacts",
            "✖".red().bold()
        );
        return Ok(ExitCode::BuildError);
    }
    println!("  {} Application structure verified", "✔".green());

    match &tooling {
        Tooling::Available(tool) => {
            if build_checked.get() {
                println!("  {} Production build check passed", "✔".green());
            }
            if dependencies_stale {
                println!(
                    "  {} Installing dependencies with {}...",
                    "→".blue().bold(),
                    tool
                );
                if let Err(error) =
                    DependencyManager::run_bounded(tool, &["install"], &gen_root, TOOL_TIMEOUT)
                {
                    eprintln!("{} {}", "✖".red().bold(), error);
                    eprintln!(
                        "The generated source is in place, but its dependencies are not installed."
                    );
                    return Ok(ExitCode::BuildError);
                }
                println!("  {} Dependencies installed", "✔".green());
            }
        }
        Tooling::Unavailable(reason) => {
            println!(
                "  {} Dependency install and production build check were not run: {}.",
                "⚠".yellow().bold(),
                reason
            );
            println!("    The generated source passed structural checks only.");
        }
    }

    println!("\n{} Sayn craft completed.", "✔".green().bold());

    let can_serve = gen_root.join("node_modules").exists();
    if args.no_run || !config.craft.run_dev_server || !can_serve {
        if !can_serve {
            println!("Dev server not started: dependencies are not installed.");
        } else {
            println!("Dev server not started.");
        }
        println!("\nYou can run the application with:");
        println!("  {}", "sayn app dev".bold());
        return Ok(ExitCode::Success);
    }

    println!("\n{} Starting development server...", "✦".cyan().bold());
    println!(
        "  You can also run custom scripts with: {}\n",
        "sayn app <script>".bold()
    );
    let port = sayn_executor::DevServerManager::find_available_port(3000);
    let mut server = match sayn_executor::DevServerManager::start_dev_server(&gen_root, port) {
        Ok(child) => child,
        Err(error) => {
            eprintln!(
                "{} The development server could not be started: {}",
                "✖".red().bold(),
                error
            );
            return Ok(ExitCode::BuildError);
        }
    };
    // Stay attached so the server stops with this command and an early exit
    // is reported instead of being left running in the background.
    match server.wait() {
        Ok(status) if status.success() => Ok(ExitCode::Success),
        Ok(status) => {
            eprintln!(
                "{} The development server stopped unexpectedly ({}).",
                "✖".red().bold(),
                status
            );
            Ok(ExitCode::BuildError)
        }
        Err(error) => {
            eprintln!(
                "{} The development server could not be monitored: {}",
                "✖".red().bold(),
                error
            );
            Ok(ExitCode::BuildError)
        }
    }
}
