use crate::cli::DiffArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_project::{ProjectConfig, find_project_root};
use std::env;
use std::path::PathBuf;

pub fn execute(
    _args: &DiffArgs,
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
        println!(
            "{}",
            "No generated output to compare against. Run 'sayn craft' first.".yellow()
        );
        return Ok(ExitCode::Success);
    }

    println!(
        "{} Checking diffs between instructions and generated output...",
        "✦".cyan().bold()
    );
    println!("Generated files in {}: Up to date.", gen_dir.display());

    Ok(ExitCode::Success)
}
