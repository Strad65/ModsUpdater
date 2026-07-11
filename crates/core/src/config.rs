use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::error::{CoreError, CoreResult};

const APP_NAME: &str = "modsupdater";
const CONFIG_FILE: &str = "config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Last used mods directory
    pub last_mods_dir: Option<PathBuf>,
    /// Default mod loader (fabric, forge, quilt, etc.)
    pub last_loader: Option<String>,
    /// Default Minecraft version
    pub last_game_version: Option<String>,
    /// Custom User-Agent identifier (username/project)
    pub user_agent_identifier: Option<String>,
    /// Cached computed user agent (not serialized)
    #[serde(skip)]
    user_agent_cache: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut config = Self {
            last_mods_dir: None,
            last_loader: None,
            last_game_version: None,
            user_agent_identifier: None,
            user_agent_cache: None,
        };
        config.finalize();
        config
    }
}

impl AppConfig {
    /// Load config from the standard config directory.
    /// Linux: ~/.config/modrinth-updater/config.toml
    /// Windows: %APPDATA%/modrinth-updater/config.toml
    pub fn load() -> CoreResult<Self> {
        let config_path = Self::config_path()?;
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path).map_err(|e| {
                CoreError::Config(format!("Failed to read config at {:?}: {}", config_path, e))
            })?;
            let config: AppConfig = toml::from_str(&content).map_err(|e| {
                CoreError::Config(format!("Failed to parse config: {}", e))
            })?;
            let mut config = config;
            config.finalize();
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save config to the standard config directory.
    pub fn save(&self) -> CoreResult<()> {
        let config_path = Self::config_path()?;
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CoreError::Config(format!(
                    "Failed to create config directory {:?}: {}",
                    parent, e
                ))
            })?;
        }
        let content = toml::to_string_pretty(self).map_err(|e| {
            CoreError::Config(format!("Failed to serialize config: {}", e))
        })?;
        std::fs::write(&config_path, content).map_err(|e| {
            CoreError::Config(format!("Failed to write config to {:?}: {}", config_path, e))
        })?;
        Ok(())
    }

    /// Get the path to the config file.
    fn config_path() -> CoreResult<PathBuf> {
        let dir = directories::ProjectDirs::from("", "", APP_NAME)
            .map(|d| PathBuf::from(d.config_dir()))
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(dir.join(CONFIG_FILE))
    }

    /// Get the user agent string, using the configured identifier or a default.
    pub fn user_agent(&self) -> &str {
        self.user_agent_cache
            .as_deref()
            .unwrap_or("unknown/modsupdater/0.1.0")
    }

    /// Compute the cached user agent string.
    fn finalize(&mut self) {
        self.user_agent_cache = Some(
            self.user_agent_identifier
                .clone()
                .unwrap_or_else(|| "unknown/modsupdater/0.1.0".to_string()),
        );
    }

    /// Get the data directory for storing reports and cached data.
    pub fn data_dir() -> CoreResult<PathBuf> {
        let dir = directories::ProjectDirs::from("", "", APP_NAME)
            .map(|d| PathBuf::from(d.data_dir()))
            .unwrap_or_else(|| PathBuf::from("."));
        std::fs::create_dir_all(&dir).map_err(|e| {
            CoreError::Config(format!("Failed to create data directory {:?}: {}", dir, e))
        })?;
        Ok(dir)
    }
}
