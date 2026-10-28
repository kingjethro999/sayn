use crate::cli::{AuthArgs, AuthSubcommand};
use colored::Colorize;
use sayn_core::ExitCode;

pub fn execute(args: &AuthArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    match &args.action {
        Some(AuthSubcommand::Login) => {
            println!("{} Sayn Auth Login", "✦".cyan().bold());
            println!(
                "To use Bring-Your-Own-Key (BYOK), set SAYN_API_KEY or OPENAI_API_KEY environment variable."
            );
            println!("Sayn Cloud authentication will be available in future releases.");
            Ok(ExitCode::Success)
        }
        Some(AuthSubcommand::Logout) => {
            println!("{} Logged out from Sayn local session.", "✔".green());
            Ok(ExitCode::Success)
        }
        Some(AuthSubcommand::Status) | None => {
            println!("{} Sayn Authentication Status:", "✦".cyan().bold());
            if std::env::var("SAYN_API_KEY").is_ok() || std::env::var("OPENAI_API_KEY").is_ok() {
                println!("  Status: Authenticated via BYOK environment key");
            } else {
                println!("  Status: Unauthenticated (BYOK local mode active)");
                println!(
                    "  Tip:    Export SAYN_API_KEY or OPENAI_API_KEY to configure BYOK model providers."
                );
            }
            Ok(ExitCode::Success)
        }
        Some(AuthSubcommand::Token) => {
            println!("Token retrieval is available for Sayn Cloud accounts.");
            Ok(ExitCode::Success)
        }
    }
}
