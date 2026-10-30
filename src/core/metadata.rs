//! Servicio de metadatos: sincronización con TVmaze.

use crate::db::{MediaRepository, close_db, open_db};
use reqwest::Client;

/// Resultado de sincronización de metadatos.
#[derive(Debug, Clone)]
pub struct SyncMetadataResult {
    pub title: Option<String>,
    pub synopsis: Option<String>,
    pub poster_path: Option<String>,
    pub release_date: Option<String>,
    pub vote_average: Option<f64>,
}

/// Servicio de metadatos: sincroniza con TVmaze.
pub struct MetadataService;

impl MetadataService {
    /// Sincroniza metadatos de un medio desde TVmaze.
    pub async fn sync_metadata(
        db_path: &str,
        media_id: i64,
    ) -> Result<SyncMetadataResult, Box<dyn std::error::Error>> {
        let mut conn = open_db(db_path)?;
        let mut media_repo = MediaRepository::new(&mut conn);

        // Obtener el medio para saber el título.
        let media = media_repo
            .find_by_id(media_id)?
            .ok_or_else(|| format!("medio no encontrado: id={}", media_id))?;

        let client = TvmazeClient::new();
        let fetched = sync_metadata(&media.r#type, &media.title, &client).await?;

        media_repo.update_metadata(
            media_id,
            fetched.poster_path.as_deref(),
            fetched.synopsis.as_deref(),
        )?;

        close_db(conn);

        Ok(fetched)
    }
}

/// Cliente TVmaze.
#[derive(Clone)]
pub struct TvmazeClient {
    client: reqwest::Client,
}

impl TvmazeClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
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
    // TVmaze API: busca por título
    let url = format!(
        "https://api.tvmaze.com/search/shows?q={}",
        urlencoding::encode(title)
    );
    let resp: serde_json::Value = client.client.get(&url).send().await?.json().await?;

    if let Some(results) = resp.as_array()
        && let Some(first) = results.first()
        && let Some(show) = first.get("show")
    {
        let title = show
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let synopsis = show
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let poster_path = show
            .get("image")
            .and_then(|v| v.get("medium"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let release_date = show
            .get("premiered")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let vote_average = show
            .get("rating")
            .and_then(|v| v.get("average"))
            .and_then(|v| v.as_f64());

        return Ok(SyncMetadataResult {
            title,
            synopsis,
            poster_path,
            release_date,
            vote_average,
        });
    }

    Ok(SyncMetadataResult {
        title: None,
        synopsis: None,
        poster_path: None,
        release_date: None,
        vote_average: None,
    })
}
