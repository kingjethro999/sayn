use crate::cli::RollbackArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_project::{ProjectConfig, find_project_root};
use sayn_state::SnapshotManager;
use std::env;
use std::path::PathBuf;

pub fn execute(
    args: &RollbackArgs,
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

    let config = ProjectConfig::from_file(&root.join("project.toml"))?;
    let gen_dir = root.join(&config.output.root);

    println!(
        "{} Restoring snapshot '{}'...",
        "✦".cyan().bold(),
        args.id.bold()
    );

    match SnapshotManager::rollback(&root, &gen_dir, &args.id) {
        Ok(info) => {
            println!("  {} Rollback completed successfully", "✔".green());
            println!("  Restored files: {}", info.file_count);
            println!("  Snapshot time:  {}", info.timestamp);
            println!("  Note: User-authored .sayn files were preserved untouched.");
            Ok(ExitCode::Success)
        }
        Err(e) => {
            eprintln!("{} Failed to rollback snapshot: {}", "✖".red().bold(), e);
            Ok(ExitCode::GeneralError)
        }
    }
}
