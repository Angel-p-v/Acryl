//! Servicio de descarga y caché de pósters.
//!
//! Descarga pósters desde APIs públicas y los almacena en caché en disco bajo
//! `~/.local/share/acryl/cache/posters/`. Las descargas se realizan en segundo
//! plano para no bloquear la UI de GTK4.
//!
//! Características:
//! - Límite de descargas concurrentes (semáforo) para no saturar red ni disco.
//! - Reporte de progreso por descarga (inicio, finalizado, error).
//! - Limpieza de caché de pósters huérfanos (imágenes ya no referenciadas por
//!   ningún medio de la biblioteca).

use crate::config::default_data_dir;
use reqwest::Client;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;

/// Tiempo máximo para descargar un póster.
const POSTER_TIMEOUT_SECS: u64 = 20;

/// Número máximo de descargas concurrentes por defecto.
const DEFAULT_MAX_CONCURRENT_DOWNLOADS: usize = 4;

/// Directorio de caché de pósters.
fn poster_cache_dir() -> PathBuf {
    default_data_dir().join("cache/posters")
}

/// Asegura que el directorio de caché exista.
fn ensure_cache_dir() -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let dir = poster_cache_dir();
    fs::create_dir_all(&dir)
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
    Ok(dir)
}

/// Nombre de archivo de caché para un póster dado su URL (hash + extensión).
///
/// El nombre es determinista para una misma URL, lo que permite localizar el
/// fichero cacheado sin depender del directorio base.
pub fn poster_cache_filename(poster_url: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    poster_url.hash(&mut hasher);
    let hash = hasher.finish();

    let ext = Path::new(poster_url)
        .extension()
        .and_then(|e| e.to_str())
        .filter(|ext| ext.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or("jpg");

    format!("{}.{}", hash, ext)
}

/// Ruta de caché para un póster dado su URL o identificador.
///
/// Busca en varias ubicaciones posibles para mantener compatibilidad con
/// cachés creados en diferentes versiones o entornos:
///   1. XDG_DATA_HOME/acryl/cache/posters/  (ruta principal)
///   2. XDG_DATA_HOME/cache/posters/        (ruta sin directorio acryl/)
///   3. ~/.local/share/acryl/cache/posters/ (ruta clásica sin XDG_DATA_HOME)
pub fn poster_cache_path(poster_url: &str) -> PathBuf {
    let filename = poster_cache_filename(poster_url);

    // 1. Ruta principal: XDG_DATA_HOME/acryl/cache/posters/
    let primary = poster_cache_dir().join(&filename);
    if primary.exists() {
        return primary;
    }

    // 2. Ruta alternativa: XDG_DATA_HOME/cache/posters/ (sin directorio acryl/)
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        let alt1 = PathBuf::from(xdg).join("cache/posters").join(&filename);
        if alt1.exists() {
            return alt1;
        }
    }

    // 3. Ruta clásica: ~/.local/share/acryl/cache/posters/ (sin XDG_DATA_HOME)
    if let Ok(home) = std::env::var("HOME") {
        let alt2 = PathBuf::from(home)
            .join(".local/share/acryl/cache/posters")
            .join(&filename);
        if alt2.exists() {
            return alt2;
        }
    }

    // Por defecto, devolver la ruta principal (se creará al descargar)
    primary
}

/// Comprueba si un póster ya está en caché.
pub fn is_cached(poster_url: &str) -> bool {
    Path::new(&poster_cache_path(poster_url)).exists()
}

/// Devuelve la ruta de un póster en caché, o None si no está descargado.
pub fn cached_path(poster_url: &str) -> Option<PathBuf> {
    let path = poster_cache_path(poster_url);
    path.exists().then_some(path)
}

/// Evento de progreso de una descarga de póster.
#[derive(Debug, Clone)]
pub struct ProgressEvent {
    /// URL de la imagen que se está descargando.
    pub url: String,
    /// Tipo de evento.
    pub kind: ProgressKind,
}

/// Tipo de evento de progreso.
#[derive(Debug, Clone)]
pub enum ProgressKind {
    /// La descarga comenzó.
    Started,
    /// La descarga terminó y el archivo quedó en `path`.
    Completed { path: PathBuf },
    /// La descarga falló con `error`.
    Failed { error: String },
}

/// Función que recibe eventos de progreso de una descarga.
pub type ProgressReporter = Arc<dyn Fn(&ProgressEvent) + Send + Sync>;

/// Servicio de descarga de pósters.
pub struct PosterService {
    client: Client,
    /// Semáforo que limita el número de descargas concurrentes.
    semaphore: Arc<Semaphore>,
    /// Reportero opcional de progreso.
    progress: Option<ProgressReporter>,
}

impl PosterService {
    /// Crea un nuevo servicio con el cliente HTTP por defecto y hasta
    /// `DEFAULT_MAX_CONCURRENT_DOWNLOADS` descargas concurrentes.
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(POSTER_TIMEOUT_SECS))
            .user_agent(concat!("acryl/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();
        Self {
            client,
            semaphore: Arc::new(Semaphore::new(DEFAULT_MAX_CONCURRENT_DOWNLOADS)),
            progress: None,
        }
    }

    /// Crea un servicio con un límite personalizado de descargas concurrentes.
    pub fn with_concurrency_limit(max_concurrent: usize) -> Self {
        let mut service = Self::new();
        service.semaphore = Arc::new(Semaphore::new(max_concurrent));
        service
    }

    /// Asigna un reportero de progreso. Se invoca con un evento al iniciar y
    /// al finalizar (con éxito o error) cada descarga de este servicio.
    pub fn with_progress_reporter(mut self, reporter: ProgressReporter) -> Self {
        self.progress = Some(reporter);
        self
    }

    /// Notifica un evento de progreso si hay reportero configurado.
    fn report(&self, url: &str, kind: ProgressKind) {
        if let Some(reporter) = &self.progress {
            reporter(&ProgressEvent {
                url: url.to_string(),
                kind,
            });
        }
    }

    /// Descarga un póster de forma asíncrona usando tokio.
    /// Diseñada para llamarse desde tokio::spawn sin bloquear la UI.
    ///
    /// El semáforo del servicio limita cuántas descargas se ejecutan a la vez,
    /// evitando saturar la red y el disco cuando varias tarjetas piden pósters
    /// simultáneamente.
    pub async fn download_async(
        &self,
        poster_url: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        self.download_async_impl(poster_url, false).await
    }

    /// Fuerza la descarga de un póster aunque ya exista en caché.
    /// Sobrescribe el archivo cacheado con la versión actual del servidor.
    pub async fn download_async_force(
        &self,
        poster_url: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        self.download_async_impl(poster_url, true).await
    }

    async fn download_async_impl(
        &self,
        poster_url: &str,
        force: bool,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        // Limitar el número de descargas concurrentes.
        let _permit = self.semaphore.acquire().await?;

        // Si ya está en caché y no se fuerza la descarga, devolver la ruta existente.
        if !force && let Some(cached) = cached_path(poster_url) {
            self.report(
                poster_url,
                ProgressKind::Completed {
                    path: cached.clone(),
                },
            );
            return Ok(cached);
        }

        self.report(poster_url, ProgressKind::Started);

        ensure_cache_dir()?;

        let response = self.client.get(poster_url).send().await?;
        if !response.status().is_success() {
            let error = format!("HTTP {} descargando {}", response.status(), poster_url);
            self.report(
                poster_url,
                ProgressKind::Failed {
                    error: error.clone(),
                },
            );
            return Err(error.into());
        }

        let bytes = response.bytes().await?;

        // Escribir primero en un archivo temporal único y renombrarlo al final
        // evita que otra lectura vea una imagen a medio escribir (el selector y
        // la vista de detalles pueden pedir la misma URL en paralelo).
        let cache_path = poster_cache_path(poster_url);
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_nanos())
            .unwrap_or_default();
        let file_name = cache_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("poster");
        let temp_path = cache_path.with_file_name(format!(
            "{}.{}.{}.tmp",
            file_name,
            std::process::id(),
            unique
        ));

        if let Err(error) = fs::write(&temp_path, &*bytes) {
            let _ = fs::remove_file(&temp_path);
            let error = format!("error escribiendo caché de {}: {}", poster_url, error);
            self.report(
                poster_url,
                ProgressKind::Failed {
                    error: error.clone(),
                },
            );
            return Err(error.into());
        }
        if let Err(error) = fs::rename(&temp_path, &cache_path) {
            let _ = fs::remove_file(&temp_path);
            let error = format!("error renombrando caché de {}: {}", poster_url, error);
            self.report(
                poster_url,
                ProgressKind::Failed {
                    error: error.clone(),
                },
            );
            return Err(error.into());
        }

        self.report(
            poster_url,
            ProgressKind::Completed {
                path: cache_path.clone(),
            },
        );

        Ok(cache_path)
    }
}

/// Elimina de la caché el póster asociado a una URL.
///
/// Devuelve `Ok(true)` si se eliminó, `Ok(false)` si no existía.
pub fn remove_cached(poster_url: &str) -> std::io::Result<bool> {
    match fs::remove_file(poster_cache_path(poster_url)) {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

/// Limpia el caché de pósters eliminando archivos más antiguos que
/// `max_age_days`.
pub fn clean_cache(max_age_days: u64) -> Result<usize, Box<dyn std::error::Error>> {
    let dir = poster_cache_dir();
    if !dir.exists() {
        return Ok(0);
    }

    let cutoff =
        std::time::SystemTime::now() - std::time::Duration::from_secs(max_age_days * 86400);
    let mut removed = 0usize;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if let Ok(modified) = metadata.modified()
            && modified < cutoff
        {
            fs::remove_file(entry.path())?;
            removed += 1;
        }
    }

    Ok(removed)
}

/// Elimina del caché de pósters (en `dir`) los archivos que ya no están
/// referenciados por ninguna URL dada (pósters/banners de medios o imágenes
/// de episodio borradas de la biblioteca).
///
/// `referenced_urls` son las URLs de imagen aún en uso: `poster_path` y
/// `banner_path` de la tabla `media`, y `episode_image_path` de
/// `media_files`. El llamador es responsable de recopilarlas desde el
/// repositorio, de modo que este servicio no dependa de la capa de BD.
///
/// Devuelve el número de archivos eliminados.
pub fn clean_orphaned(
    dir: &Path,
    referenced_urls: &[&str],
) -> Result<usize, Box<dyn std::error::Error>> {
    if !dir.exists() {
        return Ok(0);
    }

    // Rutas de caché esperadas para todas las URLs aún referenciadas, dentro
    // de `dir` (no usando poster_cache_path, que apunta al directorio global).
    let referenced: HashSet<PathBuf> = referenced_urls
        .iter()
        .map(|url| dir.join(poster_cache_filename(url)))
        .collect();

    let mut removed = 0usize;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && !referenced.contains(&path) {
            fs::remove_file(&path)?;
            removed += 1;
        }
    }

    Ok(removed)
}

/// Wrapper de conveniencia que limpia el directorio de caché por defecto
/// (`~/.local/share/acryl/cache/posters/`).
pub fn clean_orphaned_default(
    referenced_urls: &[&str],
) -> Result<usize, Box<dyn std::error::Error>> {
    clean_orphaned(&poster_cache_dir(), referenced_urls)
}

impl Default for PosterService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poster_cache_path_es_deterministico() {
        let url = "https://example.com/poster.jpg";
        let path1 = poster_cache_path(url);
        let path2 = poster_cache_path(url);
        assert_eq!(path1, path2);
        assert!(path1.extension().unwrap() == "jpg");
    }

    #[test]
    fn poster_cache_path_diferente_para_urls_diferentes() {
        let path1 = poster_cache_path("https://example.com/a.jpg");
        let path2 = poster_cache_path("https://example.com/b.jpg");
        assert_ne!(path1, path2);
    }

    #[test]
    fn poster_cache_path_sanea_extensiones_no_seguras() {
        let path = poster_cache_path("https://example.com/poster.jpg?token=abc");
        assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("jpg"));
    }

    #[test]
    fn cached_path_devuelve_none_si_no_existe() {
        assert!(cached_path("https://example.com/inexistente.jpg").is_none());
    }

    #[test]
    fn poster_service_nuevo_sin_panic() {
        let _service = PosterService::new();
    }

    #[test]
    fn poster_cache_path_usa_jpg_cuando_no_hay_extension() {
        let path = poster_cache_path("https://example.com/sin-extension");
        assert_eq!(path.extension().and_then(|ext| ext.to_str()), Some("jpg"));
    }

    #[test]
    fn is_cached_es_falso_para_urls_sin_descarga() {
        assert!(!is_cached(
            "https://example.com/nunca-descargado-123456.jpg"
        ));
    }

    #[test]
    fn remove_cached_devuelve_falso_si_no_existe() {
        let removed = remove_cached("https://example.com/nunca-descargado-654321.jpg")
            .expect("eliminar de caché");
        assert!(!removed);
    }

    #[test]
    fn poster_service_default_equivale_a_new() {
        let _service = PosterService::default();
    }

    // -----------------------------------------------------------------------
    // Límite de concurrencia
    // -----------------------------------------------------------------------

    #[test]
    fn limite_de_concurrencia_predeterminado_es_mayor_que_uno() {
        let service = PosterService::new();
        assert_eq!(
            service.semaphore.available_permits(),
            DEFAULT_MAX_CONCURRENT_DOWNLOADS
        );
    }

    #[test]
    fn limite_de_concurrencia_personalizado() {
        let service = PosterService::with_concurrency_limit(1);
        assert_eq!(service.semaphore.available_permits(), 1);
    }

    // -----------------------------------------------------------------------
    // Limpieza de huérfanos
    // -----------------------------------------------------------------------

    #[test]
    fn clean_orphaned_elimina_solo_archivos_no_referenciados() {
        let dir = tempfile::tempdir().expect("crear directorio temporal");
        let ref_url = "https://example.com/referenciado.jpg";
        let orphan_url = "https://example.com/huerfano.jpg";
        let ref_path = dir
            .path()
            .join(poster_cache_path(ref_url).file_name().unwrap());
        let orphan_path = dir
            .path()
            .join(poster_cache_path(orphan_url).file_name().unwrap());

        let _ = fs::write(&ref_path, b"fake-referenced");
        let _ = fs::write(&orphan_path, b"fake-orphan");

        let removed = clean_orphaned(dir.path(), &[ref_url]).expect("limpiar huérfanos");
        assert_eq!(removed, 1, "debe eliminar solo el huérfano");
        assert!(!orphan_path.exists(), "el huérfano debe haberse eliminado");
        assert!(ref_path.exists(), "el referenciado debe conservarse");
    }

    #[test]
    fn clean_orphaned_sin_cache_dir_devuelve_cero() {
        let dir = tempfile::tempdir().expect("crear directorio temporal");
        let removed = clean_orphaned(dir.path(), &[]).expect("limpiar huérfanos vacíos");
        assert_eq!(removed, 0);
    }

    #[test]
    fn clean_orphaned_todas_referenciadas_elimina_nada() {
        let dir = tempfile::tempdir().expect("crear directorio temporal");
        let url_a = "https://example.com/orphan-test-a.jpg";
        let url_b = "https://example.com/orphan-test-b.jpg";
        let path_a = dir
            .path()
            .join(poster_cache_path(url_a).file_name().unwrap());
        let path_b = dir
            .path()
            .join(poster_cache_path(url_b).file_name().unwrap());
        let _ = fs::write(&path_a, b"a");
        let _ = fs::write(&path_b, b"b");

        let removed = clean_orphaned(dir.path(), &[url_a, url_b]).expect("limpiar huérfanos");
        assert_eq!(
            removed, 0,
            "no debe eliminar nada si todo está referenciado"
        );
        assert!(path_a.exists());
        assert!(path_b.exists());
    }
}
