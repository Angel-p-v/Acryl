//! Capa de dominio: servicios de negocio.

use crate::db::media_repo::MediaInput;
use crate::db::{MediaRepository, close_db, open_db};

pub struct LibraryService;

impl LibraryService {
    pub fn index_folder(
        db_path: &str,
        folder_path: &str,
    ) -> Result<IndexResult, Box<dyn std::error::Error>> {
        let mut conn = open_db(db_path)?;

        let mut media_added = 0i64;
        let mut files_added = 0i64;
        let mut folders_processed = 0i64;

        // Indexar la carpeta raíz: sus subcarpetas directas con videos se tratan como series
        Self::index_root_folder(
            &mut conn,
            folder_path,
            &mut media_added,
            &mut files_added,
            &mut folders_processed,
        )?;

        close_db(conn);

        Ok(IndexResult {
            media_added,
            files_added,
            folders_processed,
        })
    }

    /// Indexa una carpeta raíz. Sus subcarpetas directas que contienen videos se tratan
    /// como series independientes (parent_id = None). Cada subcarpeta puede contener
    /// temporadas (sub-subcarpetas con videos) o archivos de video directamente.
    fn index_root_folder(
        conn: &mut rusqlite::Connection,
        folder_path: &str,
        media_added: &mut i64,
        files_added: &mut i64,
        folders_processed: &mut i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let entries: Vec<_> = std::fs::read_dir(folder_path)?
            .filter_map(|e| e.ok())
            .collect();

        for entry in entries {
            let path = entry.path();

            if path.is_dir() {
                *folders_processed += 1;

                // Esta subcarpeta es una serie candidata
                Self::index_series_folder(
                    conn,
                    path.to_str().unwrap(),
                    media_added,
                    files_added,
                    folders_processed,
                )?;
            }
        }

        Ok(())
    }

    /// Indexa una carpeta como serie. Si tiene subcarpetas con videos, esas son temporadas.
    /// Si tiene videos directamente, esos son los episodios de la serie.
    fn index_series_folder(
        conn: &mut rusqlite::Connection,
        folder_path: &str,
        media_added: &mut i64,
        files_added: &mut i64,
        folders_processed: &mut i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let entries: Vec<_> = std::fs::read_dir(folder_path)?
            .filter_map(|e| e.ok())
            .collect();

        // Separar subcarpetas y archivos de video directos
        let mut subdirs: Vec<std::path::PathBuf> = Vec::new();
        let mut direct_video_files: Vec<String> = Vec::new();

        for entry in &entries {
            let path = entry.path();
            if path.is_dir() {
                subdirs.push(path);
            } else if path.is_file()
                && let Some(ext) = path.extension().and_then(|e| e.to_str())
                && VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str())
                && let Some(path_str) = path.to_str()
            {
                direct_video_files.push(path_str.to_string());
            }
        }

        // Crear el registro de la serie (padre, parent_id = None)
        let folder_name = std::path::Path::new(folder_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let sort_title = folder_name.to_lowercase();

        let series_id = {
            let mut media_repo = MediaRepository::new(conn);
            let media = media_repo.upsert(&MediaInput {
                r#type: "series".to_string(),
                title: folder_name.clone(),
                sort_title,
                year: None,
                folder_path: folder_path.to_string(),
                parent_id: None,
            })?;
            *media_added += 1;
            media.id
        };

        // Indexar videos directos de esta carpeta como episodios de la serie
        // (siempre, aunque también haya subcarpetas/temporadas)
        if !direct_video_files.is_empty() {
            direct_video_files.sort();
            for file_path_str in direct_video_files.iter() {
                let file_name = std::path::Path::new(file_path_str)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                let file_size = std::fs::metadata(file_path_str)
                    .ok()
                    .and_then(|m| m.len().try_into().ok())
                    .or(None);

                let mut file_repo = crate::db::file_repo::FileRepository::new(conn);
                file_repo.upsert_many(&[crate::db::file_repo::MediaFileInput {
                    media_id: series_id,
                    season: None,
                    episode: None,
                    title: Some(file_name),
                    file_path: file_path_str.clone(),
                    size_bytes: file_size,
                    mtime_ms: None,
                }])?;

                *files_added += 1;
            }
        }

        // Si hay subcarpetas, cada una es una temporada (hijo de la serie)
        for subdir in subdirs {
            if let Some(subdir_str) = subdir.to_str() {
                Self::index_season_folder(
                    conn,
                    subdir_str,
                    Some(series_id),
                    media_added,
                    files_added,
                    folders_processed,
                )?;
            }
        }

        Ok(())
    }

    /// Indexa una carpeta como temporada (hijo de una serie).
    /// Los videos dentro de esta carpeta son los episodios de la temporada.
    fn index_season_folder(
        conn: &mut rusqlite::Connection,
        folder_path: &str,
        parent_series_id: Option<i64>,
        media_added: &mut i64,
        files_added: &mut i64,
        folders_processed: &mut i64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let entries: Vec<_> = std::fs::read_dir(folder_path)?
            .filter_map(|e| e.ok())
            .collect();

        let mut video_files: Vec<String> = Vec::new();

        for entry in &entries {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|e| e.to_str())
                && VIDEO_EXTENSIONS.contains(&ext.to_lowercase().as_str())
                && let Some(path_str) = path.to_str()
            {
                video_files.push(path_str.to_string());
            }
        }

        if video_files.is_empty() {
            return Ok(());
        }

        *folders_processed += 1;
        video_files.sort();

        let folder_name = std::path::Path::new(folder_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let sort_title = folder_name.to_lowercase();

        let season_id = {
            let mut media_repo = MediaRepository::new(conn);
            let media = media_repo.upsert(&MediaInput {
                r#type: "series".to_string(),
                title: folder_name.clone(),
                sort_title,
                year: None,
                folder_path: folder_path.to_string(),
                parent_id: parent_series_id,
            })?;
            *media_added += 1;
            media.id
        };

        for (idx, file_path_str) in video_files.iter().enumerate() {
            let file_name = std::path::Path::new(file_path_str)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            let file_size = std::fs::metadata(file_path_str)
                .ok()
                .and_then(|m| m.len().try_into().ok())
                .or(None);

            let mut file_repo = crate::db::file_repo::FileRepository::new(conn);
            file_repo.upsert_many(&[crate::db::file_repo::MediaFileInput {
                media_id: season_id,
                season: None,
                episode: Some(idx as i32 + 1),
                title: Some(file_name),
                file_path: file_path_str.clone(),
                size_bytes: file_size,
                mtime_ms: None,
            }])?;

            *files_added += 1;
        }

        Ok(())
    }
}

pub fn cleanup_media_not_in_folders(
    db_path: &str,
    folders: &[String],
) -> Result<usize, Box<dyn std::error::Error>> {
    let conn = open_db(db_path)?;

    // Obtener IDs de medios que no están en ninguna de las carpetas actuales.
    let to_delete: Vec<i64> = {
        let mut stmt = conn.prepare("SELECT id, folder_path FROM media")?;
        let mut ids = Vec::new();
        for row in stmt
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })?
            .flatten()
        {
            let (media_id, folder_path) = row;
            let in_any_folder = folders.iter().any(|f| folder_path.starts_with(f));
            if !in_any_folder {
                ids.push(media_id);
            }
        }
        ids
    }; // stmt se suelta aquí

    // Eliminar archivos, historial y medios huérfanos
    for media_id in &to_delete {
        conn.execute("DELETE FROM media_files WHERE media_id = ?1", [media_id])?;
        conn.execute(
            "DELETE FROM playback_history WHERE media_id = ?1",
            [media_id],
        )?;
        conn.execute("DELETE FROM media WHERE id = ?1", [media_id])?;
    }

    // También eliminar medios cuya carpeta sea EXACTAMENTE una carpeta configurada
    // (esto ocurre si la carpeta raíz fue tratada erróneamente como serie en versiones anteriores)
    for folder in folders {
        conn.execute("DELETE FROM media_files WHERE media_id IN (SELECT id FROM media WHERE folder_path = ?1)", [folder])?;
        conn.execute(
            "DELETE FROM playback_history WHERE media_id IN (SELECT id FROM media WHERE folder_path = ?1)",
            [folder],
        )?;
        conn.execute("DELETE FROM media WHERE folder_path = ?1", [folder])?;
    }

    close_db(conn);

    Ok(to_delete.len())
}

#[derive(Debug, Clone)]
pub struct IndexResult {
    pub media_added: i64,
    pub files_added: i64,
    pub folders_processed: i64,
}

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "webm", "mov", "flv", "wmv"];
