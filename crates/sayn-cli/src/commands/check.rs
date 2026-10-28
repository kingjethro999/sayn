use crate::cli::CheckArgs;
use colored::Colorize;
use sayn_core::{ExitCode, Severity};
use sayn_project::{ProjectConfig, find_project_root};
use sayn_source::{discover_sayn_files, parse_sayn_source, validate_document};
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn execute(
    args: &CheckArgs,
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

    println!(
        "{} Checking Sayn project: {}",
        "✦".cyan().bold(),
        config.sayn.name.bold()
    );
    println!("  Root:   {}", root.display());
    println!("  Target: {}", config.sayn.target);

    let files = discover_sayn_files(&root, &config.source.roots);
    println!("  Discovered {} .sayn source file(s)", files.len());

    if files.is_empty() {
        println!(
            "{}",
            "Warning: No .sayn source files found in configured roots.".yellow()
        );
    }

    let mut total_errors = 0;
    let mut total_warnings = 0;

    for file in &files {
        let content = fs::read_to_string(file)?;
        let doc = parse_sayn_source(file, &content);
        let diags = validate_document(&doc, &root);

        for diag in diags {
            match diag.severity {
                Severity::Error => {
                    total_errors += 1;
                    eprintln!("{}", diag.to_string().red());
                }
                Severity::Warning => {
                    total_warnings += 1;
                    println!("{}", diag.to_string().yellow());
                }
                _ => {
                    println!("{}", diag);
                }
            }
        }
    }

    if total_errors > 0 || (args.strict && total_warnings > 0) {
        eprintln!(
            "\n{} Check failed: {} error(s), {} warning(s)",
            "✖".red().bold(),
            total_errors,
            total_warnings
        );
        Ok(ExitCode::SourceError)
    } else {
        println!(
            "\n{} All checks passed: 0 errors, {} warning(s)",
            "✔".green().bold(),
            total_warnings
        );
        Ok(ExitCode::Success)
    }
}
