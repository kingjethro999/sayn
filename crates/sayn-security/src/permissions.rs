use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    Allow,
    #[default]
    Ask,
    Deny,
}

impl fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PermissionLevel::Allow => write!(f, "allow"),
            PermissionLevel::Ask => write!(f, "ask"),
            PermissionLevel::Deny => write!(f, "deny"),
        }
    }
}

impl FromStr for PermissionLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "allow" => Ok(PermissionLevel::Allow),
            "ask" => Ok(PermissionLevel::Ask),
            "deny" => Ok(PermissionLevel::Deny),
            other => Err(format!("Unknown permission level '{}'", other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub network: PermissionLevel,
    pub install_dependencies: PermissionLevel,
    pub elevated_commands: PermissionLevel,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            network: PermissionLevel::Ask,
            install_dependencies: PermissionLevel::Ask,
            elevated_commands: PermissionLevel::Deny,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionAction {
    NetworkRequest,
    InstallDependencies,
    ElevatedCommand,
}

impl SecurityPolicy {
    pub fn allows(&self, action: PermissionAction) -> bool {
        match action {
            PermissionAction::NetworkRequest => self.network == PermissionLevel::Allow,
            PermissionAction::InstallDependencies => {
                self.install_dependencies == PermissionLevel::Allow
            }
            PermissionAction::ElevatedCommand => self.elevated_commands == PermissionLevel::Allow,
        }
    }

    pub fn requires_confirmation(&self, action: PermissionAction) -> bool {
        match action {
            PermissionAction::NetworkRequest => self.network == PermissionLevel::Ask,
            PermissionAction::InstallDependencies => {
                self.install_dependencies == PermissionLevel::Ask
            }
            PermissionAction::ElevatedCommand => self.elevated_commands == PermissionLevel::Ask,
        }
    }
}
