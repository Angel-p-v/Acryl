//! Repositorio de archivos multimedia.

use crate::db::media_repo::MediaRepository;
use rusqlite::{Connection, Result as RusqliteResult, params};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MediaFile {
    pub id: i64,
    pub media_id: i64,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub title: Option<String>,
    pub file_path: String,
    pub size_bytes: Option<i64>,
    pub mtime_ms: Option<i64>,
    pub indexed_at: String,
}

#[derive(Debug, Clone)]
pub struct MediaFileInput {
    pub media_id: i64,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub title: Option<String>,
    pub file_path: String,
    pub size_bytes: Option<i64>,
    pub mtime_ms: Option<i64>,
}

pub struct FileRepository<'a> {
    conn: &'a mut Connection,
}

impl<'a> FileRepository<'a> {
    pub fn new(conn: &'a mut Connection) -> Self {
        Self { conn }
    }

    pub fn upsert_many(&mut self, files: &[MediaFileInput]) -> RusqliteResult<usize> {
        if files.is_empty() {
            return Ok(0);
        }

        let tx = self.conn.transaction()?;
        for file in files {
            tx.execute(
                "INSERT INTO media_files (media_id, season, episode, title, file_path, size_bytes, mtime_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(file_path) DO UPDATE SET
                   media_id = excluded.media_id,
                   season = excluded.season,
                   episode = excluded.episode,
                   title = excluded.title,
                   file_path = excluded.file_path,
                   size_bytes = excluded.size_bytes,
                   mtime_ms = excluded.mtime_ms,
                   indexed_at = datetime('now')",
                params![
                    file.media_id,
                    file.season,
                    file.episode,
                    file.title,
                    file.file_path,
                    file.size_bytes,
                    file.mtime_ms
                ],
            )?;
        }
        tx.commit()?;
        Ok(files.len())
    }

    pub fn list_by_media(&mut self, media_id: i64) -> RusqliteResult<Vec<MediaFile>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, media_id, season, episode, title, file_path, size_bytes, mtime_ms, indexed_at
             FROM media_files WHERE media_id = ?1 ORDER BY season, episode, file_path",
        )?;
        let rows: Vec<MediaFile> = stmt
            .query_map(params![media_id], map_file_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }
}

fn map_file_row(row: &rusqlite::Row<'_>) -> RusqliteResult<MediaFile> {
    Ok(MediaFile {
        id: row.get(0)?,
        media_id: row.get(1)?,
        season: row.get(2)?,
        episode: row.get(3)?,
        title: row.get(4)?,
        file_path: row.get(5)?,
        size_bytes: row.get(6)?,
        mtime_ms: row.get(7)?,
        indexed_at: row.get(8)?,
    })
}
