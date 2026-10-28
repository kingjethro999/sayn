use crate::exit_code::ExitCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SaynError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Source error: {0}")]
    Source(String),

    #[error("Planning error: {0}")]
    Plan(String),

    #[error("Generation error: {0}")]
    Generation(String),

    #[error("Build error: {0}")]
    Build(String),

    #[error("Security violation: {0}")]
    Security(String),

    #[error("CLI error: {0}")]
    Cli(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("General error: {0}")]
    General(String),
}

impl SaynError {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            SaynError::Config(_) => ExitCode::ConfigError,
            SaynError::Source(_) => ExitCode::SourceError,
            SaynError::Plan(_) => ExitCode::PlanError,
            SaynError::Generation(_) => ExitCode::GenerationError,
            SaynError::Build(_) => ExitCode::BuildError,
            SaynError::Security(_) => ExitCode::SecurityError,
            SaynError::Cli(_) => ExitCode::CliUsageError,
            SaynError::Io(_) | SaynError::General(_) => ExitCode::GeneralError,
        }
    }
}
