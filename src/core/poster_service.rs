//! Servicio de descarga y caché de pósters.
//!
//! Descarga pósters desde APIs públicas (listadas en https://github.com/public-apis/public-apis)
//! y los almacena en caché en disco bajo ~/.cache/acryl/posters/.
//! Las descargas se realizan en segundo plano para no bloquear la UI de GTK4.

use crate::config::default_data_dir;
use reqwest::Client;
use std::fs;
use std::path::{Path, PathBuf};

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
pub fn poster_cache_path(poster_url: &str) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    poster_url.hash(&mut hasher);
    let hash = hasher.finish();

    let ext = Path::new(poster_url)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("jpg");

    poster_cache_dir().join(format!("{}.{}", hash, ext))
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
        Self {
            client: Client::new(),
        }
    }

    /// Descarga un póster de forma asíncrona usando tokio.
    /// Diseñada para llamarse desde tokio::spawn sin bloquear la UI.
    pub async fn download_async(
        &self,
        poster_url: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        // Si ya está en caché, devolver la ruta existente.
        if let Some(cached) = cached_path(poster_url) {
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
    fn cached_path_devuelve_none_si_no_existe() {
        assert!(cached_path("https://example.com/inexistente.jpg").is_none());
    }

    #[test]
    fn poster_service_nuevo_sin_panic() {
        let _service = PosterService::new();
    }
}
