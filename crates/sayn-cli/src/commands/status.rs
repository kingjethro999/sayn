use crate::cli::StatusArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_project::{ProjectConfig, find_project_root};
use sayn_source::discover_sayn_files;
use std::env;
use std::path::PathBuf;

pub fn execute(
    _args: &StatusArgs,
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

    let active_model = sayn_hermes_models::ModelCatalog::get_active_model();
    let is_nous_auth = sayn_hermes_auth::NousAuthManager::load_auth_state()
        .ok()
        .flatten()
        .is_some();
    let provider_name = if is_nous_auth {
        "Nous Portal (Authenticated)"
    } else if std::env::var("SAYN_API_KEY").is_ok() || std::env::var("OPENAI_API_KEY").is_ok() {
        "BYOK (Environment key)"
    } else {
        "Not configured"
    };

    println!("{} Sayn Project Status", "✦".cyan().bold());
    println!("  Name:       {}", config.sayn.name.bold());
    println!("  Project ID: {}", config.sayn.project_id.dimmed());
    println!("  Target:     {}", config.sayn.target);
    println!("  Framework:  {}", config.web.framework);
    println!("  Provider:   {}", provider_name.dimmed());
    println!("  Model:      {}", active_model.cyan());
    println!("  Root:       {}", root.display());

    let files = discover_sayn_files(&root, &config.source.roots);
    println!("\nDiscovered Instructions ({}):", files.len());
    for f in &files {
        let rel = f.strip_prefix(&root).unwrap_or(f);
        println!("  - {}", rel.display());
    }

    let gen_dir = root.join(&config.output.root);
    println!("\nGenerated Output:");
    if gen_dir.exists() {
        println!("  Location:   {}", gen_dir.display());
        let has_pkg = gen_dir.join("package.json").exists();
        println!(
            "  Status:     {}",
            if has_pkg {
                "Generated (valid)".green()
            } else {
                "Incomplete".yellow()
            }
        );
    } else {
        println!(
            "  Status:     {}",
            "Not generated yet (run 'sayn craft')".yellow()
        );
    }

    if let Some(session) = sayn_hermes_executor::SessionManager::load_latest(&root) {
        println!("\nLatest Execution Session:");
        println!("  Run ID:     {}", session.run_id.dimmed());
        println!("  Model:      {}", session.model.cyan());
        println!("  Timestamp:  {}", session.started_at);
        println!("  Operations: {}", session.operations_count);
    }

    Ok(ExitCode::Success)
}
