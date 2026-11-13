use crate::cli::HistoryArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_hermes_executor::SessionManager;
use sayn_project::find_project_root;
use std::env;
use std::path::PathBuf;

pub fn execute(
    _args: &HistoryArgs,
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

    println!("{} Sayn Project Execution History", "✦".cyan().bold());
    println!("  Root: {}\n", root.display());

    let sessions = SessionManager::list(&root);
    if sessions.is_empty() {
        println!("  No previous execution runs recorded in .sayn/sessions.");
        println!(
            "  Run '{}' to execute your instructions.",
            "sayn craft".bold()
        );
        return Ok(ExitCode::Success);
    }

    println!(
        "{:<38} {:<22} {:<24} {:<12} {:<6}",
        "RUN ID", "TIMESTAMP", "MODEL", "STATUS", "OPS"
    );
    println!("{}", "-".repeat(105).dimmed());

    for s in sessions {
        let status_colored = match s.status {
            sayn_hermes_executor::SessionStatus::Completed => "completed".green(),
            sayn_hermes_executor::SessionStatus::Failed => "failed".red(),
            sayn_hermes_executor::SessionStatus::Interrupted => "interrupted".yellow(),
            sayn_hermes_executor::SessionStatus::Running => "running".cyan(),
        };

        println!(
            "{:<38} {:<22} {:<24} {:<12} {:<6}",
            s.run_id.dimmed(),
            s.started_at,
            s.model.cyan(),
            status_colored,
            s.operations_count
        );
    }

    Ok(ExitCode::Success)
}
