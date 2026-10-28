use crate::cli::DoctorArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_project::run_diagnostics;

pub fn execute(_args: &DoctorArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    println!(
        "{} Diagnosing Sayn local development environment...\n",
        "✦".cyan().bold()
    );

    let report = run_diagnostics();
    for check in &report.checks {
        if check.passed {
            println!("  {} {}: {}", "✔".green(), check.name.bold(), check.details);
        } else {
            eprintln!("  {} {}: {}", "✖".red(), check.name.bold(), check.details);
            if let Some(rec) = &check.recommendation {
                eprintln!("    Recommendation: {}", rec.yellow());
            }
        }
    }

    if report.all_passed {
        println!(
            "\n{} Environment is fully ready for Sayn.",
            "✔".green().bold()
        );
        Ok(ExitCode::Success)
    } else {
        println!(
            "\n{} Some environment requirements are missing.",
            "✖".yellow().bold()
        );
        Ok(ExitCode::GeneralError)
    }
}
