use crate::cli::CraftArgs;
use colored::Colorize;
use sayn_core::{ExitCode, Severity};
use sayn_executor::Materializer;
use sayn_planner::build_generation_plan;
use sayn_project::{ProjectConfig, find_project_root};
use sayn_source::{discover_sayn_files, parse_sayn_source, validate_document};
use std::env;
use std::fs;
use std::path::PathBuf;

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

    let existing_manifest = sayn_state::Manifest::load(&root).ok().flatten();
    let is_up_to_date = existing_manifest
        .as_ref()
        .map(|m| m.is_up_to_date(&current_source_hashes) && gen_root.join("package.json").exists())
        .unwrap_or(false);

    if is_up_to_date && args.action.as_deref() != Some("force") {
        println!(
            "  {} Instruction files unchanged. Reusing verified generated code in {}.",
            "✦".cyan().bold(),
            gen_root.display()
        );
    } else {
        // Materialize using sayn-executor
        let files_count = Materializer::materialize_plan(&gen_root, &plan)?;
        println!(
            "  {} Materialized {} Next.js artifacts in {}",
            "✔".green(),
            files_count,
            gen_root.display()
        );

        // Update and save manifest
        let mut new_manifest =
            sayn_state::Manifest::new(&config.sayn.project_id, &config.sayn.template_version);
        new_manifest.source_hashes = current_source_hashes;
        if let Ok(gen_hashes) = sayn_state::Manifest::scan_directory_hashes(&root, &gen_root, None)
        {
            new_manifest.generated_hashes = gen_hashes;
        }
        let _ = new_manifest.save(&root);
    }

    // Save checkpoint
    let checkpoint_dir = root.join(".sayn/checkpoints");
    fs::create_dir_all(&checkpoint_dir)?;
    fs::write(
        checkpoint_dir.join("latest.json"),
        format!(
            r#"{{"timestamp": "{}", "status": "completed", "files_count": {}}}"#,
            chrono::Utc::now().to_rfc3339(),
            files.len()
        ),
    )?;

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

    // Auto-install dependencies if node_modules does not exist
    let node_modules = gen_root.join("node_modules");
    if !node_modules.exists() {
        let pm = if !config.web.package_manager.is_empty() {
            config.web.package_manager.as_str()
        } else {
            "pnpm"
        };
        println!(
            "\n{} Auto-installing dependencies with {}...",
            "✦".cyan().bold(),
            pm
        );
        match sayn_executor::DependencyManager::install_dependencies(&gen_root) {
            Ok(true) => {
                println!("  {} Dependencies installed successfully", "✔".green());
            }
            Ok(false) => {
                println!(
                    "  {} Failed to auto-install dependencies. Please ensure {} is installed.",
                    "⚠".yellow().bold(),
                    pm
                );
            }
            Err(e) => {
                println!(
                    "  {} Dependency installation warning: {}",
                    "⚠".yellow().bold(),
                    e
                );
            }
        }
    }

    println!(
        "\n{} Sayn craft completed successfully!",
        "✔".green().bold()
    );

    if args.no_run {
        println!("Dev server omitted (--no-run specified).");
        println!("\nYou can run the application with:");
        println!("  {}", "sayn app dev".bold());
    } else {
        println!("\n{} Starting development server...", "✦".cyan().bold());
        println!("  You can also run custom scripts with: {}\n", "sayn app <script>".bold());
        let _ = sayn_executor::DevServerManager::start_dev_server(&gen_root, 3000);
    }

    Ok(ExitCode::Success)
}
