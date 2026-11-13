use crate::cli::BuildArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_executor::{DependencyManager, TOOL_TIMEOUT};
use sayn_project::{ProjectConfig, find_project_root};
use sayn_security::PermissionAction;
use sayn_source::discover_sayn_files;
use std::env;
use std::path::PathBuf;

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
    if !gen_dir.join("package.json").exists() {
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

    // The generated application must correspond to the current instructions.
    // Build never regenerates: that would mean an unrequested model request.
    println!("Checking current source revision...");
    let mut current_source_hashes = std::collections::HashMap::new();
    for file in discover_sayn_files(&root, &config.source.roots) {
        let rel = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        current_source_hashes.insert(rel, sayn_state::Manifest::compute_file_hash(&file)?);
    }
    match sayn_state::Manifest::load(&root) {
        Ok(Some(manifest)) if manifest.is_up_to_date(&current_source_hashes) => {}
        Ok(_) => {
            eprintln!(
                "{} The generated application is out of date with your .sayn instructions.",
                "✖".red().bold()
            );
            eprintln!("Run 'sayn craft' to update it, then run 'sayn build' again.");
            return Ok(ExitCode::BuildError);
        }
        Err(error) => {
            eprintln!(
                "{} The generation manifest could not be read: {}",
                "✖".red().bold(),
                error
            );
            return Ok(ExitCode::GeneralError);
        }
    }

    let Some(tool) = DependencyManager::resolve_package_manager(&config.web.package_manager) else {
        eprintln!(
            "{} No usable package manager (pnpm or npm) was found, so the production build cannot run.",
            "✖".red().bold()
        );
        eprintln!("Install Node.js with pnpm or npm, then run 'sayn build' again.");
        return Ok(ExitCode::BuildError);
    };

    if !gen_dir.join("node_modules").exists() {
        if !config
            .to_security_policy()
            .allows(PermissionAction::InstallDependencies)
        {
            eprintln!(
                "{} Dependencies are not installed and this project does not permit installing them automatically.",
                "✖".red().bold()
            );
            eprintln!(
                "Run '{} install' inside {}, or set install_dependencies = \"allow\" in project.toml, then run 'sayn build' again.",
                tool,
                gen_dir.display()
            );
            return Ok(ExitCode::BuildError);
        }
        println!("Installing dependencies with {tool}...");
        if let Err(error) =
            DependencyManager::run_bounded(tool, &["install"], &gen_dir, TOOL_TIMEOUT)
        {
            eprintln!("{} {}", "✖".red().bold(), error);
            return Ok(ExitCode::BuildError);
        }
    }

    println!("Running production build...");
    if let Err(error) =
        DependencyManager::run_bounded(tool, &["run", "build"], &gen_dir, TOOL_TIMEOUT)
    {
        eprintln!("{} Production build failed: {}", "✖".red().bold(), error);
        return Ok(ExitCode::BuildError);
    }

    println!("{} Production build succeeded.", "✔".green().bold());
    println!("\nGenerated project root: {}", gen_dir.display());
    println!(
        "Framework output:       {}",
        gen_dir.join(".next").display()
    );
    println!("Deploy the generated project root using your hosting provider's Next.js settings.");
    if args.out_dir.is_some() {
        println!(
            "{} --out-dir is not supported for the Next.js target; the framework output stays in the location above.",
            "⚠".yellow().bold()
        );
    }
    Ok(ExitCode::Success)
}
