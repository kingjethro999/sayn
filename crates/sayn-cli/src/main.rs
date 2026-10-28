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
        Commands::Watch(args) => commands::watch::execute(args, project_opt),
        Commands::Fix(args) => commands::fix::execute(args, project_opt),
        Commands::Snapshot(args) => commands::snapshot::execute(args, project_opt),
        Commands::Rollback(args) => commands::rollback::execute(args, project_opt),
        Commands::Export(args) => commands::export::execute(args, project_opt),
        Commands::Clean(args) => commands::clean::execute(args, project_opt),
        Commands::Permissions(args) => commands::permissions::execute(args),
        Commands::Whoami(args) => commands::whoami::execute(args),
        Commands::Logout(args) => commands::logout::execute(args),
        Commands::Provider(args) => commands::provider::execute(args),
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
    use std::path::PathBuf;

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

    #[test]
    fn test_cli_snapshot_and_rollback() {
        let cli_snap = Cli::try_parse_from(["sayn", "snapshot", "v1"]);
        assert!(cli_snap.is_ok());
        if let Ok(c) = cli_snap {
            match c.command {
                Commands::Snapshot(args) => assert_eq!(args.id, Some("v1".to_string())),
                _ => panic!("Expected Snapshot"),
            }
        }

        let cli_rb = Cli::try_parse_from(["sayn", "rollback", "v1"]);
        assert!(cli_rb.is_ok());
        if let Ok(c) = cli_rb {
            match c.command {
                Commands::Rollback(args) => assert_eq!(args.id, "v1"),
                _ => panic!("Expected Rollback"),
            }
        }
    }

    #[test]
    fn test_cli_export_and_clean() {
        let cli_export = Cli::try_parse_from(["sayn", "export", "--output", "./dist"]);
        assert!(cli_export.is_ok());
        if let Ok(c) = cli_export {
            match c.command {
                Commands::Export(args) => assert_eq!(args.output, Some(PathBuf::from("./dist"))),
                _ => panic!("Expected Export"),
            }
        }

        let cli_clean = Cli::try_parse_from(["sayn", "clean", "--all"]);
        assert!(cli_clean.is_ok());
        if let Ok(c) = cli_clean {
            match c.command {
                Commands::Clean(args) => assert!(args.all),
                _ => panic!("Expected Clean"),
            }
        }
    }

    #[test]
    fn test_cli_watch_and_fix() {
        let cli_watch = Cli::try_parse_from(["sayn", "watch", "--manual"]);
        assert!(cli_watch.is_ok());
        if let Ok(c) = cli_watch {
            match c.command {
                Commands::Watch(args) => assert!(args.manual),
                _ => panic!("Expected Watch"),
            }
        }

        let cli_fix = Cli::try_parse_from(["sayn", "fix", "--dry-run"]);
        assert!(cli_fix.is_ok());
        if let Ok(c) = cli_fix {
            match c.command {
                Commands::Fix(args) => assert!(args.dry_run),
                _ => panic!("Expected Fix"),
            }
        }
    }

    #[test]
    fn test_cli_governance_commands() {
        let cli_perm = Cli::try_parse_from(["sayn", "permissions"]);
        assert!(cli_perm.is_ok());

        let cli_who = Cli::try_parse_from(["sayn", "whoami"]);
        assert!(cli_who.is_ok());

        let cli_logout = Cli::try_parse_from(["sayn", "logout", "--all-providers"]);
        assert!(cli_logout.is_ok());

        let cli_prov = Cli::try_parse_from(["sayn", "provider", "list"]);
        assert!(cli_prov.is_ok());
    }
}
