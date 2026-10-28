use crate::cli::CreateArgs;
use colored::Colorize;
use sayn_core::{ExitCode, Target};
use sayn_project::create_project;
use std::env;

pub fn execute(args: &CreateArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let target = if args.api {
        println!(
            "{}",
            "Note: API target is currently reserved for future release. Using placeholder."
                .yellow()
        );
        Target::Api
    } else if args.game {
        println!(
            "{}",
            "Note: Game target is currently reserved for future release. Using placeholder."
                .yellow()
        );
        Target::Game
    } else {
        Target::Web
    };

    let (name, destination) = match &args.name {
        Some(n) if n == "." => {
            let current_dir = env::current_dir()?;
            let dir_name = current_dir
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "sayn-app".to_string());
            (dir_name, current_dir)
        }
        Some(n) => {
            let dest = env::current_dir()?.join(n);
            (n.clone(), dest)
        }
        None => {
            let default_name = "sayn-app".to_string();
            let dest = env::current_dir()?.join(&default_name);
            (default_name, dest)
        }
    };

    println!(
        "{} creating Sayn project '{}' at {}",
        "✦".cyan().bold(),
        name.bold(),
        destination.display()
    );

    match create_project(&name, target, &destination) {
        Ok(path) => {
            println!(
                "{} Created Sayn project successfully at {}",
                "✔".green().bold(),
                path.display()
            );
            println!("\nNext steps:");
            if destination != env::current_dir()? {
                println!("  cd {}", name);
            }
            println!("  sayn check    # Validate .sayn instructions");
            println!("  sayn plan     # Preview components and routes");
            println!("  sayn craft    # Generate application and run dev server");
            Ok(ExitCode::Success)
        }
        Err(e) => {
            eprintln!("{} Failed to create project: {}", "✖".red().bold(), e);
            Ok(ExitCode::GeneralError)
        }
    }
}
