use crate::cli::AppArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_executor::DependencyManager;
use sayn_project::{ProjectConfig, find_project_root};
use std::env;
use std::path::PathBuf;

pub fn execute(
    args: &AppArgs,
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

    let gen_dir = root.join(&config.output.root);
    if !gen_dir.exists() || !gen_dir.join("package.json").exists() {
        eprintln!(
            "{} No generated application found in {}. Run 'sayn craft' first.",
            "✖".red().bold(),
            gen_dir.display()
        );
        return Ok(ExitCode::BuildError);
    }

    // If no script provided, list available scripts
    let script = match &args.script {
        Some(s) => s.clone(),
        None => {
            println!(
                "{} Available scripts in generated project:\n",
                "✦".cyan().bold()
            );
            match DependencyManager::list_scripts(&gen_dir) {
                Ok(scripts) => {
                    if scripts.is_empty() {
                        println!("  No scripts defined in package.json");
                    } else {
                        for (name, cmd) in &scripts {
                            println!("  {} → {}", name.bold(), cmd.dimmed());
                        }
                        println!("\nRun a script with: sayn app <script>");
                    }
                }
                Err(e) => {
                    eprintln!("{} Failed to read scripts: {}", "✖".red().bold(), e);
                }
            }
            return Ok(ExitCode::Success);
        }
    };

    let pm = DependencyManager::detect_package_manager().unwrap_or("pnpm");
    println!(
        "{} Running '{}' in generated project ({})...\n",
        "✦".cyan().bold(),
        format!("{} run {}", pm, script).bold(),
        gen_dir.display()
    );

    let extra_args: Vec<String> = args.args.clone();
    match DependencyManager::run_script(&gen_dir, &script, &extra_args) {
        Ok(true) => Ok(ExitCode::Success),
        Ok(false) => {
            eprintln!(
                "{} Script '{}' failed or package manager not found.",
                "✖".red().bold(),
                script
            );
            Ok(ExitCode::BuildError)
        }
        Err(e) => {
            eprintln!("{} Failed to run script: {}", "✖".red().bold(), e);
            Ok(ExitCode::GeneralError)
        }
    }
}
