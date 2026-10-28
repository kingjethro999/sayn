use crate::cli::InitArgs;
use colored::Colorize;
use sayn_core::{ExitCode, Target};
use sayn_project::init_project;
use std::env;

pub fn execute(args: &InitArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let target_dir = match &args.directory {
        Some(d) => env::current_dir()?.join(d),
        None => env::current_dir()?,
    };

    let dir_name = target_dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "sayn-project".to_string());

    println!(
        "{} Initializing Sayn in {}",
        "✦".cyan().bold(),
        target_dir.display()
    );

    match init_project(&dir_name, Target::Web, &target_dir) {
        Ok(path) => {
            println!(
                "{} Sayn initialized successfully in {}",
                "✔".green().bold(),
                path.display()
            );
            println!("\nNext steps:");
            println!("  sayn check    # Validate instructions");
            println!("  sayn craft    # Generate code and run dev server");
            Ok(ExitCode::Success)
        }
        Err(e) => {
            eprintln!("{} Failed to initialize project: {}", "✖".red().bold(), e);
            Ok(ExitCode::GeneralError)
        }
    }
}
