use crate::cli::CleanArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_project::find_project_root;
use sayn_state::CleanManager;
use std::env;
use std::path::PathBuf;

pub fn execute(
    _args: &CleanArgs,
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

    println!(
        "{} Cleaning disposable caches and build artifacts...",
        "✦".cyan().bold()
    );

    let report = CleanManager::clean_disposable_state(&root)?;

    if report.cleaned_paths.is_empty() {
        println!(
            "  {} No disposable cache files found. Project is already clean.",
            "✔".green()
        );
    } else {
        for path in report.cleaned_paths {
            println!("  {} Removed {}", "✔".green(), path.display());
        }
        println!("  {} Clean operation completed successfully.", "✔".green());
    }

    Ok(ExitCode::Success)
}
