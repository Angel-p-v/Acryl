//! Servicio de descarga y caché de pósters.
//!
//! Descarga pósters desde APIs públicas (listadas en https://github.com/public-apis/public-apis)
//! y los almacena en caché en disco bajo `~/.local/share/acryl/cache/posters/`.
//! Las descargas se realizan en segundo plano para no bloquear la UI de GTK4.

use crate::config::default_data_dir;
use reqwest::Client;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Tiempo máximo para descargar un póster.
const POSTER_TIMEOUT_SECS: u64 = 20;

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

/// Ruta de caché para un póster dado su URL o identificador.
///
/// Busca en varias ubicaciones posibles para mantener compatibilidad con
/// cachés creados en diferentes versiones o entornos:
///   1. XDG_DATA_HOME/acryl/cache/posters/  (ruta principal)
///   2. XDG_DATA_HOME/cache/posters/        (ruta sin directorio acryl/)
///   3. ~/.local/share/acryl/cache/posters/ (ruta clásica sin XDG_DATA_HOME)
pub fn poster_cache_path(poster_url: &str) -> PathBuf {
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

    let filename = format!("{}.{}", hash, ext);

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

/// Servicio de descarga de pósters.
pub struct PosterService {
    client: Client,
}

impl PosterService {
    /// Crea un nuevo servicio con el cliente HTTP por defecto.
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(POSTER_TIMEOUT_SECS))
            .user_agent(concat!("acryl/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();
        Self { client }
    }

    /// Descarga un póster de forma asíncrona usando tokio.
    /// Diseñada para llamarse desde tokio::spawn sin bloquear la UI.
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
        // Si ya está en caché y no se fuerza la descarga, devolver la ruta existente.
        if !force && let Some(cached) = cached_path(poster_url) {
            return Ok(cached);
        }

        ensure_cache_dir()?;

        let response = self.client.get(poster_url).send().await?;
        if !response.status().is_success() {
            return Err(format!("HTTP {} descargando {}", response.status(), poster_url).into());
        }

        let bytes = response.bytes().await?;

        let cache_path = poster_cache_path(poster_url);
        fs::write(&cache_path, &*bytes)?;

        Ok(cache_path)
    }
}

impl Default for PosterService {
    fn default() -> Self {
        Self::new()
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

/// Limpia el caché de pósters eliminando archivos más antiguos que max_age_days.
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
}
