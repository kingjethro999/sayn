/// Standard exit codes for the Sayn compiler toolchain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    /// Command completed successfully.
    Success = 0,
    /// Unspecified general error.
    GeneralError = 1,
    /// Invalid arguments or CLI syntax error.
    CliUsageError = 2,
    /// Invalid or missing project configuration.
    ConfigError = 3,
    /// Source parsing or validation error.
    SourceError = 4,
    /// Planning conflict or resolution failure.
    PlanError = 5,
    /// AI generation or provider communication failure.
    GenerationError = 6,
    /// Target application build or runtime check failure.
    BuildError = 7,
    /// Security violation or unauthorized operation.
    SecurityError = 8,
}

impl ExitCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}
