use crate::cli::{ModelArgs, ModelSubcommand};
use colored::Colorize;
use sayn_core::ExitCode;

pub fn execute(args: &ModelArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    match &args.action {
        Some(ModelSubcommand::List) | None => {
            println!("{} Available Model Catalog for Sayn:\n", "✦".cyan().bold());
            println!("  * hermes-3-llama-3.1-405b (default)");
            println!("    claude-3-5-sonnet");
            println!("    gpt-4o");
            println!("    gemini-1.5-pro");
            println!("    qwen-2.5-coder-32b");
            println!("\nSwitch active model with: sayn model select <name>");
            Ok(ExitCode::Success)
        }
        Some(ModelSubcommand::Select { name }) => {
            println!("{} Selected model: {}", "✔".green(), name.bold());
            Ok(ExitCode::Success)
        }
    }
}
