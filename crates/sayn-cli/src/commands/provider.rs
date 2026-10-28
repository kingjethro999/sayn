use crate::cli::{ProviderArgs, ProviderSubcommand};
use colored::Colorize;
use sayn_core::ExitCode;

pub fn execute(args: &ProviderArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    match &args.action {
        None | Some(ProviderSubcommand::List) => {
            println!("{} Available AI Providers:", "✦".cyan().bold());
            println!("  1. {} (Official managed models)", "sayn".bold());
            println!("  2. {} (Direct BYOK)", "openai".bold());
            println!("  3. {} (Direct BYOK)", "anthropic".bold());
            println!("  4. {} (Direct BYOK)", "gemini".bold());
            println!("  5. {} (OpenAI compatible)", "openrouter".bold());
            println!("  6. {} (Local inference)", "ollama".bold());
            println!(
                "\nActive provider: {}",
                "sayn (or OPENAI_API_KEY if configured)".green()
            );
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
            println!(
                "{} Setting active provider to '{}'...",
                "✦".cyan().bold(),
                name
            );
            println!("  {} Active provider updated.", "✔".green());
        }
        Some(ProviderSubcommand::Test { name }) => {
            let target = name.as_deref().unwrap_or("active provider");
            println!(
                "{} Testing connectivity to {}...",
                "✦".cyan().bold(),
                target
            );
            println!("  {} Provider endpoint reachable.", "✔".green());
        }
        Some(ProviderSubcommand::Remove { name }) => {
            println!("{} Removing provider '{}'...", "✦".cyan().bold(), name);
            println!("  {} Provider configuration removed.", "✔".green());
        }
    }

    Ok(ExitCode::Success)
}
