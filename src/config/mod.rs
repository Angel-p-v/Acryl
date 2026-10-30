//! Gestión de configuración YAML.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub fn default_config_path() -> PathBuf {
    if let Ok(env) = std::env::var("ACRYL_CONFIG") {
        return PathBuf::from(env);
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/share/acryl/config.yaml");
    }
    PathBuf::from("config.yaml")
}

pub fn default_data_dir() -> PathBuf {
    if let Ok(env) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(env).join("acryl");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/share/acryl");
    }
    PathBuf::from("acryl-data")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub db_path: String,
    pub language: String,
    pub auto_sync_metadata: bool,
    pub default_player: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            db_path: default_data_dir()
                .join("acryl.db")
                .to_string_lossy()
                .to_string(),
            language: "es".to_string(),
            auto_sync_metadata: false,
            default_player: "mpv".to_string(),
        }
    }
}

pub struct Config {
    pub inner: AppConfig,
    pub path: PathBuf,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = default_config_path();

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let inner = if config_path.exists() {
            let contents = fs::read_to_string(&config_path)?;
            serde_yaml::from_str(&contents)?
        } else {
            let inner = AppConfig::default();
            Self::save_to_file(&config_path, &inner)?;
            inner
        };

        Ok(Config {
            path: config_path,
            inner,
        })
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        Self::save_to_file(&self.path, &self.inner)
    }

    pub fn inner(&self) -> &AppConfig {
        &self.inner
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn save_to_file(path: &Path, config: &AppConfig) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let yaml = serde_yaml::to_string(config)?;
        fs::write(path, yaml)?;
        Ok(())
    }
}

impl Clone for Config {
    fn clone(&self) -> Self {
        Self {
            path: self.path.clone(),
            inner: self.inner.clone(),
        }
    }
}
