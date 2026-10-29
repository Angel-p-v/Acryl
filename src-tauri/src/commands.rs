use crate::db::list_media;
use crate::media::Media;
use tauri::command;

/// Comando Tauri expuesto al frontend: lista medios desde SQLite.
///
/// `db_path` es la ruta al archivo .db; `query` filtra por titulo (opcional).
#[command]
pub fn list_media_cmd(db_path: String, query: Option<String>) -> Result<Vec<Media>, String> {
    list_media(&db_path, query.as_deref()).map_err(|e| e.to_string())
}
