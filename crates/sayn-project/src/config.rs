use sayn_core::{Framework, Target};
use sayn_security::SecurityPolicy;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaynMeta {
    pub format_version: u32,
    pub project_id: String,
    pub name: String,
    pub target: Target,
    pub template: String,
    pub template_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    pub roots: Vec<String>,
    pub extension: String,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            roots: vec!["app".to_string()],
            extension: ".sayn".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    pub root: PathBuf,
    pub commit_generated: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from(".sayn/generated"),
            commit_generated: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    pub framework: Framework,
    pub package_manager: String,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            framework: Framework::NextJs,
            package_manager: "auto".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CraftConfig {
    pub run_dev_server: bool,
    pub preserve_manual_edits: String,
}

impl Default for CraftConfig {
    fn default() -> Self {
        Self {
            run_dev_server: true,
            preserve_manual_edits: "conflict".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionsConfig {
    pub network: String,
    pub install_dependencies: String,
    pub elevated_commands: String,
}

impl Default for PermissionsConfig {
    fn default() -> Self {
        Self {
            network: "ask".to_string(),
            install_dependencies: "ask".to_string(),
            elevated_commands: "deny".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub sayn: SaynMeta,
    #[serde(default)]
    pub source: SourceConfig,
    #[serde(default)]
    pub output: OutputConfig,
    #[serde(default)]
    pub web: WebConfig,
    #[serde(default)]
    pub craft: CraftConfig,
    #[serde(default)]
    pub permissions: PermissionsConfig,
}

impl ProjectConfig {
    pub fn new(name: impl Into<String>, target: Target) -> Self {
        let name_str = name.into();
        Self {
            sayn: SaynMeta {
                format_version: 1,
                project_id: Uuid::new_v4().to_string(),
                name: name_str,
                target,
                template: match target {
                    Target::Web => "web-next".to_string(),
                    Target::Api => "api-placeholder".to_string(),
                    Target::Game => "game-placeholder".to_string(),
                },
                template_version: "0.1.0".to_string(),
            },
            source: SourceConfig::default(),
            output: OutputConfig::default(),
            web: WebConfig::default(),
            craft: CraftConfig::default(),
            permissions: PermissionsConfig::default(),
        }
    }

    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn from_file(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: ProjectConfig = toml::from_str(&content)?;
        Ok(config)
    }

    pub fn to_security_policy(&self) -> SecurityPolicy {
        use std::str::FromStr;
        SecurityPolicy {
            network: sayn_security::PermissionLevel::from_str(&self.permissions.network)
                .unwrap_or_default(),
            install_dependencies: sayn_security::PermissionLevel::from_str(
                &self.permissions.install_dependencies,
            )
            .unwrap_or_default(),
            elevated_commands: sayn_security::PermissionLevel::from_str(
                &self.permissions.elevated_commands,
            )
            .unwrap_or(sayn_security::PermissionLevel::Deny),
        }
    }
}
