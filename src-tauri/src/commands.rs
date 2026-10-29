//! Comandos Tauri expuestos al frontend.
//!
//! Cada comando abre la base de datos SQLite, ejecuta la operación a través
//! de los repositorios de `db::` y cierra la conexión. No hay estado global.

use serde::Serialize;
use tauri::command;

use crate::db::{
    FileRepository, ListMediaOptions, MediaRepository, SearchMediaOptions, close_db, open_db,
};
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
