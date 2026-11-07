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

/// Preferencias predeterminadas de pistas para reproducir cualquier medio.
///
/// Se aplican cuando el archivo no tiene una preferencia propia guardada y
/// tienen prioridad sobre cualquier preferencia por-archivo.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultTracks {
    /// Idioma de audio predeterminado (`language:ja`, `language:es`, ...).
    ///
    /// Las claves antiguas por pista (`embedded:audio:jpn:aac:japanese`) se
    /// siguen aceptando y se migran a la clave por idioma al cargar Ajustes.
    #[serde(default)]
    pub audio: Option<String>,
    /// Idioma de subtítulos predeterminado (`language:es`, `language:und`, ...).
    ///
    /// Las claves antiguas por pista también se aceptan por compatibilidad.
    #[serde(default)]
    pub subtitle: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub db_path: String,
    #[serde(default)]
    pub media_folders: Vec<String>,
    pub language: String,
    pub auto_sync_metadata: bool,
    pub default_player: String,
    #[serde(default)]
    pub default_tracks: DefaultTracks,
    /// Modo de tema: "dark" (por defecto) o "light".
    #[serde(default)]
    pub theme: String,
    /// Reproducir automáticamente el siguiente episodio al terminar uno.
    #[serde(default = "default_auto_play_next_episode")]
    pub auto_play_next_episode: bool,
    /// Umbral de completado de una reproducción, en segundos.
    ///
    /// Una reproducción se marca como completada (y el archivo como visto) cuando
    /// le quedan menos de este umbral para terminar. Para películas se usa el
    /// doble de este valor, manteniendo el comportamiento histórico (300s para
    /// episodios/anime, 600s para películas por defecto).
    #[serde(default = "default_completion_threshold_seconds")]
    pub playback_completion_threshold_seconds: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            db_path: default_data_dir()
                .join("acryl.db")
                .to_string_lossy()
                .to_string(),
            media_folders: Vec::new(),
            language: "es".to_string(),
            auto_sync_metadata: false,
            default_player: "mpv".to_string(),
            default_tracks: DefaultTracks::default(),
            theme: "dark".to_string(),
            auto_play_next_episode: default_auto_play_next_episode(),
            playback_completion_threshold_seconds: default_completion_threshold_seconds(),
        }
    }
}

/// Umbral de completado por defecto: 300 segundos (5 minutos).
fn default_completion_threshold_seconds() -> u64 {
    300
}

/// Reproducción automática del siguiente episodio activada por defecto.
fn default_auto_play_next_episode() -> bool {
    true
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

    /// Exporta la configuración actual a un archivo YAML en `path`.
    pub fn export_config_to(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        Self::save_to_file(path, &self.inner)
    }

    /// Importa configuración desde un archivo YAML en `path`, la valida y la
    /// guarda como la configuración activa. Devuelve el `AppConfig` importado.
    pub fn import_config_from(
        &mut self,
        path: &Path,
    ) -> Result<AppConfig, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let inner: AppConfig = serde_yaml::from_str(&contents)?;
        // Validar que las carpetas de medios existan y sean legibles.
        for folder in &inner.media_folders {
            if Path::new(folder).exists() && Path::new(folder).is_dir() {
                // OK
            } else {
                return Err(format!("carpeta de medios no válida: {}", folder).into());
            }
        }
        self.inner = inner;
        self.save()?;
        Ok(self.inner.clone())
    }

    /// Copia el archivo de base de datos SQLite a `path` (backup).
    ///
    /// La base de datos usa WAL, así que antes de copiar se fuerza un
    /// checkpoint para que el archivo principal contenga todos los datos
    /// confirmados. Sin este paso, la copia podría omitir transacciones que
    /// todavía viven en el archivo `-wal`.
    pub fn backup_db_to(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let db_path = Path::new(&self.inner.db_path);
        if !db_path.exists() {
            return Err(format!("base de datos no encontrada: {}", db_path.display()).into());
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        {
            let conn = rusqlite::Connection::open(db_path)?;
            conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        }
        fs::copy(db_path, path)?;
        Ok(())
    }

    /// Restaura el archivo de base de datos SQLite desde `path`.
    ///
    /// Valida la copia antes de sobrescribir la base activa y elimina los
    /// archivos `-wal`/`-shm` de la base actual para que SQLite no mezcle
    /// cambios antiguos con los datos restaurados. El llamador debe volver a
    /// abrir la BD después.
    pub fn restore_db_from(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if !path.exists() {
            return Err(format!("archivo de backup no encontrado: {}", path.display()).into());
        }

        // Validar que la copia es una base SQLite legible antes de reemplazar
        // la biblioteca del usuario.
        {
            let conn = rusqlite::Connection::open_with_flags(
                path,
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )?;
            let check: String = conn.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
            if check != "ok" {
                return Err(format!("copia de seguridad no válida: {}", check).into());
            }
        }

        let db_path = Path::new(&self.inner.db_path);
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }

        for suffix in ["-wal", "-shm"] {
            let mut sidecar = db_path.as_os_str().to_os_string();
            sidecar.push(suffix);
            let sidecar = PathBuf::from(sidecar);
            if sidecar.exists() {
                fs::remove_file(sidecar)?;
            }
        }

        fs::copy(path, db_path)?;
        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config_with_tmp() -> (Config, TempDir, TempDir) {
        let config_dir = TempDir::new().expect("config dir");
        let data_dir = TempDir::new().expect("data dir");
        let config_path = config_dir.path().join("config.yaml");
        let db_path = data_dir.path().join("acryl.db");
        // Crear una base SQLite real para que backup/restore puedan validarla.
        {
            let conn = rusqlite::Connection::open(&db_path).unwrap();
            conn.execute_batch(
                "CREATE TABLE prueba (id INTEGER PRIMARY KEY, valor TEXT);
                 INSERT INTO prueba (valor) VALUES ('original');",
            )
            .unwrap();
        }

        let config = Config {
            path: config_path.clone(),
            inner: AppConfig {
                db_path: db_path.to_string_lossy().to_string(),
                media_folders: vec!["/tmp/foo".to_string()],
                language: "es".to_string(),
                auto_sync_metadata: false,
                default_player: "mpv".to_string(),
                default_tracks: DefaultTracks::default(),
                theme: "dark".to_string(),
                playback_completion_threshold_seconds: 300,
                auto_play_next_episode: true,
            },
        };
        (config, config_dir, data_dir)
    }

    #[test]
    fn export_config_to_escribe_yaml_valido() {
        let (config, config_dir, _data_dir) = test_config_with_tmp();
        let export_path = config_dir.path().join("exported.yaml");
        config
            .export_config_to(&export_path)
            .expect("exportar config");
        let contents = fs::read_to_string(&export_path).expect("leer exportado");
        assert!(contents.contains("db_path:"));
        assert!(contents.contains("language:"));
        assert!(contents.contains("media_folders:"));
        drop(config);
        drop(config_dir);
    }

    #[test]
    fn import_config_from_carga_y_valida() {
        let (mut config, config_dir, _data_dir) = test_config_with_tmp();
        let import_path = config_dir.path().join("import.yaml");
        // Crear la carpeta de medios que se va a importar.
        std::fs::create_dir_all("/tmp/bar").unwrap();
        // Escribir un YAML válido para importar.
        let yaml = r#"
db_path: /tmp/imported.db
media_folders:
  - /tmp/bar
language: en
auto_sync_metadata: true
default_player: mpv
default_tracks: {}
playback_completion_threshold_seconds: 600
"#;
        fs::write(&import_path, yaml).expect("escribir import");
        let imported = config
            .import_config_from(&import_path)
            .expect("importar config");
        assert_eq!(imported.language, "en");
        assert_eq!(imported.default_player, "mpv");
        assert_eq!(imported.playback_completion_threshold_seconds, 600);
        // La config activa se actualizó.
        assert_eq!(config.inner().language, "en");
        drop(config);
        drop(config_dir);
        let _ = std::fs::remove_dir_all("/tmp/bar");
    }

    #[test]
    fn import_config_from_rechaza_carpeta_inexistente() {
        let (mut config, config_dir, _data_dir) = test_config_with_tmp();
        let import_path = config_dir.path().join("bad_import.yaml");
        let yaml = r#"
db_path: /tmp/x.db
media_folders:
  - /tmp/no-existe-xyz-12345
language: es
auto_sync_metadata: false
default_player: mpv
default_tracks: {}
"#;
        fs::write(&import_path, yaml).expect("escribir import");
        let resultado = config.import_config_from(&import_path);
        assert!(resultado.is_err(), "debe rechazar carpeta inexistente");
        drop(config);
        drop(config_dir);
    }

    #[test]
    fn backup_db_to_copia_archivo_bd() {
        let (config, config_dir, _data_dir) = test_config_with_tmp();
        let backup_path = config_dir.path().join("backup.db");
        config.backup_db_to(&backup_path).expect("backup db");
        assert!(backup_path.exists());

        let copia = rusqlite::Connection::open(&backup_path).expect("abrir backup");
        let valor: String = copia
            .query_row("SELECT valor FROM prueba WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("leer valor del backup");
        assert_eq!(valor, "original");
        drop(config);
        drop(config_dir);
    }

    #[test]
    fn backup_db_to_incluye_datos_pendientes_en_wal() {
        let (config, config_dir, data_dir) = test_config_with_tmp();
        // Desactivar el checkpoint automático para que la fila quede en el
        // archivo -wal hasta que el backup fuerce el checkpoint.
        let conn = rusqlite::Connection::open(data_dir.path().join("acryl.db"))
            .expect("abrir BD para WAL");
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA wal_autocheckpoint = 0;")
            .expect("configurar WAL");
        conn.execute("INSERT INTO prueba (valor) VALUES ('en-wal')", [])
            .expect("insertar fila en WAL");

        let backup_path = config_dir.path().join("backup-wal.db");
        config.backup_db_to(&backup_path).expect("backup con WAL");
        drop(conn);

        let copia = rusqlite::Connection::open(&backup_path).expect("abrir backup WAL");
        let count: i64 = copia
            .query_row(
                "SELECT COUNT(*) FROM prueba WHERE valor = 'en-wal'",
                [],
                |row| row.get(0),
            )
            .expect("contar fila en WAL");
        assert_eq!(count, 1, "el backup debe incluir los datos del WAL");
        drop(config);
        drop(config_dir);
        drop(data_dir);
    }

    #[test]
    fn restore_db_from_sobrescribe_bd() {
        let (config, config_dir, data_dir) = test_config_with_tmp();
        let backup_path = config_dir.path().join("backup.db");
        config.backup_db_to(&backup_path).expect("backup db");

        // Modificar la base activa.
        {
            let conn = rusqlite::Connection::open(data_dir.path().join("acryl.db"))
                .expect("abrir BD activa");
            conn.execute("UPDATE prueba SET valor = 'modificado' WHERE id = 1", [])
                .expect("modificar BD activa");
        }

        config.restore_db_from(&backup_path).expect("restaurar db");
        let restaurada = rusqlite::Connection::open(data_dir.path().join("acryl.db"))
            .expect("abrir BD restaurada");
        let valor: String = restaurada
            .query_row("SELECT valor FROM prueba WHERE id = 1", [], |row| {
                row.get(0)
            })
            .expect("leer valor restaurado");
        assert_eq!(valor, "original");
        drop(config);
        drop(config_dir);
        drop(data_dir);
    }

    #[test]
    fn restore_db_from_rechaza_archivo_inexistente() {
        let (config, _config_dir, _data_dir) = test_config_with_tmp();
        let resultado = config.restore_db_from(Path::new("/tmp/no-existe-backup-xyz.db"));
        assert!(resultado.is_err(), "debe rechazar backup inexistente");
        drop(config);
    }

    #[test]
    fn restore_db_from_rechaza_archivo_corrupto() {
        let (config, config_dir, _data_dir) = test_config_with_tmp();
        let corrupto = config_dir.path().join("corrupto.db");
        fs::write(&corrupto, b"esto no es sqlite").unwrap();
        let resultado = config.restore_db_from(&corrupto);
        assert!(resultado.is_err(), "debe rechazar una copia corrupta");
        drop(config);
        drop(config_dir);
    }

    #[test]
    fn app_config_theme_se_guarda_y_carga() {
        // El tema por defecto es oscuro.
        assert_eq!(AppConfig::default().theme, "dark");

        // Un tema claro se serializa y deserializa correctamente.
        let claro = AppConfig {
            theme: "light".to_string(),
            ..Default::default()
        };
        let yaml = serde_yaml::to_string(&claro).expect("serializar tema");
        let de_serializado: AppConfig = serde_yaml::from_str(&yaml).expect("deserializar tema");
        assert_eq!(de_serializado.theme, "light");
    }
}
