use crate::cli::PlanArgs;
use colored::Colorize;
use sayn_core::{ExitCode, Target};
use sayn_ir::{
    ComponentPlan, DependencyPlan, FileOperation, GenerationPlan, RoutePlan, TokenEstimate,
};
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

    // Build structured plan
    let mut routes = Vec::new();
    let mut components = Vec::new();
    let mut dependencies = Vec::new();
    let mut operations = Vec::new();

    // Default Web (Next.js) dependencies
    if config.sayn.target == Target::Web {
        dependencies.push(DependencyPlan {
            name: "next".to_string(),
            version: "^15.0.0".to_string(),
            is_dev: false,
        });
        dependencies.push(DependencyPlan {
            name: "react".to_string(),
            version: "^19.0.0".to_string(),
            is_dev: false,
        });
        dependencies.push(DependencyPlan {
            name: "react-dom".to_string(),
            version: "^19.0.0".to_string(),
            is_dev: false,
        });
        dependencies.push(DependencyPlan {
            name: "lucide-react".to_string(),
            version: "^0.460.0".to_string(),
            is_dev: false,
        });
    }

    // Analyze documents for routes and sections
    for doc in &parsed_docs {
        let relative_path = doc.file_path.strip_prefix(&root).unwrap_or(&doc.file_path);
        let route_name = if relative_path.file_stem().and_then(|s| s.to_str()) == Some("page") {
            "/".to_string()
        } else {
            format!(
                "/{}",
                relative_path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
            )
        };

        routes.push(RoutePlan {
            route: route_name,
            source_instruction: relative_path.to_path_buf(),
            target_file: PathBuf::from(".sayn/generated/app").join(
                relative_path
                    .with_extension("tsx")
                    .file_name()
                    .unwrap_or_default(),
            ),
        });

        for sec in &doc.sections {
            components.push(ComponentPlan {
                name: sec.title.clone(),
                description: format!("Component planned for section '{}'", sec.title),
                file_path: PathBuf::from(format!(
                    ".sayn/generated/components/{}.tsx",
                    sec.title.replace(' ', "")
                )),
            });
        }
    }

    // Planned file operations
    operations.push(FileOperation::Create {
        path: PathBuf::from(".sayn/generated/package.json"),
        description: "Generate Next.js package manifest".to_string(),
    });
    operations.push(FileOperation::Create {
        path: PathBuf::from(".sayn/generated/app/layout.tsx"),
        description: "Root application layout with theme provider".to_string(),
    });
    operations.push(FileOperation::Create {
        path: PathBuf::from(".sayn/generated/app/page.tsx"),
        description: "Application landing page implementation".to_string(),
    });

    let token_estimate = TokenEstimate {
        input_min: 5000 + (files.len() * 2000),
        input_max: 10000 + (files.len() * 4000),
        output_min: 8000 + (files.len() * 3000),
        output_max: 20000 + (files.len() * 6000),
    };

    let plan = GenerationPlan {
        project_name: config.sayn.name.clone(),
        target: config.sayn.target,
        framework: config.web.framework,
        instructions_count: files.len(),
        routes,
        components,
        dependencies,
        operations,
        token_estimate,
        warnings: Vec::new(),
    };

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
