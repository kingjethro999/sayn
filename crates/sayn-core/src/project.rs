use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    #[default]
    Web,
    Api,
    Game,
}

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Target::Web => write!(f, "web"),
            Target::Api => write!(f, "api"),
            Target::Game => write!(f, "game"),
        }
    }
}

impl FromStr for Target {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "web" => Ok(Target::Web),
            "api" => Ok(Target::Api),
            "game" => Ok(Target::Game),
            other => Err(format!(
                "Unknown target '{}'. Valid targets are: web, api, game",
                other
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Framework {
    #[default]
    NextJs,
    Custom,
}

impl fmt::Display for Framework {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Framework::NextJs => write!(f, "nextjs"),
            Framework::Custom => write!(f, "custom"),
        }
    }
}

impl FromStr for Framework {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "nextjs" | "next" | "next.js" => Ok(Framework::NextJs),
            "custom" => Ok(Framework::Custom),
            other => Err(format!("Unknown framework '{}'", other)),
        }
    }
}
