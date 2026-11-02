//! Servicio de metadatos: sincronización con TVmaze.
//!
//! TVmaze es una API pública que no requiere clave. Se usa para completar
//! sinopsis, año y URL del póster de series y anime. La descarga del póster
//! se delega en `PosterService`, que mantiene una caché local.

use crate::core::poster_service::{PosterService, is_cached};
use crate::db::file_repo::{FileRepository, MediaFile};
use crate::db::media_repo::ListMediaOptions;
use crate::db::{MediaRepository, close_db, open_db};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

const TVMAZE_TIMEOUT_SECS: u64 = 20;

/// Resultado de sincronización de metadatos.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SyncMetadataResult {
    pub title: Option<String>,
    pub synopsis: Option<String>,
    pub poster_path: Option<String>,
    pub banner_path: Option<String>,
    pub release_date: Option<String>,
    pub vote_average: Option<f64>,
    /// Géneros/categorías devueltos por la fuente de metadatos.
    pub genres: Vec<String>,
    /// Identificador del show en TVmaze, usado para pedir sus imágenes anchas.
    pub show_id: Option<u64>,
}

/// Resumen de una actualización forzada de toda la biblioteca.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefreshMetadataResult {
    pub total: usize,
    pub updated: usize,
    pub failed: usize,
    pub poster_failed: usize,
    pub banner_failed: usize,
}

#[derive(Debug, Clone, Default)]
struct SyncOutcome {
    metadata: SyncMetadataResult,
    poster_failed: bool,
    banner_failed: bool,
}

/// Episodio tal y como lo devuelve TVmaze, con su imagen individual.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TvmazeEpisode {
    pub season: Option<i32>,
    pub number: Option<i32>,
    pub image_url: Option<String>,
}

/// Una imagen disponible para póster o banner, obtenida de la API de metadatos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageOption {
    /// Tipo de imagen: "poster", "banner" o "background".
    pub image_type: String,
    /// Proveedor de la imagen: "TVmaze", "Fanart.tv", etc.
    pub provider: String,
    /// Resolución: siempre "original".
    pub resolution: String,
    /// URL de la imagen.
    pub url: String,
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

        let mut banner_path = fetched.banner_path.clone();
        let mut banner_failed = false;

        if let Some(banner_url) = banner_path.as_deref().filter(|url| !url.is_empty()) {
            let poster_service = PosterService::new();
            let download = if force {
                poster_service.download_async_force(banner_url).await
            } else {
                poster_service.download_async(banner_url).await
            };

            match download {
                Ok(_) => {
                    if force
                        && let Some(old_url) = media.banner_path.as_deref()
                        && old_url != banner_url
                        && let Err(error) = crate::core::poster_service::remove_cached(old_url)
                    {
                        eprintln!(
                            "No se pudo eliminar el banner antiguo de '{}': {}",
                            media.title, error
                        );
                    }
                }
                Err(error) => {
                    banner_failed = true;
                    eprintln!(
                        "No se pudo descargar el banner de '{}': {}",
                        media.title, error
                    );
                    // Al forzar, si el banner nuevo falla, se conserva el anterior.
                    if force {
                        banner_path = media.banner_path.clone();
                    }
                }
            }
        } else if force {
            // Sin banner nuevo: conservar el que ya hubiera.
            banner_path = media.banner_path.clone();
        }

        {
            let mut conn = open_db(db_path)?;
            let mut media_repo = MediaRepository::new(&mut conn);
            if force {
                media_repo.overwrite_metadata(
                    media_id,
                    &crate::db::media_repo::MediaMetadataInput {
                        title: new_title,
                        sort_title: &sort_title,
                        year,
                        poster_path: poster_path.as_deref(),
                        banner_path: banner_path.as_deref(),
                        synopsis: fetched.synopsis.as_deref(),
                    },
                )?;
            } else {
                media_repo.update_metadata(
                    media_id,
                    year,
                    poster_path.as_deref(),
                    banner_path.as_deref(),
                    fetched.synopsis.as_deref(),
                )?;
            }

            // Persistir la puntuación obtenida de los metadatos.
            if let Err(error) = media_repo.set_vote_average(media_id, fetched.vote_average) {
                eprintln!(
                    "No se pudo guardar la puntuación de '{}': {}",
                    media.title, error
                );
            }

            // Guardar el identificador del show para pedir sus episodios sin
            // repetir la búsqueda por título.
            if let Some(show_id) = fetched.show_id
                && let Err(error) =
                    media_repo.set_metadata_source_id(media_id, Some(show_id as i64))
            {
                eprintln!(
                    "No se pudo guardar el origen de metadatos de '{}': {}",
                    media.title, error
                );
            }

            // Extraer, normalizar y asociar categorías (géneros) al medio,
            // evitando duplicados. Un medio sin categorías es válido.
            if !fetched.genres.is_empty() {
                let mut category_ids = Vec::new();
                for genre in &fetched.genres {
                    if let Ok(Some(category_id)) = media_repo.get_or_create_category(genre) {
                        category_ids.push(category_id);
                    }
                }
                if let Err(error) = media_repo.set_media_categories(media_id, &category_ids) {
                    eprintln!(
                        "No se pudieron asociar las categorías de '{}': {}",
                        media.title, error
                    );
                }
            }

            // Marcar la comprobación de categorías aunque la fuente no las
            // ofrezca, para no repetir la petición en cada arranque.
            if let Err(error) = media_repo.mark_categories_fetched(media_id) {
                eprintln!(
                    "No se pudo marcar la extracción de categorías de '{}': {}",
                    media.title, error
                );
            }
            close_db(conn);
        }

        Ok(SyncOutcome {
            metadata: fetched,
            poster_failed,
            banner_failed,
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
        let client = TvmazeClient::new();

        let mut changed = 0usize;
        for item in media {
            let needs_metadata = item.metadata_fetched_at.is_none();
            let needs_categories = item.categories_fetched_at.is_none();

            // Se consulta la fuente si faltan metadatos o si la biblioteca es
            // anterior a las categorías. Así se rellenan géneros existentes
            // sin repetir la petición en cada arranque.
            if needs_metadata || needs_categories {
                match runtime.block_on(Self::sync_metadata(db_path, item.id)) {
                    Ok(_) => changed += 1,
                    Err(error) => eprintln!(
                        "No se pudieron obtener metadatos de '{}': {}",
                        item.title, error
                    ),
                }
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

            if let Some(banner_url) = item.banner_path.as_deref().filter(|url| !url.is_empty())
                && !is_cached(banner_url)
            {
                match runtime.block_on(PosterService::new().download_async(banner_url)) {
                    Ok(_) => changed += 1,
                    Err(error) => eprintln!(
                        "No se pudo descargar el banner de '{}': {}",
                        item.title, error
                    ),
                }
            }

            match runtime.block_on(Self::sync_episode_images(db_path, item.id, &client)) {
                Ok(updated) => changed += updated,
                Err(error) => eprintln!(
                    "No se pudieron obtener imágenes de episodios de '{}': {}",
                    item.title, error
                ),
            }
        }

        Ok(changed)
    }

    /// Obtiene y cachea la imagen individual de cada episodio del show.
    ///
    /// Solo consulta la fuente si hay archivos pendientes. Un episodio sin
    /// imagen en TVmaze se marca como comprobado y la UI usa el fallback.
    async fn sync_episode_images(
        db_path: &str,
        media_id: i64,
        client: &TvmazeClient,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let (show_id, files) = {
            let mut conn = open_db(db_path)?;
            let mut media_repo = MediaRepository::new(&mut conn);
            let media = media_repo
                .find_by_id(media_id)?
                .ok_or_else(|| format!("medio no encontrado: id={}", media_id))?;
            let files = media_repo.list_files_for_show(media_id)?;
            close_db(conn);
            (media.metadata_source_id, files)
        };

        let Some(show_id) = show_id else {
            return Ok(0);
        };
        let pending: Vec<_> = files
            .iter()
            .filter(|file| file.episode_image_checked_at.is_none())
            .collect();
        if pending.is_empty() {
            return Ok(0);
        }

        let Some(episodes) = fetch_tvmaze_episodes(show_id as u64, client).await else {
            return Ok(0);
        };

        let poster_service = PosterService::new();
        let mut updated = 0usize;

        for file in pending {
            let Some((season, episode)) = episode_position(file) else {
                mark_episode_checked(db_path, &file.file_path)?;
                continue;
            };
            let Some(image_url) = find_episode_image(&episodes, season, episode) else {
                mark_episode_checked(db_path, &file.file_path)?;
                continue;
            };

            match poster_service.download_async(&image_url).await {
                Ok(_) => {
                    let mut conn = open_db(db_path)?;
                    let mut media_repo = MediaRepository::new(&mut conn);
                    if let Err(error) = media_repo.set_episode_image(&file.file_path, &image_url) {
                        eprintln!(
                            "No se pudo guardar la imagen de '{}': {}",
                            file.file_path, error
                        );
                    } else {
                        updated += 1;
                    }
                    close_db(conn);
                }
                Err(error) => {
                    // Sin marcar: se reintenta en la próxima sincronización.
                    eprintln!(
                        "No se pudo descargar la imagen del episodio '{}': {}",
                        file.file_path, error
                    );
                }
            }
        }

        Ok(updated)
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
        let client = TvmazeClient::new();

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
                    if outcome.banner_failed {
                        result.banner_failed += 1;
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

            if let Err(error) =
                runtime.block_on(Self::sync_episode_images(db_path, item.id, &client))
            {
                eprintln!(
                    "No se pudieron actualizar las imágenes de episodios de '{}': {}",
                    item.title, error
                );
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

/// Busca metadatos en TVmaze y, además, una imagen ancha para el banner.
///
/// Para anime se consulta primero AniList y, si no hay banner, Kitsu; en el
/// resto de casos se usan las imágenes `background`/`banner` de TVmaze. La
/// ausencia de banner no es un error: la vista de detalles usa un degradado.
pub async fn sync_metadata(
    media_type: &str,
    title: &str,
    client: &TvmazeClient,
) -> Result<SyncMetadataResult, Box<dyn std::error::Error>> {
    let mut metadata = SyncMetadataResult::default();

    for candidate in search_candidates(media_type, title) {
        let url = format!(
            "https://api.tvmaze.com/search/shows?q={}",
            urlencoding::encode(&candidate)
        );
        let response: Value = client
            .client
            .get(&url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let parsed = parse_search_response(&response);
        if parsed.title.is_some() {
            metadata = parsed;
            break;
        }
    }

    metadata.banner_path = fetch_banner_url(media_type, title, metadata.show_id, client).await;
    Ok(metadata)
}

/// Genera variantes de búsqueda razonables para títulos de películas que
/// incluyen sufijos comerciales (`The Movie`, `Movie`, `Film`, ...). TVmaze no
/// indexa esos sufijos como parte del título del show.
fn search_candidates(media_type: &str, title: &str) -> Vec<String> {
    let title = title.trim();
    if title.is_empty() {
        return Vec::new();
    }

    let mut candidates = vec![title.to_string()];
    if media_type != "movie" {
        return candidates;
    }

    let lower = title.to_lowercase();
    for suffix in [
        " the movie",
        " movie",
        " film",
        " la pelicula",
        " la película",
        " pelicula",
        " película",
    ] {
        if let Some(stripped) = lower.strip_suffix(suffix)
            && let Some(original) = title.get(..stripped.len())
        {
            let candidate = original.trim().to_string();
            if !candidate.is_empty() && !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
    }

    candidates
}

/// Busca una imagen ancha para el banner del medio.
async fn fetch_banner_url(
    media_type: &str,
    title: &str,
    show_id: Option<u64>,
    client: &TvmazeClient,
) -> Option<String> {
    if media_type == "anime" {
        if let Some(url) = fetch_anilist_banner(title, client).await {
            return Some(url);
        }
        if let Some(url) = fetch_kitsu_banner(title, client).await {
            return Some(url);
        }
    }

    match show_id {
        Some(id) => fetch_tvmaze_banner(id, client).await,
        None => None,
    }
}

/// Obtiene el banner de AniList (GraphQL público, sin clave).
async fn fetch_anilist_banner(title: &str, client: &TvmazeClient) -> Option<String> {
    let body = serde_json::json!({
        "query": "query ($search: String) { Media(search: $search, type: ANIME) { bannerImage } }",
        "variables": { "search": title }
    });

    let response = client
        .client
        .post("https://graphql.anilist.co")
        .json(&body)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<Value>()
        .await
        .ok()?;

    parse_anilist_banner(&response)
}

/// Obtiene el cover ancho de Kitsu como respaldo de AniList.
async fn fetch_kitsu_banner(title: &str, client: &TvmazeClient) -> Option<String> {
    let url = format!(
        "https://kitsu.io/api/edge/anime?filter%5Btext%5D={}&page%5Blimit%5D=1",
        urlencoding::encode(title)
    );

    let response = client
        .client
        .get(&url)
        .header("Accept", "application/vnd.api+json")
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<Value>()
        .await
        .ok()?;

    parse_kitsu_banner(&response)
}

/// Obtiene la imagen `background` o `banner` de TVmaze para un show.
async fn fetch_tvmaze_banner(show_id: u64, client: &TvmazeClient) -> Option<String> {
    let url = format!("https://api.tvmaze.com/shows/{}/images", show_id);

    let response = client
        .client
        .get(&url)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<Value>()
        .await
        .ok()?;

    parse_tvmaze_banner(&response)
}

/// Obtiene todas las imágenes disponibles (póster y banner) de TVmaze para un show,
/// incluyendo cada resolución disponible (original y medium).
///
/// Devuelve una `Vec<ImageOption>` con una entrada por cada combinación
/// (tipo, resolución) que la API ofrezca. Si no hay `show_id` o la consulta falla,
/// devuelve un vector vacío (no es un error).
pub async fn fetch_tvmaze_images(show_id: u64, client: &TvmazeClient) -> Vec<ImageOption> {
    let url = format!("https://api.tvmaze.com/shows/{}/images", show_id);

    let response: Value = match client.client.get(&url).send().await {
        Ok(r) => match r.error_for_status() {
            Ok(r) => match r.json::<Value>().await {
                Ok(v) => v,
                Err(_) => return Vec::new(),
            },
            Err(_) => return Vec::new(),
        },
        Err(_) => return Vec::new(),
    };

    let images = match response.as_array() {
        Some(arr) => arr,
        None => return Vec::new(),
    };

    let mut options: Vec<ImageOption> = Vec::new();
    let mut seen_urls: HashSet<String> = HashSet::new();

    for image in images {
        let image_type = match image.get("type").and_then(Value::as_str) {
            Some(t) if !t.is_empty() => t.to_string(),
            _ => continue,
        };

        // Solo nos interesan pósters y banners/backgrounds.
        if image_type != "poster" && image_type != "banner" && image_type != "background" {
            continue;
        }

        let resolutions = match image.get("resolutions") {
            Some(r) => r,
            None => continue,
        };

        // Solo tomamos la resolución "original" (mejor calidad) para evitar
        // mostrar la misma imagen dos veces (original y medium son URLs
        // diferentes aunque sean la misma imagen).
        let url = match resolutions
            .get("original")
            .and_then(|res| res.get("url"))
            .and_then(Value::as_str)
            .filter(|u| !u.is_empty())
        {
            Some(u) => u.to_string(),
            None => continue,
        };

        // Evitar duplicados: si ya vimos esta URL, no la añadimos de nuevo.
        if !seen_urls.insert(url.clone()) {
            continue;
        }

        options.push(ImageOption {
            image_type: image_type.clone(),
            provider: "TVmaze".to_string(),
            resolution: "original".to_string(),
            url,
        });
    }

    options
}

/// Obtiene la lista de episodios de un show con sus imágenes individuales.
async fn fetch_tvmaze_episodes(show_id: u64, client: &TvmazeClient) -> Option<Vec<TvmazeEpisode>> {
    let url = format!("https://api.tvmaze.com/shows/{}/episodes", show_id);

    let response = client
        .client
        .get(&url)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json::<Value>()
        .await
        .ok()?;

    Some(parse_tvmaze_episodes(&response))
}

fn parse_tvmaze_episodes(response: &Value) -> Vec<TvmazeEpisode> {
    let Some(episodes) = response.as_array() else {
        return Vec::new();
    };

    episodes
        .iter()
        .map(|episode| {
            let season = episode
                .get("season")
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok());
            let number = episode
                .get("number")
                .and_then(Value::as_i64)
                .and_then(|value| i32::try_from(value).ok());
            let image_url = episode
                .get("image")
                .and_then(|image| image.get("medium").or_else(|| image.get("original")))
                .and_then(Value::as_str)
                .filter(|url| !url.is_empty())
                .map(str::to_string);

            TvmazeEpisode {
                season,
                number,
                image_url,
            }
        })
        .collect()
}

/// Devuelve la imagen del episodio que coincide exactamente en temporada y
/// número. No se reutiliza el póster de la serie ni se inventa una imagen.
fn find_episode_image(episodes: &[TvmazeEpisode], season: i32, episode: i32) -> Option<String> {
    episodes
        .iter()
        .find(|candidate| candidate.season == Some(season) && candidate.number == Some(episode))
        .and_then(|candidate| candidate.image_url.clone())
}

/// Deduce la temporada y el episodio de un archivo a partir de su título o de
/// su ruta. Si no hay marca reconocible, usa los valores guardados al indexar.
fn episode_position(file: &MediaFile) -> Option<(i32, i32)> {
    let from_title = file
        .title
        .as_deref()
        .and_then(crate::core::library::parse_season_episode);
    if from_title.is_some() {
        return from_title;
    }

    let from_path = Path::new(&file.file_path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(crate::core::library::parse_season_episode);
    if from_path.is_some() {
        return from_path;
    }

    match (file.season, file.episode) {
        (Some(season), Some(episode)) => Some((season, episode)),
        _ => None,
    }
}

fn mark_episode_checked(
    db_path: &str,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut conn = open_db(db_path)?;
    let mut repo = FileRepository::new(&mut conn);
    repo.mark_episode_image_checked(file_path)?;
    close_db(conn);
    Ok(())
}

fn parse_search_response(response: &Value) -> SyncMetadataResult {
    let Some(show) = response
        .as_array()
        .and_then(|results| results.first())
        .and_then(|first| first.get("show"))
    else {
        return SyncMetadataResult::default();
    };

    let genres = show
        .get("genres")
        .and_then(|g| g.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|g| g.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();

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
        genres,
        banner_path: None,
        show_id: show.get("id").and_then(Value::as_u64),
    }
}

/// Extrae el banner de una respuesta de AniList.
fn parse_anilist_banner(response: &Value) -> Option<String> {
    response
        .get("data")?
        .get("Media")?
        .get("bannerImage")?
        .as_str()
        .map(str::to_string)
        .filter(|url| !url.is_empty())
}

/// Extrae el cover ancho de una respuesta de Kitsu.
fn parse_kitsu_banner(response: &Value) -> Option<String> {
    response
        .get("data")?
        .as_array()?
        .first()?
        .get("attributes")?
        .get("coverImage")?
        .get("original")?
        .as_str()
        .map(str::to_string)
        .filter(|url| !url.is_empty())
}

/// Extrae la mejor imagen ancha (`background` o `banner`) de TVmaze.
fn parse_tvmaze_banner(images: &Value) -> Option<String> {
    let images = images.as_array()?;
    let pick = |kind: &str| {
        images
            .iter()
            .find(|image| image.get("type").and_then(Value::as_str) == Some(kind))
    };
    let image = pick("background").or_else(|| pick("banner"))?;
    let resolutions = image.get("resolutions")?;

    for key in ["original", "medium"] {
        if let Some(url) = resolutions
            .get(key)
            .and_then(|resolution| resolution.get("url"))
            .and_then(Value::as_str)
            .filter(|url| !url.is_empty())
        {
            return Some(url.to_string());
        }
    }

    None
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
                "id": 64632,
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
        assert_eq!(parsed.show_id, Some(64632));
        assert_eq!(parsed.banner_path, None);
    }

    #[test]
    fn parse_tvmaze_banner_prefiere_background_y_original() {
        let images = json!([
            {
                "type": "poster",
                "resolutions": {
                    "original": {"url": "https://example.com/poster.jpg"}
                }
            },
            {
                "type": "banner",
                "resolutions": {
                    "original": {"url": "https://example.com/banner.jpg"}
                }
            },
            {
                "type": "background",
                "resolutions": {
                    "medium": {"url": "https://example.com/background-medium.jpg"},
                    "original": {"url": "https://example.com/background.jpg"}
                }
            }
        ]);

        assert_eq!(
            parse_tvmaze_banner(&images).as_deref(),
            Some("https://example.com/background.jpg")
        );
        assert_eq!(parse_tvmaze_banner(&json!([])), None);
    }

    #[test]
    fn parse_tvmaze_banner_usa_medium_si_no_hay_original() {
        let images = json!([{
            "type": "background",
            "resolutions": {
                "medium": {"url": "https://example.com/background-medium.jpg"}
            }
        }]);

        assert_eq!(
            parse_tvmaze_banner(&images).as_deref(),
            Some("https://example.com/background-medium.jpg")
        );
    }

    #[test]
    fn parse_anilist_banner_extrae_la_imagen() {
        let response = json!({
            "data": {
                "Media": {
                    "bannerImage": "https://s4.anilist.co/file/anilistcdn/media/anime/banner/151807.jpg"
                }
            }
        });

        assert_eq!(
            parse_anilist_banner(&response).as_deref(),
            Some("https://s4.anilist.co/file/anilistcdn/media/anime/banner/151807.jpg")
        );
        assert_eq!(
            parse_anilist_banner(&json!({"data": {"Media": null}})),
            None
        );
        assert_eq!(parse_anilist_banner(&json!({})), None);
    }

    #[test]
    fn parse_kitsu_banner_extrae_el_cover() {
        let response = json!({
            "data": [{
                "attributes": {
                    "coverImage": {
                        "original": "https://media.kitsu.app/anime/46231/cover_image.jpg"
                    }
                }
            }]
        });

        assert_eq!(
            parse_kitsu_banner(&response).as_deref(),
            Some("https://media.kitsu.app/anime/46231/cover_image.jpg")
        );
        assert_eq!(parse_kitsu_banner(&json!({"data": []})), None);
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

    fn media_file(
        season: Option<i32>,
        episode: Option<i32>,
        title: Option<&str>,
        file_path: &str,
    ) -> MediaFile {
        MediaFile {
            id: 1,
            media_id: 1,
            season,
            episode,
            title: title.map(str::to_string),
            file_path: file_path.to_string(),
            size_bytes: None,
            mtime_ms: None,
            indexed_at: String::new(),
            audio_stream_index: None,
            subtitle_stream_index: None,
            subtitles_disabled: false,
            streams_indexed_at: None,
            episode_image_path: None,
            episode_image_checked_at: None,
        }
    }

    #[test]
    fn parse_tvmaze_episodes_extrae_temporada_numero_e_imagen() {
        let response = json!([
            {
                "season": 1,
                "number": 2,
                "image": {"medium": "https://example.com/ep1x2.jpg"}
            },
            {
                "season": 2,
                "number": 1,
                "image": {"original": "https://example.com/ep2x1.jpg"}
            },
            {"season": 2, "number": 2, "image": null}
        ]);

        let episodes = parse_tvmaze_episodes(&response);
        assert_eq!(episodes.len(), 3);
        assert_eq!(
            episodes[0].image_url.as_deref(),
            Some("https://example.com/ep1x2.jpg")
        );
        assert_eq!(episodes[1].season, Some(2));
        assert_eq!(
            episodes[1].image_url.as_deref(),
            Some("https://example.com/ep2x1.jpg")
        );
        assert!(episodes[2].image_url.is_none());
        assert!(parse_tvmaze_episodes(&json!({"error": "x"})).is_empty());
    }

    #[test]
    fn find_episode_image_exige_temporada_y_episodio_exactos() {
        let episodes = parse_tvmaze_episodes(&json!([
            {
                "season": 1,
                "number": 2,
                "image": {"medium": "https://example.com/ep1x2.jpg"}
            },
            {"season": 1, "number": 3, "image": null}
        ]));

        assert_eq!(
            find_episode_image(&episodes, 1, 2).as_deref(),
            Some("https://example.com/ep1x2.jpg")
        );
        assert_eq!(find_episode_image(&episodes, 1, 3), None);
        assert_eq!(find_episode_image(&episodes, 2, 2), None);
        assert_eq!(find_episode_image(&episodes, 1, 99), None);
    }

    #[test]
    fn episode_position_prioriza_titulo_y_ruta_sobre_los_campos() {
        let from_title = media_file(Some(9), Some(9), Some("Show.S01E02"), "/tmp/Show.mkv");
        assert_eq!(episode_position(&from_title), Some((1, 2)));

        let from_path = media_file(None, None, None, "/tmp/Show.2x05.mkv");
        assert_eq!(episode_position(&from_path), Some((2, 5)));

        let from_fields = media_file(Some(3), Some(7), Some("Sin marca"), "/tmp/otro.mkv");
        assert_eq!(episode_position(&from_fields), Some((3, 7)));

        let unknown = media_file(None, None, Some("Especial"), "/tmp/especial.mkv");
        assert_eq!(episode_position(&unknown), None);
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
                banner_failed: 0,
            }
        );
    }
}
