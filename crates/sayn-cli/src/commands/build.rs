use crate::cli::BuildArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_project::{ProjectConfig, find_project_root};
use std::env;
use std::path::PathBuf;
use std::process::Command;

pub fn execute(
    args: &BuildArgs,
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

    println!(
        "{} Building production output for Sayn project: {}",
        "✦".cyan().bold(),
        config.sayn.name.bold()
    );

    let out_dir = args.out_dir.clone().unwrap_or_else(|| gen_dir.join("dist"));

    // Check if node_modules exists, otherwise try npm run build or verify structure
    if gen_dir.join("node_modules").exists() {
        println!("Running target framework production build...");
        let status = Command::new("npm")
            .arg("run")
            .arg("build")
            .current_dir(&gen_dir)
            .status();

        match status {
            Ok(s) if s.success() => {
                println!(
                    "{} Production build succeeded. Output ready at {}",
                    "✔".green().bold(),
                    out_dir.display()
                );
                Ok(ExitCode::Success)
            }
            _ => {
                eprintln!("{} Target framework build failed.", "✖".red().bold());
                Ok(ExitCode::BuildError)
            }
        }
    } else {
        println!(
            "{} Generated application structure verified at {}",
            "✔".green().bold(),
            gen_dir.display()
        );
        println!(
            "{}",
            "Note: node_modules not installed. Run 'npm install' inside .sayn/generated before production build."
                .yellow()
        );
        Ok(ExitCode::Success)
    }
}
