//! Repositorio de archivos (`media_files`).
//!
//! Puerto de `src/core/db/repositories/fileRepo.ts` a Rust sobre `rusqlite`.
//! Recibe la conexión SQLite como parámetro (inyección de dependencias).

use crate::db::Connection;
use rusqlite::{params, Result as RusqliteResult};

/// Entidad de dominio: un archivo de medios indexado.
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

/// Datos de entrada para insertar un archivo.
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

/// Repositorio de archivos sobre una conexión SQLite dada.
pub struct FileRepository<'a> {
    conn: &'a Connection,
}

impl<'a> FileRepository<'a> {
    /// Crea el repositorio sobre la conexión dada.
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Inserta o actualiza varios archivos en una transacción. Devuelve la
    /// cantidad de archivos procesados.
    pub fn upsert_many(&self, files: &[MediaFileInput]) -> RusqliteResult<usize> {
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

    /// Busca un archivo por ruta.
    pub fn find_by_path(&self, file_path: &str) -> RusqliteResult<Option<MediaFile>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, media_id, season, episode, title, file_path, size_bytes, mtime_ms, indexed_at
             FROM media_files WHERE file_path = ?1",
        )?;
        let row = stmt.query_row(params![file_path], |row| map_file_row(row));
        match row {
            Ok(file) => Ok(Some(file)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Lista archivos de un título, ordenados por temporada y episodio.
    pub fn list_by_media(&self, media_id: i64) -> RusqliteResult<Vec<MediaFile>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, media_id, season, episode, title, file_path, size_bytes, mtime_ms, indexed_at
             FROM media_files WHERE media_id = ?1 ORDER BY season, episode, file_path",
        )?;
        let rows: Vec<MediaFile> = stmt
            .query_map(params![media_id], |row| map_file_row(row))?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    /// Lista todas las rutas de archivo indexadas.
    pub fn list_all_paths(&self) -> RusqliteResult<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT file_path FROM media_files")?;
        let rows: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    /// Elimina archivos de un título. Devuelve cuántos se eliminaron.
    pub fn remove_by_media(&self, media_id: i64) -> RusqliteResult<i64> {
        let changes = self
            .conn
            .execute("DELETE FROM media_files WHERE media_id = ?1", params![media_id])?;
        Ok(changes)
    }

    /// Elimina los archivos que ya no están en disco (rutas no conocidas).
    /// Se usa tras una indexación completa para limpiar huérfanos.
    pub fn remove_missing(&self, known_paths: &[String]) -> RusqliteResult<i64> {
        let all = self.list_all_paths()?;
        let orphans: Vec<String> = all
            .into_iter()
            .filter(|p| !known_paths.contains(p))
            .collect();

        if orphans.is_empty() {
            return Ok(0);
        }

        let tx = self.conn.transaction()?;
        for path in &orphans {
            tx.execute("DELETE FROM media_files WHERE file_path = ?1", params![path])?;
        }
        tx.commit()?;
        Ok(orphans.len() as i64)
    }

    /// Cuenta archivos, opcionalmente de un título.
    pub fn count(&self, media_id: Option<i64>) -> RusqliteResult<i64> {
        match media_id {
            Some(id) => {
                let row: i64 = self
                    .conn
                    .query_row("SELECT COUNT(*) FROM media_files WHERE media_id = ?1", params![id], |row| {
                        row.get(0)
                    })?;
                Ok(row)
            }
            None => {
                let row: i64 = self.conn.query_row("SELECT COUNT(*) FROM media_files", [], |row| row.get(0))?;
                Ok(row)
            }
        }
    }

    /// Actualiza la ruta de un archivo (rename/move). Devuelve true si se actualizó.
    pub fn update_file_path(&self, old_path: &str, new_path: &str) -> RusqliteResult<bool> {
        let changes = self.conn.execute(
            "UPDATE media_files SET file_path = ?1, indexed_at = datetime('now') WHERE file_path = ?2",
            params![new_path, old_path],
        )?;
        Ok(changes > 0)
    }
}

/// Convierte una fila de SQLite a la entidad MediaFile.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_db;

    fn new_repo() -> (FileRepository<'static>, MediaRepository<'static>) {
        let db_path = ":memory:".to_string();
        let conn = open_db(&db_path).expect("debe abrir BD en memoria");
        let conn_ref: &'static Connection = Box::leak(Box::new(conn));
        (FileRepository::new(conn_ref), MediaRepository::new(conn_ref))
    }

    #[test]
    fn upsert_many_y_list_by_media() {
        let (file_repo, media_repo) = new_repo();
        let media = media_repo
            .upsert(&MediaInput {
                r#type: "series".to_string(),
                title: "Breaking Bad".to_string(),
                sort_title: "breaking bad".to_string(),
                year: Some(2008),
                folder_path: "/media/series/breaking-bad".to_string(),
            })
            .expect("insertar media");

        file_repo
            .upsert_many(&[
                MediaFileInput {
                    media_id: media.id,
                    season: Some(1),
                    episode: Some(1),
                    title: Some("Piloto".to_string()),
                    file_path: "/media/series/breaking-bad/s01e01.mkv".to_string(),
                    size_bytes: Some(1_000_000),
                    mtime_ms: None,
                },
                MediaFileInput {
                    media_id: media.id,
                    season: Some(1),
                    episode: Some(2),
                    title: Some("Capítulo 2".to_string()),
                    file_path: "/media/series/breaking-bad/s01e02.mkv".to_string(),
                    size_bytes: Some(1_100_000),
                    mtime_ms: None,
                },
            ])
            .expect("insertar archivos");

        let files = file_repo.list_by_media(media.id).expect("listar");
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].episode, Some(1));
        assert_eq!(files[1].episode, Some(2));

        let count = file_repo.count(Some(media.id)).expect("contar");
        assert_eq!(count, 2);
    }

    #[test]
    fn remove_missing_limpias_huerfanos() {
        let (file_repo, media_repo) = new_repo();
        let media = media_repo
            .upsert(&MediaInput {
                r#type: "movie".to_string(),
                title: "Test".to_string(),
                sort_title: "test".to_string(),
                year: None,
                folder_path: "/media/movies/test".to_string(),
            })
            .expect("insertar media");

        file_repo
            .upsert_many(&[MediaFileInput {
                media_id: media.id,
                season: None,
                episode: None,
                title: None,
                file_path: "/media/movies/test.mkv".to_string(),
                size_bytes: None,
                mtime_ms: None,
            }])
            .expect("insertar archivo");

        let borrados = file_repo
            .remove_missing(&["/media/movies/otro.mkv".to_string()])
            .expect("limpiar");
        assert_eq!(borrados, 1);

        let restante = file_repo.find_by_path("/media/movies/test.mkv").expect("buscar");
        assert!(restante.is_none());
    }
}
