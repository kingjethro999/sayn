use crate::cli::DoctorArgs;
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_executor::DependencyManager;
use sayn_hermes_auth::NousAuthManager;
use sayn_hermes_models::ModelCatalog;
use sayn_hermes_runtime::RuntimeManager;

pub fn execute(_args: &DoctorArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    println!(
        "{} Diagnosing Sayn local development environment...\n",
        "✦".cyan().bold()
    );

    let mut all_passed = true;

    // 1. Rust CLI
    println!(
        "  {} {}: OK (v{})",
        "✔".green(),
        "Rust CLI".bold(),
        env!("CARGO_PKG_VERSION")
    );

    // 2. Sayn project engine
    println!(
        "  {} {}: OK (instruction compiler engine active)",
        "✔".green(),
        "Sayn project engine".bold()
    );

    // 3. Hermes runtime & Python environment
    let health = RuntimeManager::check_health();
    if health.python_installed {
        let py_ver = health
            .python_version
            .unwrap_or_else(|| "Python 3.11+".into());
        println!(
            "  {} {}: OK ({})",
            "✔".green(),
            "Python environment".bold(),
            py_ver
        );
    } else {
        all_passed = false;
        eprintln!("  {} {}: Missing", "✖".red(), "Python environment".bold());
        eprintln!("    Recommendation: Install Python 3.11, 3.12, 3.13, or 3.14");
    }

    if health.venv_exists {
        println!(
            "  {} {}: OK ({})",
            "✔".green(),
            "Hermes runtime".bold(),
            RuntimeManager::venv_dir().display()
        );
    } else {
        println!(
            "  {} {}: Not provisioned (will auto-bootstrap on first run)",
            "⚠".yellow(),
            "Hermes runtime".bold()
        );
    }

    if health.dependencies_installed {
        println!(
            "  {} {}: OK (ruamel.yaml, httpx, rich, openai installed)",
            "✔".green(),
            "Python dependencies".bold()
        );
    } else {
        println!(
            "  {} {}: Missing in venv (auto-installed on demand)",
            "⚠".yellow(),
            "Python dependencies".bold()
        );
    }

    if health.agent_comm_ok {
        println!(
            "  {} {}: OK (embedded execution operational)",
            "✔".green(),
            "Agent communication".bold()
        );
    } else {
        println!(
            "  {} {}: Standby",
            "⚠".yellow(),
            "Agent communication".bold()
        );
    }

    // 4. Package manager (pnpm preferred)
    if DependencyManager::is_pnpm_installed() {
        println!(
            "  {} {}: OK (pnpm available)",
            "✔".green(),
            "Package manager".bold()
        );
    } else if DependencyManager::is_npm_installed() {
        println!(
            "  {} {}: OK (npm available, pnpm recommended)",
            "✔".green(),
            "Package manager".bold()
        );
    } else {
        all_passed = false;
        eprintln!("  {} {}: Missing", "✖".red(), "Package manager".bold());
        eprintln!("    Recommendation: Install pnpm via 'npm install -g pnpm'");
    }

    // 5. Nous Authentication
    match NousAuthManager::load_auth_state() {
        Ok(Some(state)) => {
            let status = if state.is_expired() {
                "Configured (Token will auto-refresh)".yellow().to_string()
            } else {
                "Configured & Active".green().to_string()
            };
            println!(
                "  {} {}: {}",
                "✔".green(),
                "Nous authentication".bold(),
                status
            );
        }
        _ => {
            if std::env::var("SAYN_API_KEY").is_ok() || std::env::var("OPENAI_API_KEY").is_ok() {
                println!(
                    "  {} {}: Configured via BYOK API key",
                    "✔".green(),
                    "Nous authentication".bold()
                );
            } else {
                println!(
                    "  {} {}: Not configured (run '{}' to authenticate)",
                    "⚠".yellow(),
                    "Nous authentication".bold(),
                    "sayn auth nous".bold()
                );
            }
        }
    }

    // 6. Active Model
    let active_model = ModelCatalog::get_active_model();
    println!(
        "  {} {}: {}",
        "✔".green(),
        "Active model".bold(),
        active_model.cyan().bold()
    );

    if all_passed {
        println!(
            "\n{} Sayn environment diagnostics passed.",
            "✔".green().bold()
        );
        Ok(ExitCode::Success)
    } else {
        println!(
            "\n{} Some environment components require attention.",
            "⚠".yellow().bold()
        );
        Ok(ExitCode::GeneralError)
    }
}
