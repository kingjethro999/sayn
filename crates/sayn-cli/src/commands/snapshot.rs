use crate::cli::SnapshotArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_project::{ProjectConfig, find_project_root};
use sayn_state::SnapshotManager;
use std::env;
use std::path::PathBuf;

pub fn execute(
    args: &SnapshotArgs,
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

    if !gen_dir.exists() {
        eprintln!(
            "{} No generated application found in {}. Run 'sayn craft' first.",
            "✖".red().bold(),
            gen_dir.display()
        );
        return Ok(ExitCode::BuildError);
    }

    println!("{} Creating application snapshot...", "✦".cyan().bold());
    let info = SnapshotManager::create_snapshot(&root, &gen_dir, args.id.as_deref())?;

    println!("  {} Snapshot created successfully", "✔".green());
    println!("  Snapshot ID:     {}", info.id.bold());
    println!("  Generated files: {}", info.file_count);
    println!("  Size:            {} bytes", info.total_bytes);
    println!("  Timestamp:       {}", info.timestamp);
    println!("\nRestore at any time with:");
    println!("  sayn rollback {}", info.id);

    Ok(ExitCode::Success)
}
