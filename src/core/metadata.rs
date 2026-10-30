//! Servicio de metadatos: sincronización con TVmaze.
//!
//! TVmaze es una API pública que no requiere clave. Se usa para completar
//! sinopsis, año y URL del póster de series y anime. La descarga del póster
//! se delega en `PosterService`, que mantiene una caché local.

use crate::core::poster_service::{PosterService, is_cached};
use crate::db::media_repo::ListMediaOptions;
use crate::db::{MediaRepository, close_db, open_db};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

const TVMAZE_TIMEOUT_SECS: u64 = 20;

/// Resultado de sincronización de metadatos.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SyncMetadataResult {
    pub title: Option<String>,
    pub synopsis: Option<String>,
    pub poster_path: Option<String>,
    pub release_date: Option<String>,
    pub vote_average: Option<f64>,
}

/// Resumen de una actualización forzada de toda la biblioteca.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefreshMetadataResult {
    pub total: usize,
    pub updated: usize,
    pub failed: usize,
    pub poster_failed: usize,
}

#[derive(Debug, Clone, Default)]
struct SyncOutcome {
    metadata: SyncMetadataResult,
    poster_failed: bool,
}

/// Servicio de metadatos: sincroniza con TVmaze.
pub struct MetadataService;

impl MetadataService {
    /// Sincroniza los metadatos de un medio desde TVmaze y descarga su póster.
    ///
    /// La conexión a SQLite no se mantiene abierta durante la petición HTTP.
    pub async fn sync_metadata(
        db_path: &str,
        media_id: i64,
    ) -> Result<SyncMetadataResult, Box<dyn std::error::Error>> {
        Ok(Self::sync_metadata_impl(db_path, media_id, false)
            .await?
            .metadata)
    }

    async fn sync_metadata_impl(
        db_path: &str,
        media_id: i64,
        force: bool,
    ) -> Result<SyncOutcome, Box<dyn std::error::Error>> {
        let media = {
            let mut conn = open_db(db_path)?;
            let mut media_repo = MediaRepository::new(&mut conn);
            let media = media_repo
                .find_by_id(media_id)?
                .ok_or_else(|| format!("medio no encontrado: id={}", media_id))?;
            close_db(conn);
            media
        };

        let client = TvmazeClient::new();
        let fetched = sync_metadata(&media.r#type, &media.title, &client).await?;

        // Al forzar, un resultado vacío no debe borrar los metadatos existentes.
        if force && fetched.title.is_none() {
            return Err(format!("TVmaze no devolvió resultados para '{}'", media.title).into());
        }

        let year = fetched.release_date.as_deref().and_then(parse_year);
        let new_title = fetched
            .title
            .as_deref()
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .unwrap_or(media.title.as_str());
        let sort_title = new_title.to_lowercase();

        let mut poster_path = fetched.poster_path.clone();
        let mut poster_failed = false;

        if let Some(poster_url) = poster_path.as_deref().filter(|url| !url.is_empty()) {
            let poster_service = PosterService::new();
            let download = if force {
                poster_service.download_async_force(poster_url).await
            } else {
                poster_service.download_async(poster_url).await
            };

            match download {
                Ok(_) => {
                    if force
                        && let Some(old_url) = media.poster_path.as_deref()
                        && old_url != poster_url
                        && let Err(error) = crate::core::poster_service::remove_cached(old_url)
                    {
                        eprintln!(
                            "No se pudo eliminar el póster antiguo de '{}': {}",
                            media.title, error
                        );
                    }
                }
                Err(error) => {
                    poster_failed = true;
                    eprintln!(
                        "No se pudo descargar el póster de '{}': {}",
                        media.title, error
                    );
                    // Al forzar, si el póster nuevo falla, se conserva el anterior.
                    if force {
                        poster_path = media.poster_path.clone();
                    }
                }
            }
        } else if force {
            // TVmaze no ofrece póster: conservar el que ya hubiera.
            poster_path = media.poster_path.clone();
        }

        {
            let mut conn = open_db(db_path)?;
            let mut media_repo = MediaRepository::new(&mut conn);
            if force {
                media_repo.overwrite_metadata(
                    media_id,
                    new_title,
                    &sort_title,
                    year,
                    poster_path.as_deref(),
                    fetched.synopsis.as_deref(),
                )?;
            } else {
                media_repo.update_metadata(
                    media_id,
                    year,
                    poster_path.as_deref(),
                    fetched.synopsis.as_deref(),
                )?;
            }
            close_db(conn);
        }

        Ok(SyncOutcome {
            metadata: fetched,
            poster_failed,
        })
    }

    /// Sincroniza los medios que aún no tienen metadatos y descarga los pósters
    /// que falten en la caché. Pensada para ejecutarse en un hilo secundario.
    ///
    /// Devuelve cuántos medios se han actualizado.
    pub fn sync_missing_metadata(
        db_path: &str,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let media = load_all_parent_media(db_path)?;

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        let mut changed = 0usize;
        for item in media {
            if item.metadata_fetched_at.is_none() {
                match runtime.block_on(Self::sync_metadata(db_path, item.id)) {
                    Ok(_) => changed += 1,
                    Err(error) => eprintln!(
                        "No se pudieron obtener metadatos de '{}': {}",
                        item.title, error
                    ),
                }
                continue;
            }

            if let Some(poster_url) = item.poster_path.as_deref().filter(|url| !url.is_empty())
                && !is_cached(poster_url)
            {
                match runtime.block_on(PosterService::new().download_async(poster_url)) {
                    Ok(_) => changed += 1,
                    Err(error) => eprintln!(
                        "No se pudo descargar el póster de '{}': {}",
                        item.title, error
                    ),
                }
            }
        }

        Ok(changed)
    }

    /// Vuelve a consultar TVmaze y sobrescribe título, año, sinopsis y póster
    /// de todos los medios padre indexados. Pensada para ejecutarse en un hilo
    /// secundario y no bloquear la UI.
    pub fn refresh_all_metadata(
        db_path: &str,
    ) -> Result<RefreshMetadataResult, Box<dyn std::error::Error + Send + Sync>> {
        let media = load_all_parent_media(db_path)?;

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        let mut result = RefreshMetadataResult {
            total: media.len(),
            ..Default::default()
        };

        for item in media {
            match runtime.block_on(Self::sync_metadata_impl(db_path, item.id, true)) {
                Ok(outcome) => {
                    result.updated += 1;
                    if outcome.poster_failed {
                        result.poster_failed += 1;
                    }
                }
                Err(error) => {
                    result.failed += 1;
                    eprintln!(
                        "No se pudieron actualizar los metadatos de '{}': {}",
                        item.title, error
                    );
                }
            }
        }

        Ok(result)
    }
}

fn load_all_parent_media(
    db_path: &str,
) -> Result<Vec<crate::db::media_repo::Media>, Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = open_db(db_path)?;
    let mut media_repo = MediaRepository::new(&mut conn);
    let media = media_repo.list(ListMediaOptions {
        limit: Some(i64::MAX),
        ..Default::default()
    })?;
    close_db(conn);
    Ok(media)
}

/// Cliente TVmaze.
#[derive(Clone)]
pub struct TvmazeClient {
    client: reqwest::Client,
}

impl TvmazeClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(TVMAZE_TIMEOUT_SECS))
            .user_agent(concat!("acryl/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default();
        Self { client }
    }
}

impl Default for TvmazeClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Busca metadatos en TVmaze.
pub async fn sync_metadata(
    _media_type: &str,
    title: &str,
    client: &TvmazeClient,
) -> Result<SyncMetadataResult, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.tvmaze.com/search/shows?q={}",
        urlencoding::encode(title)
    );
    let response: Value = client
        .client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(parse_search_response(&response))
}

fn parse_search_response(response: &Value) -> SyncMetadataResult {
    let Some(show) = response
        .as_array()
        .and_then(|results| results.first())
        .and_then(|first| first.get("show"))
    else {
        return SyncMetadataResult::default();
    };

    SyncMetadataResult {
        title: show.get("name").and_then(Value::as_str).map(str::to_string),
        synopsis: show.get("summary").and_then(Value::as_str).map(clean_html),
        poster_path: show
            .get("image")
            .and_then(|image| image.get("medium"))
            .and_then(Value::as_str)
            .map(str::to_string),
        release_date: show
            .get("premiered")
            .and_then(Value::as_str)
            .map(str::to_string),
        vote_average: show
            .get("rating")
            .and_then(|rating| rating.get("average"))
            .and_then(Value::as_f64),
    }
}

fn parse_year(date: &str) -> Option<i32> {
    date.get(0..4)?.parse().ok()
}

/// Elimina el HTML de las sinopsis de TVmaze y decodifica las entidades más
/// habituales. La UI usa `Label::set_label`, por lo que no debe recibir markup.
fn clean_html(input: &str) -> String {
    let mut text = String::with_capacity(input.len());
    let mut inside_tag = false;

    for c in input.chars() {
        match c {
            '<' => inside_tag = true,
            '>' => {
                inside_tag = false;
                text.push(' ');
            }
            _ if !inside_tag => text.push(c),
            _ => {}
        }
    }

    let text = text
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");

    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_search_response_extrae_campos() {
        let response = json!([{
            "score": 1.0,
            "show": {
                "name": "Solo Leveling",
                "summary": "<p>Hola &amp; adiós</p>",
                "image": {"medium": "https://example.com/poster.jpg"},
                "premiered": "2024-01-06",
                "rating": {"average": 8.0}
            }
        }]);

        let parsed = parse_search_response(&response);
        assert_eq!(parsed.title.as_deref(), Some("Solo Leveling"));
        assert_eq!(parsed.synopsis.as_deref(), Some("Hola & adiós"));
        assert_eq!(
            parsed.poster_path.as_deref(),
            Some("https://example.com/poster.jpg")
        );
        assert_eq!(parsed.release_date.as_deref(), Some("2024-01-06"));
        assert_eq!(parsed.vote_average, Some(8.0));
    }

    #[test]
    fn parse_search_response_vacia_devuelve_valores_por_defecto() {
        assert_eq!(
            parse_search_response(&json!([])),
            SyncMetadataResult::default()
        );
        assert_eq!(
            parse_search_response(&json!({"error": "sin resultados"})),
            SyncMetadataResult::default()
        );
    }

    #[test]
    fn clean_html_quita_etiquetas_y_entidades() {
        let input = "<p>Una serie&nbsp;de   acción &amp; aventura.</p><br/><p>Segunda línea</p>";
        assert_eq!(
            clean_html(input),
            "Una serie de acción & aventura. Segunda línea"
        );
    }

    #[test]
    fn parse_year_extrae_el_anio() {
        assert_eq!(parse_year("2024-01-06"), Some(2024));
        assert_eq!(parse_year("2026"), Some(2026));
        assert_eq!(parse_year("sin fecha"), None);
        assert_eq!(parse_year(""), None);
    }

    #[test]
    fn sync_missing_metadata_sin_medios_no_hace_peticiones() {
        let dir = tempfile::TempDir::new().expect("crear directorio temporal");
        let db_path = dir.path().join("test.db");
        let db_path = db_path.to_str().expect("ruta UTF-8");

        {
            let conn = open_db(db_path).expect("abrir BD");
            close_db(conn);
        }

        let updated =
            MetadataService::sync_missing_metadata(db_path).expect("sincronizar biblioteca vacía");
        assert_eq!(updated, 0);
    }

    #[test]
    fn refresh_all_metadata_sin_medios_devuelve_resumen_vacio() {
        let dir = tempfile::TempDir::new().expect("crear directorio temporal");
        let db_path = dir.path().join("test.db");
        let db_path = db_path.to_str().expect("ruta UTF-8");

        {
            let conn = open_db(db_path).expect("abrir BD");
            close_db(conn);
        }

        let result =
            MetadataService::refresh_all_metadata(db_path).expect("actualizar biblioteca vacía");
        assert_eq!(
            result,
            RefreshMetadataResult {
                total: 0,
                updated: 0,
                failed: 0,
                poster_failed: 0,
            }
        );
    }
}
