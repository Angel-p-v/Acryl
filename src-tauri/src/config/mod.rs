//! Gestión de configuración de la aplicación almacenada en YAML.
//!
//! La configuración se lee desde un fichero YAML en el directorio de datos
//! de la aplicación (o en una ruta explícita). Se expone al frontend
//! a través de comandos Tauri y se gestiona como estado de la aplicación.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Ruta por defecto del directorio de datos de la aplicación (base de datos,
/// copias de seguridad, caché). Respeta `XDG_DATA_HOME` o cae a
/// `~/.local/share/acryl`.
pub fn default_data_dir() -> PathBuf {
    if let Ok(env) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(env).join("acryl");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/share/acryl");
    }
    PathBuf::from("acryl-data")
}

/// Ruta por defecto del fichero de configuración.
///
/// Respeta `XDG_CONFIG_HOME` o cae a `~/.config/acryl/config.yaml`. El
/// usuario puede sobrescribirla con la variable de entorno
/// `MEDIA_MANAGER_CONFIG`.
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

    // En producción, usamos directorios estándar de usuario escribibles,
    // nunca junto al ejecutable (un AppImage monta como solo lectura).
    if let Ok(env) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(env).join("acryl/config.yaml");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config/acryl/config.yaml");
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
    ///
    /// Las rutas por defecto (config y base de datos) se resuelven en
    /// directorios de usuario escribibles (`~/.config/acryl/` y
    /// `~/.local/share/acryl/`), nunca junto al ejecutable, para evitar
    /// fallos en sistemas de archivos de solo lectura (AppImage, Flatpak,
    /// etc.).
    pub fn new() -> Result<Self, String> {
        let config_path = default_config_path();

        // Asegurar que el directorio de configuración existe.
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let config = if config_path.exists() {
            Self::load_from_file(&config_path)?
        } else {
            let mut config = AppConfig::default();
            // Si db_path es el valor por defecto relativo, usar el directorio
            // de datos del usuario (escriturable).
            if config.db_path == "acryl.db" {
                config.db_path = default_data_dir()
                    .join("acryl.db")
                    .to_string_lossy()
                    .to_string();
            }
            Self::save_to_file(&config_path, &config)?;
            config
        };

        // Asegurar que el directorio de datos de la base de datos existe.
        if let Some(parent) = Path::new(&config.db_path).parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

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
