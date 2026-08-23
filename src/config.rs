use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct SyncConfig {
    pub enabled: bool,
    pub debounce_seconds: u64,
    pub data_dir: Option<PathBuf>,
    pub remote: String,
    pub branch: String,
    pub pull_on_startup: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            debounce_seconds: 5,
            data_dir: None,
            remote: "origin".to_string(),
            branch: "main".to_string(),
            pull_on_startup: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Config {
    pub sync: SyncConfig,
}

impl Config {
    pub fn load() -> Self {
        let Some(mut path) = dirs::config_dir() else {
            return Self::default();
        };
        path.push("hopes");
        path.push("config.toml");
        let Ok(contents) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        toml::from_str(&contents).unwrap_or_default()
    }
}
