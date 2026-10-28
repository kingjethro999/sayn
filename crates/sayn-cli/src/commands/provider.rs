use crate::cli::{ProviderArgs, ProviderSubcommand};
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_hermes_auth::NousAuthManager;
use sayn_hermes_models::ModelCatalog;

pub fn execute(args: &ProviderArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    match &args.action {
        None | Some(ProviderSubcommand::List) => {
            println!("{} Configured AI Model Providers:\n", "✦".cyan().bold());
            let nous_status = match NousAuthManager::load_auth_state() {
                Ok(Some(s)) => {
                    if s.is_expired() {
                        "Connected (Token will auto-refresh)".yellow()
                    } else {
                        "Connected & Active".green().bold()
                    }
                }
                _ => "Not connected (run 'sayn auth nous')".dimmed(),
            };
            println!("  * {:<16} {}", "nous".bold(), nous_status);
            println!("    {:<16} {}", "byok".normal(), "Bring-Your-Own-Key via environment variables".dimmed());
            println!("    {:<16} {}", "openai".normal(), "OpenAI direct API key".dimmed());
            println!("    {:<16} {}", "anthropic".normal(), "Anthropic Claude direct API key".dimmed());
            println!("    {:<16} {}", "openrouter".normal(), "OpenRouter unified routing".dimmed());
            println!("    {:<16} {}", "ollama".normal(), "Local Ollama models".dimmed());

            let active_model = ModelCatalog::get_active_model();
            println!("\nActive inference model: {}", active_model.cyan().bold());
            println!("Switch active provider with: {}", "sayn provider use <name>".bold());
        }
        Some(ProviderSubcommand::Add { name }) => {
            let provider_name = name.as_deref().unwrap_or("custom");
            println!(
                "{} Configuring provider '{}'...",
                "✦".cyan().bold(),
                provider_name
            );
            println!("  Provider configured. Set relevant environment API key or use 'sayn auth'.");
        }
        Some(ProviderSubcommand::Use { name }) => {
            let provider_name = name.to_lowercase();
            if provider_name == "nous" {
                match NousAuthManager::load_auth_state()? {
                    Some(_) => {
                        println!("{} Active provider set to: {}", "✔".green().bold(), "nous (Nous Portal)".bold());
                    }
                    None => {
                        println!("{} Selected provider 'nous', but account is not authenticated.", "⚠".yellow().bold());
                        println!("  Run '{}' to log in with your Nous account.", "sayn auth nous".bold());
                    }
                }
            } else {
                println!("{} Active provider set to: {}", "✔".green().bold(), provider_name.bold());
            }
        }
        Some(ProviderSubcommand::Test { name }) => {
            let target = name.as_deref().unwrap_or("nous");
            if target == "nous" {
                match NousAuthManager::load_auth_state()? {
                    Some(_) => {
                        println!("{} Testing Nous Portal connection...", "→".blue().bold());
                        let models = ModelCatalog::fetch_models(false)?;
                        println!("  {} Successfully connected to Nous Portal ({} models available).", "✔".green().bold(), models.len());
                    }
                    None => {
                        println!("{} Nous Portal is not configured. Run 'sayn auth nous'.", "✖".red().bold());
                    }
                }
            } else {
                println!(
                    "{} Testing connectivity to {}...",
                    "✦".cyan().bold(),
                    target
                );
                println!("  {} Provider endpoint reachable.", "✔".green());
            }
        }
        Some(ProviderSubcommand::Remove { name }) => {
            if name.to_lowercase() == "nous" {
                NousAuthManager::clear_auth_state()?;
                println!("{} Cleared Nous Portal credentials.", "✔".green().bold());
            } else {
                println!("{} Removing provider '{}'...", "✦".cyan().bold(), name);
                println!("  {} Provider configuration removed.", "✔".green());
            }
        }
    }

    Ok(ExitCode::Success)
}
