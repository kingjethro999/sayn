use sayn_core::{Framework, Target};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutePlan {
    pub route: String,
    pub source_instruction: PathBuf,
    pub target_file: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentPlan {
    pub name: String,
    pub description: String,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyPlan {
    pub name: String,
    pub version: String,
    pub is_dev: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileOperation {
    Create { path: PathBuf, description: String },
    Modify { path: PathBuf, description: String },
    Delete { path: PathBuf, reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TokenEstimate {
    pub input_min: usize,
    pub input_max: usize,
    pub output_min: usize,
    pub output_max: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationPlan {
    pub project_name: String,
    pub target: Target,
    pub framework: Framework,
    pub instructions_count: usize,
    pub routes: Vec<RoutePlan>,
    pub components: Vec<ComponentPlan>,
    pub dependencies: Vec<DependencyPlan>,
    pub operations: Vec<FileOperation>,
    pub token_estimate: TokenEstimate,
    pub warnings: Vec<String>,
}

impl GenerationPlan {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}
