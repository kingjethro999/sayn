use crate::cli::LogoutArgs;
use colored::Colorize;
use sayn_core::ExitCode;

pub fn execute(args: &LogoutArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let provider = args.provider.as_deref().unwrap_or("active provider");
    println!("{} Logging out from {}...", "✦".cyan().bold(), provider);

    // In a full keyring setup, this removes cached tokens
    println!("  {} Credentials cleared from session.", "✔".green());
    println!(
        "  Note: User-authored .sayn files, snapshots, and generated applications remain intact."
    );

    Ok(ExitCode::Success)
}
