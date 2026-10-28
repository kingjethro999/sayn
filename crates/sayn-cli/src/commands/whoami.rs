use crate::cli::WhoamiArgs;
use colored::Colorize;
use sayn_core::ExitCode;

pub fn execute(_args: &WhoamiArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    println!(
        "{} Sayn Identity & Authentication Status",
        "✦".cyan().bold()
    );

    // Check credentials presence in environment or local config
    let has_openai_key = std::env::var("OPENAI_API_KEY").is_ok();
    let has_sayn_token = std::env::var("SAYN_AUTH_TOKEN").is_ok();

    if has_sayn_token {
        println!("  Authentication: {}", "Sayn Cloud".green().bold());
        println!("  Account status: Active");
    } else if has_openai_key {
        println!(
            "  Authentication: {}",
            "Custom BYOK (OpenAI Compatible)".cyan().bold()
        );
        println!("  Credentials:    Stored in environment (OPENAI_API_KEY)");
    } else {
        println!(
            "  Authentication: {}",
            "Local / Offline Mode".yellow().bold()
        );
        println!("  Note: No cloud account or BYOK provider configured.");
        println!("  Run 'sayn auth' or set OPENAI_API_KEY to enable AI inference.");
    }

    Ok(ExitCode::Success)
}
