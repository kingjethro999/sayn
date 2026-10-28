use crate::cli::{ModelArgs, ModelSubcommand};
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_hermes_models::ModelCatalog;

pub fn execute(args: &ModelArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    match &args.action {
        Some(ModelSubcommand::List { free, paid, refresh }) => {
            list_models(*free, *paid, *refresh)
        }
        None => {
            list_models(false, false, false)
        }
        Some(ModelSubcommand::Use { model_id }) => {
            select_model(model_id)
        }
        Some(ModelSubcommand::Select { name }) => {
            select_model(name)
        }
        Some(ModelSubcommand::Current) => {
            let active = ModelCatalog::get_active_model();
            println!("Active model: {}", active.cyan().bold());
            Ok(ExitCode::Success)
        }
    }
}

fn list_models(only_free: bool, only_paid: bool, refresh: bool) -> Result<ExitCode, Box<dyn std::error::Error>> {
    println!("{} Sayn Model Catalog\n", "✦".cyan().bold());

    let models = match ModelCatalog::fetch_models(refresh) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{} Failed to fetch live model catalog: {}", "⚠".yellow(), e);
            ModelCatalog::curated_models()
        }
    };

    let active_model = ModelCatalog::get_active_model();

    let mut displayed = 0;
    for model in &models {
        if only_free && !model.free_tier {
            continue;
        }
        if only_paid && model.free_tier {
            continue;
        }

        displayed += 1;
        let is_active = model.id == active_model;
        let marker = if is_active {
            "✔".green().bold()
        } else {
            " ".normal()
        };

        let badge = if model.free_tier {
            "[Free]".green().bold()
        } else {
            "[Paid]".cyan()
        };

        let context_str = model
            .context_length
            .map(|c| format!("{}k context", c / 1024))
            .unwrap_or_else(|| "128k context".into());

        let tools_badge = if model.tool_calling {
            "[tools]".blue()
        } else {
            "".normal()
        };

        let name_str = if is_active {
            model.id.green().bold()
        } else {
            model.id.normal()
        };

        println!("  {} {:<32} {:<8} {:<14} {}", marker, name_str, badge, context_str.dimmed(), tools_badge);

        if let Some(ref desc) = model.description {
            println!("      {}", desc.dimmed());
        }
    }

    if displayed == 0 {
        println!("  No models match the requested criteria.");
    }

    println!("\nSwitch active model with: {}", "sayn model use <model-id>".bold());
    Ok(ExitCode::Success)
}

fn select_model(model_id: &str) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let models = ModelCatalog::curated_models();
    let known = models.iter().any(|m| m.id == model_id);

    if !known {
        println!(
            "{} Model '{}' is not in the curated catalog. Setting as custom model...",
            "⚠".yellow(),
            model_id
        );
    }

    ModelCatalog::set_active_model(model_id)?;
    println!("{} Active model set to: {}", "✔".green().bold(), model_id.cyan().bold());
    Ok(ExitCode::Success)
}
