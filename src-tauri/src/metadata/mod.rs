//! Sincronización automática de metadatos con TVmaze.
//!
//! TVmaze es una API pública y gratuita que no requiere autenticación
//! (sin API key). Proporciona metadatos de series y películas: título,
//! sinopsis, póster, fecha de estreno, géneros, red/canal, sitio oficial,
//! e IDs externos (TVDB, IMDB, TVRage).
//!
//! Documentación: https://www.tvmaze.com/api
//! Rate limit: 20 requests / 10 segundos por IP (plan gratuito).
//! Licencia: CC BY-SA (requiere atribución a TVmaze).

use serde::Deserialize;
use std::collections::HashMap;

/// Resultado de búsqueda de un show en TVmaze.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub(crate) struct TvMazeSearchResult {
    pub(crate) score: Option<f64>,
    pub(crate) show: TvMazeShow,
}

/// Información principal de un show de TVmaze.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub(crate) struct TvMazeShow {
    pub(crate) id: Option<i64>,
    pub(crate) name: Option<String>,
    pub(crate) r#type: Option<String>,
    pub(crate) language: Option<String>,
    pub(crate) genres: Option<Vec<String>>,
    pub(crate) status: Option<String>,
    pub(crate) premiered: Option<String>,
    pub(crate) ended: Option<String>,
    pub(crate) official_site: Option<String>,
    pub(crate) rating: Option<TvMazeRating>,
    pub(crate) image: Option<TvMazeImage>,
    pub(crate) summary: Option<String>,
    pub(crate) externals: Option<TvMazeExternals>,
}

/// Calificación de un show en TVmaze.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub(crate) struct TvMazeRating {
    pub(crate) average: Option<f64>,
}

/// Imagen de un show en TVmaze.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub(crate) struct TvMazeImage {
    pub(crate) medium: Option<String>,
    pub(crate) original: Option<String>,
}

/// IDs externos de un show (TVDB, IMDB, TVRage).
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub(crate) struct TvMazeExternals {
    pub(crate) tvrage: Option<i64>,
    pub(crate) thetvdb: Option<i64>,
    pub(crate) imdb: Option<String>,
}

/// Metadatos normalizados obtenidos de TVmaze.
#[derive(Debug, Clone)]
pub struct FetchedMetadata {
    pub title: Option<String>,
    pub synopsis: Option<String>,
    pub poster_path: Option<String>,
    pub release_date: Option<String>,
    pub vote_average: Option<f64>,
}

/// Cliente de TVmaze.
///
/// No requiere autenticación (sin API key). Es plug-and-play.
pub struct TvMazeClient {
    client: reqwest::Client,
    base_url: String,
}

impl TvMazeClient {
    /// Crea un nuevo cliente de TVmaze.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("media-manager/0.1.0")
                .build()
                .unwrap_or_default(),
            base_url: "https://api.tvmaze.com".to_string(),
        }
    }

    /// Busca un show por título y devuelve el mejor resultado.
    pub async fn search_show(&self, query: &str) -> Result<Option<TvMazeSearchResult>, String> {
        let url = format!("{}/search/shows", self.base_url);
        let params: HashMap<&str, &str> = [("q", query)].into();

        let results: Vec<TvMazeSearchResult> = self
            .client
            .get(&url)
            .query(&params)
            .send()
            .await
            .map_err(|e| format!("error conectando a TVmaze: {}", e))?
            .json()
            .await
            .map_err(|e| format!("error parseando respuesta de TVmaze: {}", e))?;

        Ok(results.into_iter().next())
    }

    /// Obtiene los detalles completos de un show por su ID de TVmaze.
    pub async fn get_show(&self, tvmaze_id: i64) -> Result<TvMazeShow, String> {
        let url = format!("{}/shows/{}", self.base_url, tvmaze_id);

        let show: TvMazeShow = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("error conectando a TVmaze: {}", e))?
            .json()
            .await
            .map_err(|e| format!("error parseando respuesta de TVmaze: {}", e))?;

        Ok(show)
    }

    /// Busca un show por título y, si lo encuentra, devuelve sus detalles completos.
    pub async fn search_and_get_show(&self, query: &str) -> Result<Option<TvMazeShow>, String> {
        let show_id = self
            .search_show(query)
            .await?
            .and_then(|result| result.show.id);

        if let Some(id) = show_id {
            return Ok(Some(self.get_show(id).await?));
        }
        Ok(None)
    }
}

/// Sincroniza los metadatos de un medio desde TVmaze.
///
/// Busca el título en TVmaze y, si encuentra un resultado, devuelve los
/// metadatos normalizados para guardar en la base de datos.
pub async fn sync_metadata(
    _type: &str,
    title: &str,
    client: &TvMazeClient,
) -> Result<FetchedMetadata, String> {
<<<<<<< HEAD
=======
    // TVmaze no distingue entre película y serie de forma estricta en la
    // búsqueda; intentamos con el título directamente.
>>>>>>> f33b269 (feat(hito-8): metadatos automáticos con TVmaze (sin API key))
    let show = client
        .search_and_get_show(title)
        .await?
        .ok_or_else(|| format!("no se encontraron resultados en TVmaze para '{}'", title))?;

    // Limpiar etiquetas HTML de la sinopsis.
    let synopsis = show.summary.map(|s| strip_html_tags(&s));

    Ok(FetchedMetadata {
        title: show.name,
        synopsis,
<<<<<<< HEAD
        poster_path: show.image.and_then(|img| img.original.or(img.medium)),
=======
        poster_path: show
            .image
            .and_then(|img| img.original.or(img.medium)),
>>>>>>> f33b269 (feat(hito-8): metadatos automáticos con TVmaze (sin API key))
        release_date: show.premiered,
        vote_average: show.rating.and_then(|r| r.average),
    })
}

/// Elimina etiquetas HTML de una cadena de texto.
fn strip_html_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => result.push(c),
            _ => {}
        }
    }

    // Reemplazar entidades HTML comunes.
    let result = result
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poster_path_uses_original_when_available() {
<<<<<<< HEAD
=======
        // Este test verifica la lógica de selección de poster_path.
        // En producción se usa image.original o image.medium.
>>>>>>> f33b269 (feat(hito-8): metadatos automáticos con TVmaze (sin API key))
        let img = TvMazeImage {
            medium: Some("https://example.com/medium.jpg".to_string()),
            original: Some("https://example.com/original.jpg".to_string()),
        };
        let poster_path = img.original.or(img.medium);
        assert_eq!(poster_path, Some("https://example.com/original.jpg".to_string()));
    }

    #[test]
    fn poster_path_falls_back_to_medium() {
        let img = TvMazeImage {
            medium: Some("https://example.com/medium.jpg".to_string()),
            original: None,
        };
        let poster_path = img.original.or(img.medium);
        assert_eq!(poster_path, Some("https://example.com/medium.jpg".to_string()));
    }

    #[test]
    fn strip_html_tags_removes_tags() {
        let html = "<p>Hola <b>mundo</b>!</p>";
        let text = strip_html_tags(html);
        assert_eq!(text, "Hola mundo!");
    }

    #[test]
    fn strip_html_handles_entities() {
        let html = "&lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt;";
        let text = strip_html_tags(html);
<<<<<<< HEAD
=======
        // Las entidades HTML se decodifican, pero las etiquetas quedan como texto plano.
>>>>>>> f33b269 (feat(hito-8): metadatos automáticos con TVmaze (sin API key))
        assert!(!text.contains("&lt;"));
        assert!(!text.contains("&quot;"));
        assert_eq!(text, "<script>alert(\"x\")</script>");
    }
}
