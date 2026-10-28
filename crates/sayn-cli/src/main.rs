mod cli;
mod commands;

use clap::Parser;
use cli::{Cli, Commands};
use colored::Colorize;
use sayn_core::ExitCode;
use std::process;

fn main() {
    let cli = Cli::parse();
    let project_opt = cli.project.as_ref();

    let result = match &cli.command {
        Commands::Create(args) => commands::create::execute(args),
        Commands::Init(args) => commands::init::execute(args),
        Commands::Check(args) => commands::check::execute(args, project_opt),
        Commands::Plan(args) => commands::plan::execute(args, project_opt),
        Commands::Craft(args) => commands::craft::execute(args, project_opt),
        Commands::Build(args) => commands::build::execute(args, project_opt),
        Commands::Format(args) => commands::format::execute(args, project_opt),
        Commands::Status(args) => commands::status::execute(args, project_opt),
        Commands::Diff(args) => commands::diff::execute(args, project_opt),
        Commands::Doctor(args) => commands::doctor::execute(args),
        Commands::Auth(args) => commands::auth::execute(args),
        Commands::Model(args) => commands::model::execute(args),
        Commands::Config(args) => commands::config::execute(args, project_opt),
    };

    match result {
        Ok(code) => {
            process::exit(code.as_i32());
        }
        Err(e) => {
            eprintln!("{} {}", "error:".red().bold(), e);
            process::exit(ExitCode::GeneralError.as_i32());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_version_flag() {
        // Test parsing help and subcommands
        let cli = Cli::try_parse_from(["sayn", "create", "--web", "portfolio"]);
        assert!(cli.is_ok());
        if let Ok(c) = cli {
            match c.command {
                Commands::Create(args) => {
                    assert!(args.web);
                    assert_eq!(args.name, Some("portfolio".to_string()));
                }
                _ => panic!("Expected Create command"),
            }
        }
    }

    #[test]
    fn test_cli_craft_command() {
        let cli = Cli::try_parse_from(["sayn", "craft", "--no-run"]);
        assert!(cli.is_ok());
        if let Ok(c) = cli {
            match c.command {
                Commands::Craft(args) => {
                    assert!(args.no_run);
                }
                _ => panic!("Expected Craft command"),
            }
        }
    }

    #[test]
    fn test_cli_plan_command() {
        let cli = Cli::try_parse_from(["sayn", "plan", "--json"]);
        assert!(cli.is_ok());
        if let Ok(c) = cli {
            match c.command {
                Commands::Plan(args) => {
                    assert!(args.json);
                }
                _ => panic!("Expected Plan command"),
            }
        }
    }

    #[test]
    fn test_cli_check_command() {
        let cli = Cli::try_parse_from(["sayn", "check", "--strict"]);
        assert!(cli.is_ok());
        if let Ok(c) = cli {
            match c.command {
                Commands::Check(args) => {
                    assert!(args.strict);
                }
                _ => panic!("Expected Check command"),
            }
        }
    }
}
