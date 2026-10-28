use crate::cli::PlanArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_planner::build_generation_plan;
use sayn_project::{ProjectConfig, find_project_root};
use sayn_source::{discover_sayn_files, parse_sayn_source};
use std::env;
use std::fs;
use std::path::PathBuf;

pub fn execute(
    args: &PlanArgs,
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

    let files = discover_sayn_files(&root, &config.source.roots);
    let mut parsed_docs = Vec::new();
    for f in &files {
        let content = fs::read_to_string(f)?;
        parsed_docs.push(parse_sayn_source(f, &content));
    }

    // Build structured plan using sayn-planner crate
    let plan = build_generation_plan(&root, &config, &parsed_docs)?;

    if args.json {
        println!("{}", plan.to_json()?);
        return Ok(ExitCode::Success);
    }

    // Terminal formatted plan
    println!("{}\n", "Sayn Project Planner".bold());
    println!("  Project:     {}", plan.project_name.bold());
    println!("  Target:      {}", plan.target.to_string().cyan());
    println!("  Framework:   {}", plan.framework.to_string().cyan());
    println!("  Instructions discovered: {}", plan.instructions_count);

    println!("\n{}", "Planned Routes:".bold());
    for r in &plan.routes {
        println!(
            "  - {} (from {})",
            r.route.cyan(),
            r.source_instruction.display()
        );
    }

    if !plan.components.is_empty() {
        println!("\n{}", "Planned Components:".bold());
        for c in &plan.components {
            println!("  - {} -> {}", c.name.bold(), c.file_path.display());
        }
    }

    println!("\n{}", "Expected Dependencies:".bold());
    for d in &plan.dependencies {
        println!("  - {} {}", d.name, d.version.dimmed());
    }

    println!("\n{}", "Estimated AI Usage:".bold());
    println!(
        "  Input:  {}-{} tokens",
        plan.token_estimate.input_min, plan.token_estimate.input_max
    );
    println!(
        "  Output: {}-{} tokens",
        plan.token_estimate.output_min, plan.token_estimate.output_max
    );

    if args.offline {
        println!(
            "\n{}",
            "Note: Offline mode specified; no AI requests will be made.".yellow()
        );
    } else {
        println!(
            "\nRun {} to execute this plan.",
            "'sayn craft'".bold().green()
        );
    }

    Ok(ExitCode::Success)
}
