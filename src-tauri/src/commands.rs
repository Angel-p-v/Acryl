//! Comandos Tauri expuestos al frontend.
//!
//! Cada comando abre la base de datos SQLite, ejecuta la operación a través
//! de los repositorios de `db::` y cierra la conexión. No hay estado global.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::{State, command};

use crate::config::ConfigState;
use crate::db::{
    FileRepository, ListMediaOptions, MediaRepository, SearchMediaOptions, close_db, open_db,
};
use crate::metadata::{TvMazeClient, sync_metadata};
use crate::player::{MpvPlayer, PlayerState};

/// Respuesta de estado del reproductor.
#[derive(Debug, Serialize)]
pub struct PlayerStatus {
    pub time_position: f64,
    pub duration: f64,
    pub paused: bool,
    pub volume: f64,
}

/// Comando: crea (si no existe) y carga un archivo de vídeo en el reproductor.
///
/// mpv creará su propia ventana nativa (X11 en Linux) para renderizar el
/// vídeo, integrada en el proceso de la app (sin `spawn` de mpv externo).
#[command]
pub fn player_open_cmd(state: State<'_, PlayerState>, file_path: String) -> Result<(), String> {
    let mut player_guard = state.player.lock().map_err(|e| e.to_string())?;

    // Si no hay reproductor, crear uno nuevo (sin window ID: mpv crea su
    // propia ventana nativa).
    if player_guard.is_none() {
        let player = MpvPlayer::new(None).map_err(|e| e.to_string())?;
        *player_guard = Some(player);
    }

    let player = player_guard
        .as_mut()
        .ok_or_else(|| "reproductor no disponible".to_string())?;

    player.load_file(&file_path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Comando: reanuda o inicia la reproducción.
#[command]
pub fn player_play_cmd(state: State<'_, PlayerState>) -> Result<(), String> {
    let mut player_guard = state.player.lock().map_err(|e| e.to_string())?;
    let player = player_guard
        .as_mut()
        .ok_or_else(|| "no hay reproductor activo".to_string())?;
    player.play().map_err(|e| e.to_string())
}

/// Comando: pausa la reproducción.
#[command]
pub fn player_pause_cmd(state: State<'_, PlayerState>) -> Result<(), String> {
    let mut player_guard = state.player.lock().map_err(|e| e.to_string())?;
    let player = player_guard
        .as_mut()
        .ok_or_else(|| "no hay reproductor activo".to_string())?;
    player.pause().map_err(|e| e.to_string())
}

/// Comando: detiene la reproducción y cierra el archivo actual.
#[command]
pub fn player_stop_cmd(state: State<'_, PlayerState>) -> Result<(), String> {
    let mut player_guard = state.player.lock().map_err(|e| e.to_string())?;
    let player = player_guard
        .as_mut()
        .ok_or_else(|| "no hay reproductor activo".to_string())?;
    player.stop().map_err(|e| e.to_string())
}

/// Comando: busca a una posición (segundos).
#[command]
pub fn player_seek_cmd(state: State<'_, PlayerState>, seconds: f64) -> Result<(), String> {
    let mut player_guard = state.player.lock().map_err(|e| e.to_string())?;
    let player = player_guard
        .as_mut()
        .ok_or_else(|| "no hay reproductor activo".to_string())?;
    player.seek(seconds).map_err(|e| e.to_string())
}

/// Comando: establece el volumen (0.0 a 100.0).
#[command]
pub fn player_volume_cmd(state: State<'_, PlayerState>, volume: f64) -> Result<(), String> {
    let mut player_guard = state.player.lock().map_err(|e| e.to_string())?;
    let player = player_guard
        .as_mut()
        .ok_or_else(|| "no hay reproductor activo".to_string())?;
    player.set_volume(volume).map_err(|e| e.to_string())
}

/// Comando: obtiene el estado actual del reproductor.
#[command]
pub fn player_status_cmd(state: State<'_, PlayerState>) -> Result<PlayerStatus, String> {
    let mut player_guard = state.player.lock().map_err(|e| e.to_string())?;
    let player = player_guard
        .as_mut()
        .ok_or_else(|| "no hay reproductor activo".to_string())?;
    Ok(PlayerStatus {
        time_position: player.time_position().map_err(|e| e.to_string())?,
        duration: player.duration().map_err(|e| e.to_string())?,
        paused: player.is_paused().map_err(|e| e.to_string())?,
        volume: player.volume().map_err(|e| e.to_string())?,
    })
}
/// Respuesta de listado de medios (envuelve el vector para poder añadir
/// metadatos de paginación en el futuro sin romper el contrato).
#[derive(Debug, Serialize)]
pub struct MediaListResponse {
    pub media: Vec<crate::db::media_repo::Media>,
    pub total: i64,
}

/// Comando: lista medios desde SQLite con filtros opcionales.
#[command]
pub fn list_media_cmd(
    db_path: String,
    r#type: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
    order_by: Option<String>,
) -> Result<MediaListResponse, String> {
    let mut conn = open_db(&db_path).map_err(|e| e.to_string())?;
    let mut repo = MediaRepository::new(&mut conn);

    let total = repo.count(r#type.as_deref()).map_err(|e| e.to_string())?;
    let media = repo
        .list(ListMediaOptions {
            r#type,
            limit,
            offset,
            order_by,
        })
        .map_err(|e| e.to_string())?;

    close_db(conn);
    Ok(MediaListResponse { media, total })
}

/// Comando: busca medios por texto (FTS5 con fallback LIKE).
#[command]
pub fn search_media_cmd(
    db_path: String,
    query: String,
    r#type: Option<String>,
    limit: Option<i64>,
) -> Result<Vec<crate::db::media_repo::Media>, String> {
    let mut conn = open_db(&db_path).map_err(|e| e.to_string())?;
    let mut repo = MediaRepository::new(&mut conn);

    let results = repo
        .search(&query, SearchMediaOptions { r#type, limit })
        .map_err(|e| e.to_string())?;

    close_db(conn);
    Ok(results)
}

/// Comando: obtiene un medio por su id.
#[command]
pub fn get_media_cmd(
    db_path: String,
    id: i64,
) -> Result<Option<crate::db::media_repo::Media>, String> {
    let mut conn = open_db(&db_path).map_err(|e| e.to_string())?;
    let mut repo = MediaRepository::new(&mut conn);

    let media = repo.find_by_id(id).map_err(|e| e.to_string())?;

    close_db(conn);
    Ok(media)
}

/// Comando: obtiene un medio por ruta de carpeta.
#[command]
pub fn get_media_by_folder_cmd(
    db_path: String,
    folder_path: String,
) -> Result<Option<crate::db::media_repo::Media>, String> {
    let mut conn = open_db(&db_path).map_err(|e| e.to_string())?;
    let mut repo = MediaRepository::new(&mut conn);

    let media = repo
        .find_by_folder_path(&folder_path)
        .map_err(|e| e.to_string())?;

    close_db(conn);
    Ok(media)
}

/// Comando: cuenta medios, opcionalmente filtrados por tipo.
#[command]
pub fn count_media_cmd(db_path: String, r#type: Option<String>) -> Result<i64, String> {
    let mut conn = open_db(&db_path).map_err(|e| e.to_string())?;
    let mut repo = MediaRepository::new(&mut conn);

    let total = repo.count(r#type.as_deref()).map_err(|e| e.to_string())?;

    close_db(conn);
    Ok(total)
}

/// Comando: lista las rutas de carpeta de todos los medios.
#[command]
pub fn list_folders_cmd(db_path: String) -> Result<Vec<String>, String> {
    let mut conn = open_db(&db_path).map_err(|e| e.to_string())?;
    let mut repo = MediaRepository::new(&mut conn);

    let folders = repo.list_folders().map_err(|e| e.to_string())?;

    close_db(conn);
    Ok(folders)
}

/// Comando: lista los archivos de un medio.
#[command]
pub fn list_files_by_media_cmd(
    db_path: String,
    media_id: i64,
) -> Result<Vec<crate::db::file_repo::MediaFile>, String> {
    let mut conn = open_db(&db_path).map_err(|e| e.to_string())?;
    let mut repo = FileRepository::new(&mut conn);

    let files = repo.list_by_media(media_id).map_err(|e| e.to_string())?;

    close_db(conn);
    Ok(files)
}

/// Comando: obtiene el detalle completo de un medio (título + archivos).
#[command]
pub fn get_media_details_cmd(
    db_path: String,
    media_id: i64,
) -> Result<Option<crate::db::media_repo::MediaDetails>, String> {
    let mut conn = open_db(&db_path).map_err(|e| e.to_string())?;
    let mut repo = MediaRepository::new(&mut conn);

    let details = repo.get_details(media_id).map_err(|e| e.to_string())?;

    close_db(conn);
    Ok(details)
}

/// Comando: invierte el estado de favorito de un medio. Devuelve el nuevo valor.
#[command]
pub fn toggle_favorite_cmd(db_path: String, media_id: i64) -> Result<bool, String> {
    let mut conn = open_db(&db_path).map_err(|e| e.to_string())?;
    let mut repo = MediaRepository::new(&mut conn);

    let value = repo.toggle_favorite(media_id).map_err(|e| e.to_string())?;

    close_db(conn);
    Ok(value)
}

/// Comando: invierte el estado de visto de un medio. Devuelve el nuevo valor.
#[command]
pub fn toggle_watched_cmd(db_path: String, media_id: i64) -> Result<bool, String> {
    let mut conn = open_db(&db_path).map_err(|e| e.to_string())?;
    let mut repo = MediaRepository::new(&mut conn);

    let value = repo.toggle_watched(media_id).map_err(|e| e.to_string())?;

    close_db(conn);
    Ok(value)
}

/// Respuesta de obtención de configuración.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub db_path: String,
    pub language: String,
    pub auto_sync_metadata: bool,
    pub default_player: String,
}

/// Comando: obtiene la configuración actual de la aplicación.
#[command]
pub fn config_get_cmd(state: State<'_, ConfigState>) -> Result<ConfigResponse, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(ConfigResponse {
        db_path: config.db_path.clone(),
        language: config.language.clone(),
        auto_sync_metadata: config.auto_sync_metadata,
        default_player: config.default_player.clone(),
    })
}

/// Comando: actualiza la configuración de la aplicación y la persiste en disco.
#[command]
pub fn config_set_cmd(
    state: State<'_, ConfigState>,
    db_path: Option<String>,
    language: Option<String>,
    auto_sync_metadata: Option<bool>,
    default_player: Option<String>,
) -> Result<ConfigResponse, String> {
    let mut config = state.config.lock().map_err(|e| e.to_string())?;

    if let Some(v) = db_path {
        config.db_path = v;
    }
    if let Some(v) = language {
        config.language = v;
    }
    if let Some(v) = auto_sync_metadata {
        config.auto_sync_metadata = v;
    }
    if let Some(v) = default_player {
        config.default_player = v;
    }

    let config_clone = config.clone();
    drop(config);

    ConfigState::save_to_file(&state.config_path, &config_clone).map_err(|e| e.to_string())?;

    Ok(ConfigResponse {
        db_path: config_clone.db_path,
        language: config_clone.language,
        auto_sync_metadata: config_clone.auto_sync_metadata,
        default_player: config_clone.default_player,
    })
}

/// Respuesta de sincronización de metadatos.
#[derive(Debug, Serialize)]
pub struct SyncMetadataResponse {
    pub title: Option<String>,
    pub synopsis: Option<String>,
    pub poster_path: Option<String>,
    pub release_date: Option<String>,
    pub vote_average: Option<f64>,
}

/// Comando: sincroniza los metadatos de un medio desde TVmaze.
#[command]
pub async fn sync_metadata_cmd(
    db_path: String,
    media_id: i64,
) -> Result<SyncMetadataResponse, String> {
    let mut conn = open_db(&db_path).map_err(|e| e.to_string())?;
    let mut repo = MediaRepository::new(&mut conn);

    // Obtener el medio para saber el título.
    let media = repo
        .find_by_id(media_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("medio no encontrado: id={}", media_id))?;

    let client = TvMazeClient::new();
    let fetched = sync_metadata(&media.r#type, &media.title, &client).await?;

    repo.update_metadata(
        media_id,
        fetched.poster_path.as_deref(),
        fetched.synopsis.as_deref(),
    )
    .map_err(|e| e.to_string())?;

    close_db(conn);

    Ok(SyncMetadataResponse {
        title: fetched.title,
        synopsis: fetched.synopsis,
        poster_path: fetched.poster_path,
        release_date: fetched.release_date,
        vote_average: fetched.vote_average,
    })
}

/// Respuesta de registro de reproducción.
#[derive(Debug, Serialize)]
pub struct RecordPlaybackResponse {
    pub ok: bool,
}

/// Comando: registra una reproducción en el historial.
#[command]
pub fn record_playback_cmd(
    db_path: String,
    media_id: i64,
    position: f64,
    duration: f64,
) -> Result<RecordPlaybackResponse, String> {
    let mut conn = open_db(&db_path).map_err(|e| e.to_string())?;
    let mut repo = MediaRepository::new(&mut conn);

    repo.record_playback(media_id, position, duration)
        .map_err(|e| e.to_string())?;

    close_db(conn);

    Ok(RecordPlaybackResponse { ok: true })
}

/// Respuesta de historial de reproducción.
#[derive(Debug, Serialize)]
pub struct PlaybackHistoryResponse {
    pub history: Vec<crate::db::media_repo::PlaybackRecord>,
}

/// Comando: obtiene el historial de reproducción.
#[command]
pub fn get_playback_history_cmd(
    db_path: String,
    limit: Option<i64>,
) -> Result<PlaybackHistoryResponse, String> {
    let mut conn = open_db(&db_path).map_err(|e| e.to_string())?;
    let mut repo = MediaRepository::new(&mut conn);

    let history = repo
        .list_playback_history(limit)
        .map_err(|e| e.to_string())?;

    close_db(conn);

    Ok(PlaybackHistoryResponse { history })
}

/// Respuesta de indexación de carpeta.
#[derive(Debug, Serialize)]
pub struct IndexFolderResponse {
    pub media_added: i64,
    pub files_added: i64,
    pub folders_processed: i64,
}

/// Extensiones de video soportadas.
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "webm", "mov", "flv", "wmv"];

/// Comando: escanea una carpeta y indexa todos los medios de vídeo encontrados.
///
/// Crea una entrada en `media` por cada carpeta que contenga vídeos y una
/// entrada en `media_files` por cada archivo de vídeo.
#[command]
pub fn index_folder_cmd(
    db_path: String,
    folder_path: String,
) -> Result<IndexFolderResponse, String> {
    let folder = Path::new(&folder_path);
    if !folder.is_dir() {
        return Err(format!("La ruta no es una carpeta válida: {}", folder_path));
    }

    let result = {
        let mut conn = open_db(&db_path).map_err(|e| e.to_string())?;
        let mut media_repo = MediaRepository::new(&mut conn);

        let mut media_added = 0i64;
        let mut files_added = 0i64;
        let mut folders_processed = 0i64;

        // Recorrer entradas de la carpeta raíz
        for entry in fs::read_dir(folder).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();

            // Solo procesar subcarpetas (cada carpeta = una serie/película)
            if !path.is_dir() {
                continue;
            }

            folders_processed += 1;

            // Recopilar archivos de vídeo en esta carpeta
            let mut video_files: Vec<String> = Vec::new();
            if let Ok(read_dir) = fs::read_dir(&path) {
                for file_entry in read_dir.flatten() {
                    let file_path = file_entry.path();
                    if file_path.is_file() {
                        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                            if VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
                                if let Some(path_str) = file_path.to_str() {
                                    video_files.push(path_str.to_string());
                                }
                            }
                        }
                    }
                }
            }

            if video_files.is_empty() {
                continue;
            }

            // Ordenar archivos para tener un orden consistente
            video_files.sort();

            // Determinar tipo por nombre de carpeta (heurística simple)
            let folder_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
            let r#type = if folder_name.contains("anime") {
                "anime"
            } else if folder_name.contains("serie") || folder_name.contains("series") {
                "series"
            } else {
                "movie"
            };

            // Título basado en el nombre de la carpeta
            let title = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let sort_title = title.to_lowercase();

            // Insertar o actualizar el medio (bloque separado para soltar el préstamo)
            let media_id = {
                let media = media_repo
                    .upsert(&crate::db::media_repo::MediaInput {
                        r#type: r#type.to_string(),
                        title: title.clone(),
                        sort_title,
                        year: None,
                        folder_path: path.to_string_lossy().to_string(),
                    })
                    .map_err(|e| e.to_string())?;
                media.id
            };
            media_added += 1;

            // Insertar archivos de vídeo
            for (idx, file_path_str) in video_files.iter().enumerate() {
                let file_name = Path::new(file_path_str)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                // Intentar extraer temporada y episodio (ej: S01E01, s01e01, 1x01)
                let (season, episode) = extract_season_episode(&file_name);

                let file_size = fs::metadata(file_path_str)
                    .ok()
                    .and_then(|m| m.len().try_into().ok())
                    .or(None);

                let mtime_ms = fs::metadata(file_path_str)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as i64)
                    .or(None);

                // Abrir una nueva conexión solo para insertar este archivo
                let mut file_conn = open_db(&db_path).map_err(|e| e.to_string())?;
                let mut file_repo = FileRepository::new(&mut file_conn);
                file_repo
                    .upsert_many(&[crate::db::file_repo::MediaFileInput {
                        media_id,
                        season,
                        episode,
                        title: Some(file_name),
                        file_path: file_path_str.clone(),
                        size_bytes: file_size,
                        mtime_ms,
                    }])
                    .map_err(|e| e.to_string())?;
                close_db(file_conn);

                files_added += 1;

                // Para películas, solo indexar el primer archivo
                if r#type == "movie" && idx > 0 {
                    break;
                }
            }
        }

        IndexFolderResponse {
            media_added,
            files_added,
            folders_processed,
        }
    };

    Ok(result)
}

/// Extrae temporada y episodio del nombre de archivo.
/// Soporta formatos como: S01E01, s01e01, 1x01, Episodio 01, etc.
fn extract_season_episode(file_name: &str) -> (Option<i32>, Option<i32>) {
    let lower = file_name.to_lowercase();

    // Patrón S01E01 o s01e01
    if let Some(pos) = lower.find("s") {
        if let Some(rest) = lower.get(pos + 1..) {
            let season_digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if !season_digits.is_empty() {
                let season: i32 = season_digits.parse().unwrap_or(0);
                if let Some(e_pos) = rest.find("e") {
                    let ep_digits: String = rest[e_pos + 1..]
                        .chars()
                        .take_while(|c| c.is_ascii_digit())
                        .collect();
                    if !ep_digits.is_empty() {
                        let episode: i32 = ep_digits.parse().unwrap_or(0);
                        return (Some(season), Some(episode));
                    }
                }
            }
        }
    }

    // Patrón 1x01
    if let Some(pos) = lower.find('x') {
        if pos > 0 {
            let season_digits: String = lower[..pos]
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            if !season_digits.is_empty() {
                let season: i32 = season_digits.parse().unwrap_or(0);
                let ep_digits: String = lower[pos + 1..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if !ep_digits.is_empty() {
                    let episode: i32 = ep_digits.parse().unwrap_or(0);
                    return (Some(season), Some(episode));
                }
            }
        }
    }

    (None, None)
}
