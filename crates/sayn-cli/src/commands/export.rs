use crate::cli::ExportArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_project::{ProjectConfig, find_project_root};
use sayn_state::ExportManager;
use std::env;
use std::path::PathBuf;

pub fn execute(
    args: &ExportArgs,
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

    let target_dir = args.output.clone().unwrap_or_else(|| root.join("dist"));

    println!(
        "{} Exporting clean deployable application to {}...",
        "✦".cyan().bold(),
        target_dir.display()
    );

    let count = ExportManager::export_clean_application(&gen_dir, &target_dir)?;

    println!(
        "  {} Export completed ({} files copied)",
        "✔".green(),
        count
    );
    println!(
        "  Deployable workspace ready at: {}",
        target_dir.display().to_string().bold()
    );

    Ok(ExitCode::Success)
}
