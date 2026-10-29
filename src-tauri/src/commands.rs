//! Comandos Tauri expuestos al frontend.
//!
//! Cada comando abre la base de datos SQLite, ejecuta la operación a través
//! de los repositorios de `db::` y cierra la conexión. No hay estado global.

use serde::Serialize;
use tauri::{command, State};

use crate::db::{
    FileRepository, ListMediaOptions, MediaRepository, SearchMediaOptions, close_db, open_db,
};
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
pub fn player_open_cmd(
    state: State<'_, PlayerState>,
    file_path: String,
) -> Result<(), String> {
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
pub fn player_seek_cmd(
    state: State<'_, PlayerState>,
    seconds: f64,
) -> Result<(), String> {
    let mut player_guard = state.player.lock().map_err(|e| e.to_string())?;
    let player = player_guard
        .as_mut()
        .ok_or_else(|| "no hay reproductor activo".to_string())?;
    player.seek(seconds).map_err(|e| e.to_string())
}

/// Comando: establece el volumen (0.0 a 100.0).
#[command]
pub fn player_volume_cmd(
    state: State<'_, PlayerState>,
    volume: f64,
) -> Result<(), String> {
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
