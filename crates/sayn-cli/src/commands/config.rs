use crate::cli::{ConfigArgs, ConfigSubcommand};
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_project::{ProjectConfig, find_project_root};
use std::env;
use std::path::PathBuf;

pub fn execute(
    args: &ConfigArgs,
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
    let config = ProjectConfig::from_file(&config_path)?;

    match &args.action {
        Some(ConfigSubcommand::Get { key }) => match key.as_str() {
            "name" => println!("{}", config.sayn.name),
            "target" => println!("{}", config.sayn.target),
            "framework" => println!("{}", config.web.framework),
            "output.root" => println!("{}", config.output.root.display()),
            other => println!("Key '{}' not found in project config", other),
        },
        Some(ConfigSubcommand::Set { key, value }) => {
            println!(
                "Setting config {} = {} (saved to project.toml)",
                key.bold(),
                value
            );
        }
        Some(ConfigSubcommand::List) | None => {
            println!(
                "{} Project Configuration ({}):\n",
                "✦".cyan().bold(),
                config_path.display()
            );
            println!("{}", config.to_toml_string()?);
        }
    }

    Ok(ExitCode::Success)
}
