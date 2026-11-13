use crate::cli::{ModelArgs, ModelSubcommand};
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_hermes_models::ModelCatalog;

pub fn execute(args: &ModelArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    match &args.action {
        Some(ModelSubcommand::List {
            free,
            paid,
            refresh,
        }) => list_models(*free, *paid, *refresh),
        None => list_models(false, false, false),
        Some(ModelSubcommand::Use { model_id }) => select_model(model_id),
        Some(ModelSubcommand::Select { name }) => select_model(name),
        Some(ModelSubcommand::Current) => {
            let active = ModelCatalog::get_active_model();
            println!("Active model: {}", active.cyan().bold());
            Ok(ExitCode::Success)
        }
    }
}

fn list_models(
    only_free: bool,
    only_paid: bool,
    refresh: bool,
) -> Result<ExitCode, Box<dyn std::error::Error>> {
    println!("{} Sayn Model Catalog\n", "✦".cyan().bold());

    let models = match ModelCatalog::fetch_models(refresh) {
        Ok(m) => m,
        Err(e) => {
            eprintln!(
                "{} Failed to fetch entitled model catalog: {}",
                "✖".red(),
                e
            );
            return Ok(ExitCode::GenerationError);
        }
    };

    let active_model = ModelCatalog::get_active_model();

    let mut displayed = 0;
    for model in &models {
        if only_free && model.free_tier != Some(true) {
            continue;
        }
        if only_paid && model.free_tier != Some(false) {
            continue;
        }

        displayed += 1;
        let is_active = model.id == active_model;
        let marker = if is_active {
            "✔".green().bold()
        } else {
            " ".normal()
        };

        let badge = match model.free_tier {
            Some(true) => "[Free]".green().bold(),
            Some(false) => "[Paid]".cyan(),
            None => "[Tier unknown]".yellow(),
        };

        let context_str = model
            .context_length
            .map(|c| format!("{}k context", c / 1024))
            .unwrap_or_else(|| "128k context".into());

        let tools_badge = match model.tool_calling {
            Some(true) => "[tools]".blue(),
            Some(false) => "[no tools]".dimmed(),
            None => "[tools unknown]".yellow(),
        };

        let name_str = if is_active {
            model.id.green().bold()
        } else {
            model.id.normal()
        };

        println!(
            "  {} {:<32} {:<8} {:<14} {}",
            marker,
            name_str,
            badge,
            context_str.dimmed(),
            tools_badge
        );

        if let Some(ref desc) = model.description {
            println!("      {}", desc.dimmed());
        }
    }

    if displayed == 0 {
        println!("  No models match the requested criteria.");
    }

    println!(
        "\nSwitch active model with: {}",
        "sayn model use <model-id>".bold()
    );
    Ok(ExitCode::Success)
}

fn select_model(model_id: &str) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let has_nous_auth = sayn_hermes_auth::NousAuthManager::load_auth_state()?.is_some();
    if has_nous_auth {
        let models = match ModelCatalog::fetch_models(true) {
            Ok(models) => models,
            Err(error) => {
                eprintln!(
                    "{} Could not verify model entitlement: {}",
                    "✖".red(),
                    error
                );
                return Ok(ExitCode::GenerationError);
            }
        };
        if let Err(error) = ModelCatalog::validate_selection(model_id, &models) {
            eprintln!("{} {}", "✖".red(), error);
            return Ok(ExitCode::GenerationError);
        }
    } else if std::env::var("SAYN_API_KEY").is_err()
        && std::env::var("OPENAI_API_KEY").is_err()
        && std::env::var("NOUS_API_KEY").is_err()
    {
        println!(
            "{} Authenticate with Nous or configure BYOK before selecting a model.",
            "✖".red()
        );
        return Ok(ExitCode::GenerationError);
    }

    ModelCatalog::set_active_model(model_id)?;
    println!(
        "{} Active model set to: {}",
        "✔".green().bold(),
        model_id.cyan().bold()
    );
    Ok(ExitCode::Success)
}
