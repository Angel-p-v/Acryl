//! Gestión de configuración de la aplicación almacenada en YAML.
//!
//! La configuración se lee desde un fichero YAML en el directorio de datos
//! de la aplicación (o en una ruta explícita). Se expone al frontend
//! a través de comandos Tauri y se gestiona como estado de la aplicación.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Ruta por defecto del fichero de configuración.
///
/// Se resuelve como `<CARGO_MANIFEST_DIR>/config.yaml` en desarrollo y
/// como `<exe_dir>/config.yaml` en producción. El usuario puede
// sobrescribirla con la variable de entorno `MEDIA_MANAGER_CONFIG`.
/// Ruta por defecto del fichero de configuración.
///
/// Se resuelve como `<CARGO_MANIFEST_DIR>/config.yaml` en desarrollo y
/// como `<exe_dir>/config.yaml` en producción. El usuario puede
/// sobrescribirla con la variable de entorno `MEDIA_MANAGER_CONFIG`.
pub fn default_config_path() -> PathBuf {
    if let Ok(env) = std::env::var("MEDIA_MANAGER_CONFIG") {
        return PathBuf::from(env);
    }

    // En desarrollo (cargo run / cargo tauri dev), el ejecutable está en
    // target/debug o target/release. Usamos CARGO_MANIFEST_DIR para
    // escribir junto al código fuente.
    if let Ok(manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        return Path::new(&manifest_dir).join("config.yaml");
    }

    // En producción, junto al ejecutable.
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
    {
        return exe_dir.join("config.yaml");
    }

    PathBuf::from("config.yaml")
}

/// Configuración completa de la aplicación.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Ruta por defecto de la base de datos SQLite.
    pub db_path: String,
    /// Idioma de la interfaz de usuario ("es" o "en").
    pub language: String,
    /// Sincronizar metadatos automáticamente con TMDB/TVDB al indexar.
    pub auto_sync_metadata: bool,
    /// Reproductor de vídeo predeterminado.
    pub default_player: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            db_path: "acryl.db".to_string(),
            language: "es".to_string(),
            auto_sync_metadata: false,
            default_player: "mpv".to_string(),
        }
    }
}

/// Estado gestionado de la configuración.
#[derive(Debug)]
pub struct ConfigState {
    pub config: Mutex<AppConfig>,
    pub config_path: PathBuf,
}

impl ConfigState {
    /// Crea un nuevo estado de configuración cargando desde disco.
    /// Si el fichero no existe, se crea con los valores por defecto.
    pub fn new() -> Result<Self, String> {
        let config_path = default_config_path();
        let config = if config_path.exists() {
            Self::load_from_file(&config_path)?
        } else {
            let config = AppConfig::default();
            Self::save_to_file(&config_path, &config)?;
            config
        };

        Ok(Self {
            config: Mutex::new(config),
            config_path,
        })
    }

    /// Carga la configuración desde un fichero YAML.
    pub fn load_from_file(path: &Path) -> Result<AppConfig, String> {
        let contents = fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_yaml::from_str(&contents).map_err(|e| e.to_string())
    }

    /// Guarda la configuración en un fichero YAML.
    pub fn save_to_file(path: &Path, config: &AppConfig) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let yaml = serde_yaml::to_string(config).map_err(|e| e.to_string())?;
        fs::write(path, yaml).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// Devuelve una referencia a la configuración actual.
    #[allow(dead_code)]
    pub fn get(&self) -> std::sync::MutexGuard<'_, AppConfig> {
        self.config.lock().unwrap()
    }

    /// Actualiza la configuración y la persiste en disco.
    #[allow(dead_code)]
    pub fn set(&self, config: AppConfig) -> Result<(), String> {
        let config_for_save = config.clone();
        *self.config.lock().unwrap() = config;
        Self::save_to_file(&self.config_path, &config_for_save)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn default_config_has_sensible_values() {
        let config = AppConfig::default();
        assert_eq!(config.language, "es");
        assert!(!config.auto_sync_metadata);
        assert_eq!(config.default_player, "mpv");
    }

    #[test]
    fn save_and_load_roundtrip() {
        let mut tmp = NamedTempFile::new().unwrap();
        writeln!(tmp, "db_path: /tmp/test.db").unwrap();
        writeln!(tmp, "language: en").unwrap();
        writeln!(tmp, "auto_sync_metadata: true").unwrap();
        writeln!(tmp, "default_player: mpv").unwrap();
        tmp.flush().unwrap();

        let config = ConfigState::load_from_file(tmp.path()).unwrap();
        assert_eq!(config.language, "en");
        assert!(config.auto_sync_metadata);
        assert_eq!(config.db_path, "/tmp/test.db");
    }

    #[test]
    fn config_state_creates_default_file() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let config_path = tmp_dir.path().join("config.yaml");

        // Simulamos que no existe el fichero y creamos el estado manualmente
        let config = AppConfig::default();
        ConfigState::save_to_file(&config_path, &config).unwrap();

        let loaded = ConfigState::load_from_file(&config_path).unwrap();
        assert_eq!(loaded.language, "es");
        assert!(!loaded.auto_sync_metadata);
    }
}
