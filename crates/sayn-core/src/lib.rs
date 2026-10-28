pub mod diagnostic;
pub mod error;
pub mod exit_code;
pub mod project;

pub use diagnostic::{Diagnostic, Severity, SourceSpan};
pub use error::SaynError;
pub use exit_code::ExitCode;
pub use project::{Framework, Target};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SAYN_EXTENSION: &str = ".sayn";
pub const SAYN_DIR: &str = ".sayn";
pub const PROJECT_CONFIG_FILE: &str = "project.toml";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exit_codes() {
        assert_eq!(ExitCode::Success.as_i32(), 0);
        assert_eq!(ExitCode::ConfigError.as_i32(), 3);
        assert_eq!(ExitCode::SecurityError.as_i32(), 8);
    }

    #[test]
    fn test_target_from_str() {
        use std::str::FromStr;
        assert_eq!(Target::from_str("web").unwrap(), Target::Web);
        assert_eq!(Target::from_str("api").unwrap(), Target::Api);
        assert_eq!(Target::from_str("game").unwrap(), Target::Game);
        assert!(Target::from_str("invalid").is_err());
    }

    #[test]
    fn test_diagnostic_display() {
        let diag = Diagnostic::error("E001", "Contradictory requirement")
            .with_suggestion("Remove the contradictory requirement");
        let display = format!("{}", diag);
        assert!(display.contains("error[E001]"));
        assert!(display.contains("help:"));
    }
}
