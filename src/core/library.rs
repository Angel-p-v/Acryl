//! Capa de dominio: servicios de negocio.

use crate::db::{open_db, close_db, MediaRepository};
use crate::db::media_repo::MediaInput;

pub struct LibraryService;

impl LibraryService {
    pub fn index_folder(db_path: &str, folder_path: &str) -> Result<IndexResult, Box<dyn std::error::Error>> {
        let mut conn = open_db(db_path)?;
        let mut media_repo = MediaRepository::new(&mut conn);

        let mut media_added = 0i64;
        let mut files_added = 0i64;
        let mut folders_processed = 0i64;

        for entry in std::fs::read_dir(folder_path)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_dir() {
                continue;
            }

            folders_processed += 1;

            let mut video_files: Vec<String> = Vec::new();
            if let Ok(read_dir) = std::fs::read_dir(&path) {
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

            video_files.sort();

            let folder_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
            let r#type = if folder_name.contains("anime") {
                "anime"
            } else if folder_name.contains("serie") || folder_name.contains("series") {
                "series"
            } else {
                "movie"
            };

            let title = path.file_name().unwrap_or_default().to_string_lossy().to_string();
            let sort_title = title.to_lowercase();

            let media = media_repo.upsert(&MediaInput {
                r#type: r#type.to_string(),
                title: title.clone(),
                sort_title,
                year: None,
                folder_path: path.to_string_lossy().to_string(),
            })?;

            media_added += 1;
            let media_id = media.id;

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

                let mut file_repo = crate::db::file_repo::FileRepository::new(&mut conn);
                file_repo.upsert_many(&[crate::db::file_repo::MediaFileInput {
                    media_id,
                    season: None,
                    episode: None,
                    title: Some(file_name),
                    file_path: file_path_str.clone(),
                    size_bytes: file_size,
                    mtime_ms: None,
                }])?;

                files_added += 1;

                if r#type == "movie" && idx > 0 {
                    break;
                }
            }
        }

        close_db(conn);

        Ok(IndexResult {
            media_added,
            files_added,
            folders_processed,
        })
    }

    pub fn list_media(db_path: &str, r#type: Option<String>, limit: Option<i64>, offset: Option<i64>) -> Result<Vec<crate::db::media_repo::Media>, Box<dyn std::error::Error>> {
        let mut conn = open_db(db_path)?;
        let mut media_repo = MediaRepository::new(&mut conn);
        let media = media_repo.list(crate::db::media_repo::ListMediaOptions {
            r#type,
            limit,
            offset,
            order_by: None,
        })?;
        close_db(conn);
        Ok(media)
    }

    pub fn search_media(db_path: &str, query: &str, r#type: Option<String>, limit: Option<i64>) -> Result<Vec<crate::db::media_repo::Media>, Box<dyn std::error::Error>> {
        let mut conn = open_db(db_path)?;
        let mut media_repo = MediaRepository::new(&mut conn);
        let results = media_repo.search(query, crate::db::media_repo::SearchMediaOptions {
            r#type,
            limit,
        })?;
        close_db(conn);
        Ok(results)
    }

    pub fn toggle_favorite(db_path: &str, media_id: i64) -> Result<bool, Box<dyn std::error::Error>> {
        let mut conn = open_db(db_path)?;
        let mut media_repo = MediaRepository::new(&mut conn);
        let value = media_repo.toggle_favorite(media_id)?;
        close_db(conn);
        Ok(value)
    }

    pub fn toggle_watched(db_path: &str, media_id: i64) -> Result<bool, Box<dyn std::error::Error>> {
        let mut conn = open_db(db_path)?;
        let mut media_repo = MediaRepository::new(&mut conn);
        let value = media_repo.toggle_watched(media_id)?;
        close_db(conn);
        Ok(value)
    }
}

#[derive(Debug, Clone)]
pub struct IndexResult {
    pub media_added: i64,
    pub files_added: i64,
    pub folders_processed: i64,
}

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "webm", "mov", "flv", "wmv"];
