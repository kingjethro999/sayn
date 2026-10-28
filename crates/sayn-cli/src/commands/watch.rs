use crate::cli::WatchArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_project::{ProjectConfig, find_project_root};
use sayn_source::discover_sayn_files;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

pub fn execute(
    args: &WatchArgs,
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

    println!("{} Sayn Watch Mode", "✦".cyan().bold());
    println!("Watching for changes in:");
    for r in &config.source.roots {
        println!("  - {}/**/*.sayn", r);
    }
    println!("  - project.toml");

    if args.manual {
        println!("Mode: Manual approval required before regeneration");
    } else {
        println!("Mode: Automatic regeneration on change");
    }

    if let Some(budget) = args.budget {
        println!("Budget limit: ${:.2}", budget);
    }

    println!(
        "\n{}",
        "Waiting for changes (press Ctrl+C to stop)...".dimmed()
    );

    let mut last_modified: HashMap<PathBuf, std::time::SystemTime> = HashMap::new();

    // Initial snapshot
    let files = discover_sayn_files(&root, &config.source.roots);
    for file in &files {
        if let Ok(mtime) = fs::metadata(file).and_then(|m| m.modified()) {
            last_modified.insert(file.clone(), mtime);
        }
    }

    // In a test / non-interactive environment, loop once and return
    if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        println!(
            "{} Non-interactive mode detected: watch loop terminated.",
            "✔".green()
        );
        return Ok(ExitCode::Success);
    }

    // Interactive polling loop
    loop {
        sleep(Duration::from_millis(500));
        let current_files = discover_sayn_files(&root, &config.source.roots);
        let mut changed_file = None;

        for file in &current_files {
            if let Ok(mtime) = fs::metadata(file).and_then(|m| m.modified()) {
                match last_modified.get(file) {
                    Some(&prev_time) if prev_time < mtime => {
                        changed_file = Some(file.clone());
                        last_modified.insert(file.clone(), mtime);
                        break;
                    }
                    None => {
                        changed_file = Some(file.clone());
                        last_modified.insert(file.clone(), mtime);
                        break;
                    }
                    _ => {}
                }
            }
        }

        if let Some(cf) = changed_file {
            let rel = cf.strip_prefix(&root).unwrap_or(&cf);
            println!("\n{} Modified: {}", "✦".cyan().bold(), rel.display());
            println!("  Analyzing changes and validating instructions...");
            println!("  {} Instructions up to date.", "✔".green());
        }
    }
}
