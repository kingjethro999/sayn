use crate::cli::{AuthArgs, AuthSubcommand};
use colored::Colorize;
use sayn_core::ExitCode;
use sayn_hermes_auth::{
    DEFAULT_NOUS_CLIENT_ID, DEFAULT_NOUS_INFERENCE_URL, DEFAULT_NOUS_PORTAL_URL, DEFAULT_NOUS_SCOPE,
    NousAuthManager, NousAuthState,
};
use sayn_hermes_models::ModelCatalog;

pub fn execute(args: &AuthArgs) -> Result<ExitCode, Box<dyn std::error::Error>> {
    match &args.action {
        Some(AuthSubcommand::Nous { portal_url, no_browser }) => {
            login_nous(portal_url.as_deref(), *no_browser)
        }
        Some(AuthSubcommand::Login) => {
            println!("{} Sayn Authentication\n", "✦".cyan().bold());
            println!("Choose an authentication method:");
            println!("  1. Nous Portal (Recommended): Run '{}'", "sayn auth nous".bold());
            println!("  2. Bring-Your-Own-Key (BYOK): Export {} or {}\n", "SAYN_API_KEY".bold(), "OPENAI_API_KEY".bold());
            Ok(ExitCode::Success)
        }
        Some(AuthSubcommand::Logout) => {
            NousAuthManager::clear_auth_state()?;
            println!("{} Logged out from Nous Portal and cleared local credentials.", "✔".green().bold());
            Ok(ExitCode::Success)
        }
        Some(AuthSubcommand::Status) | None => {
            print_auth_status()
        }
        Some(AuthSubcommand::Token) => {
            match NousAuthManager::load_auth_state()? {
                Some(state) => {
                    let token = state.access_token;
                    if token.len() > 16 {
                        println!("Bearer {}...{}", &token[..8], &token[token.len() - 8..]);
                    } else {
                        println!("Bearer {}", token);
                    }
                    Ok(ExitCode::Success)
                }
                None => {
                    eprintln!("{} Not authenticated with Nous Portal.", "✖".red().bold());
                    eprintln!("Run 'sayn auth nous' to log in.");
                    Ok(ExitCode::GeneralError)
                }
            }
        }
    }
}

fn login_nous(portal_url: Option<&str>, no_browser: bool) -> Result<ExitCode, Box<dyn std::error::Error>> {
    println!("{} Sayn Authentication", "✦".cyan().bold());
    println!("Provider: {}\n", "Nous Portal".bold());

    println!("{} Requesting device authorization from Nous Portal...", "→".blue().bold());
    let device = match NousAuthManager::request_device_code(portal_url) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{} Failed to connect to Nous Portal: {}", "✖".red().bold(), e);
            return Ok(ExitCode::GeneralError);
        }
    };

    println!("\nTo continue authentication:");
    println!("  1. Open: {}", device.verification_uri_complete.cyan().underline());
    println!("  2. Verify confirmation code: {}\n", device.user_code.bold());

    if !no_browser {
        if NousAuthManager::open_browser(&device.verification_uri_complete) {
            println!("  {} Opened browser for verification", "✔".green());
        } else {
            println!("  {} Could not launch graphical browser — please open the link above manually.", "⚠".yellow());
        }
    }

    println!("\n{} Waiting for approval in browser (expires in {}s)...", "⏳".cyan(), device.expires_in);

    let token_resp = match NousAuthManager::poll_device_token(
        portal_url,
        &device.device_code,
        device.expires_in,
        device.interval,
        |elapsed| {
            if elapsed > 0 && elapsed % 15 == 0 {
                println!("  ... still waiting for approval ({}s elapsed)", elapsed);
            }
        },
    ) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("\n{} Authentication failed: {}", "✖".red().bold(), e);
            return Ok(ExitCode::GeneralError);
        }
    };

    let portal = portal_url.unwrap_or(DEFAULT_NOUS_PORTAL_URL).trim_end_matches('/');
    let inference = token_resp
        .inference_base_url
        .unwrap_or_else(|| DEFAULT_NOUS_INFERENCE_URL.to_string());
    let now = chrono::Utc::now();
    let exp_in = token_resp.expires_in.unwrap_or(3600);
    let expires_at = (now + chrono::Duration::seconds(exp_in as i64)).to_rfc3339();

    let auth_state = NousAuthState {
        portal_base_url: portal.to_string(),
        inference_base_url: inference.clone(),
        client_id: DEFAULT_NOUS_CLIENT_ID.to_string(),
        scope: token_resp.scope.unwrap_or_else(|| DEFAULT_NOUS_SCOPE.to_string()),
        token_type: token_resp.token_type,
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token,
        obtained_at: now.to_rfc3339(),
        expires_at,
        expires_in: exp_in,
        free_tier: false,
    };

    NousAuthManager::save_auth_state(&auth_state)?;

    println!("\n{} Authentication successful!", "✔".green().bold());
    println!("  Provider:       {}", "Nous Portal".bold());
    println!("  Inference API:  {}", inference.dimmed());

    // Verify model availability
    println!("\n{} Verifying account model eligibility...", "→".blue().bold());
    let models = ModelCatalog::fetch_models(true).unwrap_or_else(|_| ModelCatalog::curated_models());
    println!("  {} Found {} accessible model(s)", "✔".green(), models.len());

    let active_model = ModelCatalog::get_active_model();
    println!("  Active model:   {}", active_model.cyan().bold());
    println!("\n{} Sayn is ready to craft applications with Nous/Hermes!", "✦".cyan().bold());

    Ok(ExitCode::Success)
}

fn print_auth_status() -> Result<ExitCode, Box<dyn std::error::Error>> {
    println!("{} Sayn Authentication Status:\n", "✦".cyan().bold());

    if let Some(state) = NousAuthManager::load_auth_state()? {
        let is_exp = state.is_expired();
        let status_str = if is_exp {
            "Token Expired (will refresh on next request)".yellow().bold()
        } else {
            "Authenticated & Active".green().bold()
        };

        println!("  Provider:      {}", "Nous Portal".bold());
        println!("  Status:        {}", status_str);
        println!("  Portal:        {}", state.portal_base_url);
        println!("  Inference API: {}", state.inference_base_url);
        println!("  Scope:         {}", state.scope);
        println!("  Expires:       {}", state.expires_at);
        println!("  Active Model:  {}", ModelCatalog::get_active_model().cyan().bold());
        return Ok(ExitCode::Success);
    }

    if std::env::var("SAYN_API_KEY").is_ok() || std::env::var("OPENAI_API_KEY").is_ok() {
        println!("  Provider:      {}", "Bring-Your-Own-Key (BYOK)".bold());
        println!("  Status:        {}", "Configured via environment variable".green());
        println!("  Active Model:  {}", ModelCatalog::get_active_model().cyan().bold());
        return Ok(ExitCode::Success);
    }

    println!("  Status:        {}", "Not Authenticated".yellow());
    println!("\nTo authenticate with your Nous Portal account, run:");
    println!("  {}", "sayn auth nous".bold());
    println!("\nOr export a BYOK environment key:");
    println!("  export SAYN_API_KEY=your-api-key");

    Ok(ExitCode::Success)
}
