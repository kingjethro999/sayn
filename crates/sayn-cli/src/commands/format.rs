use crate::cli::FormatArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_formatter::{check_source, diff_source, format_source};
use sayn_project::{ProjectConfig, find_project_root};
use sayn_source::discover_sayn_files;
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn execute(
    args: &FormatArgs,
    project_opt: Option<&PathBuf>,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let current_dir = env::current_dir()?;
    let files_to_format = if let Some(p) = &args.path {
        if p.is_file() {
            vec![p.clone()]
        } else if p.is_dir() {
            discover_sayn_files(p, &[])
        } else {
            eprintln!(
                "{} Specified path '{}' does not exist.",
                "✖".red().bold(),
                p.display()
            );
            return Ok(ExitCode::CliUsageError);
        }
    } else {
        let root = match project_opt {
            Some(p) => p.clone(),
            None => match find_project_root(&current_dir) {
                Some(r) => r,
                None => current_dir.clone(),
            },
        };
        let roots_to_scan = if let Ok(config) = ProjectConfig::from_file(&root.join("project.toml"))
        {
            config.source.roots
        } else {
            vec!["app".to_string()]
        };
        discover_sayn_files(&root, &roots_to_scan)
    };

    if files_to_format.is_empty() {
        println!("{}", "No .sayn files found to format.".yellow());
        return Ok(ExitCode::Success);
    }

    let mut unformatted_count = 0;

    for file in &files_to_format {
        let content = fs::read_to_string(file)?;
        let is_formatted = check_source(&content);

        if !is_formatted {
            unformatted_count += 1;
            let formatted = format_source(&content);

            if args.diff {
                println!("--- {} ---", file.display());
                println!("{}", diff_source(&content, &formatted));
            } else if !args.check {
                fs::write(file, &formatted)?;
                println!("{} Formatted {}", "✔".green(), file.display());
            } else {
                eprintln!("{} Needs formatting: {}", "✖".red(), file.display());
            }
        }
    }

    if args.check && unformatted_count > 0 {
        eprintln!(
            "\n{} {} file(s) would be reformatted.",
            "✖".red().bold(),
            unformatted_count
        );
        Ok(ExitCode::SourceError)
    } else {
        if unformatted_count == 0 {
            println!(
                "{} All .sayn files are properly formatted.",
                "✔".green().bold()
            );
        }
        Ok(ExitCode::Success)
    }
}
