//! Repositorio de medios.

use crate::db::file_repo::FileRepository;
use rusqlite::{Connection, Result as RusqliteResult, params};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Media {
    pub id: i64,
    pub r#type: String,
    pub title: String,
    pub sort_title: String,
    pub year: Option<i32>,
    pub folder_path: String,
    pub created_at: String,
    pub updated_at: String,
    pub poster_path: Option<String>,
    pub synopsis: Option<String>,
    pub metadata_fetched_at: Option<String>,
    pub is_favorite: bool,
    pub is_watched: bool,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct MediaInput {
    pub r#type: String,
    pub title: String,
    pub sort_title: String,
    pub year: Option<i32>,
    pub folder_path: String,
    pub parent_id: Option<i64>,
}

#[derive(Debug, Clone, Default)]
pub struct ListMediaOptions {
    pub r#type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub order_by: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SearchMediaOptions {
    pub r#type: Option<String>,
    pub limit: Option<i64>,
}

pub struct MediaRepository<'a> {
    conn: &'a mut Connection,
}

impl<'a> MediaRepository<'a> {
    pub fn new(conn: &'a mut Connection) -> Self {
        Self { conn }
    }

    pub fn upsert(&mut self, input: &MediaInput) -> RusqliteResult<Media> {
        let insert_result = self.conn.execute(
            "INSERT INTO media (type, title, sort_title, year, folder_path, parent_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                input.r#type,
                input.title,
                input.sort_title,
                input.year,
                input.folder_path,
                input.parent_id
            ],
        );

        let id: i64 = match insert_result {
            Ok(_) => self.conn.last_insert_rowid(),
            Err(rusqlite::Error::SqliteFailure(_, _)) => {
                self.conn.execute(
                    "UPDATE media SET type = ?1, title = ?2, sort_title = ?3,
                     year = ?4, updated_at = datetime('now'), parent_id = ?6
                     WHERE folder_path = ?5",
                    params![
                        input.r#type,
                        input.title,
                        input.sort_title,
                        input.year,
                        input.folder_path,
                        input.parent_id
                    ],
                )?;
                self.conn.query_row(
                    "SELECT id FROM media WHERE folder_path = ?1",
                    params![input.folder_path],
                    |row| row.get(0),
                )?
            }
            Err(e) => return Err(e),
        };

        self.find_by_id(id)?
            .ok_or_else(|| rusqlite::Error::QueryReturnedNoRows)
    }

    pub fn find_by_id(&mut self, id: i64) -> RusqliteResult<Option<Media>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, type, title, sort_title, year, folder_path, created_at, updated_at,
                    is_favorite, is_watched, poster_path, synopsis, metadata_fetched_at, parent_id
             FROM media WHERE id = ?1",
        )?;
        let row = stmt.query_row(params![id], map_media_row);
        match row {
            Ok(media) => Ok(Some(media)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn list(&mut self, options: ListMediaOptions) -> RusqliteResult<Vec<Media>> {
        let mut sql = String::from(
            "SELECT id, type, title, sort_title, year, folder_path, created_at, updated_at,
                    is_favorite, is_watched, poster_path, synopsis, metadata_fetched_at, parent_id
             FROM media",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        // Solo mostrar medios padre (no temporadas) en la lista principal
        sql.push_str(" WHERE parent_id IS NULL");

        if let Some(ref t) = options.r#type {
            sql.push_str(" AND type = ?");
            params_vec.push(Box::new(t.clone()));
        }

        let order_clause = match options.order_by.as_deref().unwrap_or("title") {
            "added" => "ORDER BY created_at DESC",
            "recent" => "ORDER BY updated_at DESC",
            _ => "ORDER BY sort_title ASC",
        };
        sql.push(' ');
        sql.push_str(order_clause);
        sql.push_str(" LIMIT ?");
        params_vec.push(Box::new(options.limit.unwrap_or(500)));
        sql.push_str(" OFFSET ?");
        params_vec.push(Box::new(options.offset.unwrap_or(0)));

        let params_slice: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows: Vec<Media> = stmt
            .query_map(params_slice.as_slice(), map_media_row)?
            .filter_map(Result::ok)
            .collect();

        Ok(rows)
    }

    pub fn search(
        &mut self,
        query: &str,
        options: SearchMediaOptions,
    ) -> RusqliteResult<Vec<Media>> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return self.list(ListMediaOptions {
                r#type: options.r#type,
                limit: options.limit,
                ..Default::default()
            });
        }

        let fts_query = to_fts_query(trimmed);
        let mut results: Vec<Media> = Vec::new();

        if !fts_query.is_empty() {
            let mut sql = String::from(
                "SELECT m.id, m.type, m.title, m.sort_title, m.year, m.folder_path,
                        m.created_at, m.updated_at,
                        m.is_favorite, m.is_watched, m.poster_path, m.synopsis,
                        m.metadata_fetched_at, m.parent_id
                 FROM media m
                 JOIN media_fts f ON f.rowid = m.id
                 WHERE media_fts MATCH ?1 AND m.parent_id IS NULL",
            );
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(fts_query)];

            if let Some(ref t) = options.r#type {
                sql.push_str(" AND m.type = ?");
                params.push(Box::new(t.clone()));
            }

            sql.push_str(" ORDER BY bm25(media_fts), m.sort_title LIMIT ?");
            params.push(Box::new(options.limit.unwrap_or(20)));

            let params_slice: Vec<&dyn rusqlite::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();
            let mut stmt = self.conn.prepare(&sql)?;
            let rows: Vec<Media> = stmt
                .query_map(params_slice.as_slice(), map_media_row)?
                .filter_map(Result::ok)
                .collect();
            results.extend(rows);
        }

        if results.is_empty() {
            let like_pattern = format!("%{}%", escape_like(trimmed));
            let mut sql = String::from(
                "SELECT m.id, m.type, m.title, m.sort_title, m.year, m.folder_path,
                        m.created_at, m.updated_at,
                        m.is_favorite, m.is_watched, m.poster_path, m.synopsis,
                        m.metadata_fetched_at, m.parent_id
                 FROM media m
                 WHERE m.sort_title LIKE ?1 ESCAPE '\\' AND m.parent_id IS NULL",
            );
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(like_pattern)];

            if let Some(ref t) = options.r#type {
                sql.push_str(" AND m.type = ?");
                params.push(Box::new(t.clone()));
            }

            sql.push_str(" ORDER BY m.sort_title ASC LIMIT ?");
            params.push(Box::new(options.limit.unwrap_or(20)));

            let params_slice: Vec<&dyn rusqlite::ToSql> =
                params.iter().map(|p| p.as_ref()).collect();
            let mut stmt = self.conn.prepare(&sql)?;
            let rows: Vec<Media> = stmt
                .query_map(params_slice.as_slice(), map_media_row)?
                .filter_map(Result::ok)
                .collect();
            results.extend(rows);
        }

        Ok(results)
    }

    pub fn toggle_favorite(&mut self, id: i64) -> RusqliteResult<bool> {
        self.conn.execute(
            "UPDATE media SET is_favorite = NOT is_favorite, updated_at = datetime('now') WHERE id = ?1",
            params![id],
        )?;
        let value: bool = self.conn.query_row(
            "SELECT is_favorite FROM media WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(value)
    }

    pub fn toggle_watched(&mut self, id: i64) -> RusqliteResult<bool> {
        self.conn.execute(
            "UPDATE media SET is_watched = NOT is_watched, updated_at = datetime('now') WHERE id = ?1",
            params![id],
        )?;
        let value: bool = self.conn.query_row(
            "SELECT is_watched FROM media WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        Ok(value)
    }

    /// Obtiene las temporadas (hijos) de una serie padre.
    pub fn list_children(&mut self, parent_id: i64) -> RusqliteResult<Vec<Media>> {
        let sql = String::from(
            "SELECT id, type, title, sort_title, year, folder_path, created_at, updated_at,
                    is_favorite, is_watched, poster_path, synopsis, metadata_fetched_at, parent_id
             FROM media WHERE parent_id = ?1 ORDER BY sort_title ASC",
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows: Vec<Media> = stmt
            .query_map(params![parent_id], map_media_row)?
            .filter_map(Result::ok)
            .collect();

        Ok(rows)
    }

    pub fn list_folders(&mut self) -> RusqliteResult<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT folder_path FROM media ORDER BY sort_title")?;
        let rows: Vec<String> = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    pub fn list_files_by_media(
        &mut self,
        media_id: i64,
    ) -> RusqliteResult<Vec<crate::db::file_repo::MediaFile>> {
        let mut file_repo = FileRepository::new(self.conn);
        file_repo.list_by_media(media_id)
    }

    pub fn update_metadata(
        &mut self,
        media_id: i64,
        year: Option<i32>,
        poster_path: Option<&str>,
        synopsis: Option<&str>,
    ) -> RusqliteResult<()> {
        self.conn.execute(
            "UPDATE media SET year = COALESCE(?1, year), poster_path = ?2, synopsis = ?3,
             metadata_fetched_at = datetime('now'), updated_at = datetime('now')
             WHERE id = ?4",
            params![year, poster_path, synopsis, media_id],
        )?;
        Ok(())
    }

    pub fn record_playback(
        &mut self,
        media_id: i64,
        position: f64,
        duration: f64,
    ) -> RusqliteResult<()> {
        self.conn.execute(
            "INSERT INTO playback_history (media_id, position, duration) VALUES (?1, ?2, ?3)",
            params![media_id, position, duration],
        )?;
        Ok(())
    }

    pub fn get_details(&mut self, media_id: i64) -> RusqliteResult<Option<MediaDetails>> {
        let media = self.find_by_id(media_id)?;
        let media = match media {
            Some(m) => m,
            None => return Ok(None),
        };
        let mut file_repo = FileRepository::new(self.conn);
        let files = file_repo.list_by_media(media_id)?;
        Ok(Some(MediaDetails { media, files }))
    }

    pub fn find_media_id_by_file(&mut self, file_path: &str) -> RusqliteResult<Option<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT media_id FROM media_files WHERE file_path = ?1 LIMIT 1")?;
        let row = stmt.query_row(params![file_path], |row| row.get(0));
        match row {
            Ok(id) => Ok(Some(id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Inserta o reemplaza los streams de un archivo. Elimina los streams previos
    /// de ese file_path y vuelve a insertar los nuevos (idempotente por archivo).
    pub fn replace_streams_for_file(
        &mut self,
        media_id: i64,
        file_path: &str,
        streams: &[NewMediaStream],
    ) -> RusqliteResult<()> {
        let tx = self.conn.transaction()?;

        tx.execute(
            "DELETE FROM media_streams WHERE file_path = ?1",
            params![file_path],
        )?;

        for stream in streams {
            tx.execute(
                "INSERT INTO media_streams (media_id, file_path, stream_type, stream_index, language, codec, title)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    media_id,
                    file_path,
                    stream.stream_type,
                    stream.stream_index,
                    stream.language,
                    stream.codec,
                    stream.title,
                ],
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    /// Lista los streams de un medio (todos sus archivos).
    pub fn list_streams_by_media(&mut self, media_id: i64) -> RusqliteResult<Vec<MediaStream>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, media_id, file_path, stream_type, stream_index, language, codec, title
             FROM media_streams WHERE media_id = ?1
             ORDER BY stream_type, stream_index",
        )?;
        let rows: Vec<MediaStream> = stmt
            .query_map(params![media_id], map_media_stream_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    /// Lista los streams de un archivo concreto.
    pub fn list_streams_by_file(&mut self, file_path: &str) -> RusqliteResult<Vec<MediaStream>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, media_id, file_path, stream_type, stream_index, language, codec, title
             FROM media_streams WHERE file_path = ?1
             ORDER BY stream_type, stream_index",
        )?;
        let rows: Vec<MediaStream> = stmt
            .query_map(params![file_path], map_media_stream_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }
}

/// Datos para insertar un stream de audio o subtítulo de un archivo.
#[derive(Debug, Clone)]
pub struct NewMediaStream {
    pub file_path: String,
    pub stream_type: String, // "audio" | "subtitle"
    pub stream_index: i32,   // índice relativo al tipo (para --aid/--sid de mpv)
    pub language: Option<String>,
    pub codec: Option<String>,
    pub title: Option<String>,
}

/// Stream de audio o subtítulo leído de la base de datos.
#[derive(Debug, Clone)]
pub struct MediaStream {
    pub id: i64,
    pub media_id: i64,
    pub file_path: String,
    pub stream_type: String,
    pub stream_index: i32,
    pub language: Option<String>,
    pub codec: Option<String>,
    pub title: Option<String>,
}

fn map_media_stream_row(row: &rusqlite::Row<'_>) -> RusqliteResult<MediaStream> {
    Ok(MediaStream {
        id: row.get(0)?,
        media_id: row.get(1)?,
        file_path: row.get(2)?,
        stream_type: row.get(3)?,
        stream_index: row.get(4)?,
        language: row.get(5)?,
        codec: row.get(6)?,
        title: row.get(7)?,
    })
}

#[derive(Debug, Clone)]
pub struct MediaDetails {
    pub media: Media,
    pub files: Vec<crate::db::file_repo::MediaFile>,
}

fn map_media_row(row: &rusqlite::Row<'_>) -> RusqliteResult<Media> {
    Ok(Media {
        id: row.get(0)?,
        r#type: row.get(1)?,
        title: row.get(2)?,
        sort_title: row.get(3)?,
        year: row.get(4)?,
        folder_path: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
        is_favorite: row.get(8)?,
        is_watched: row.get(9)?,
        poster_path: row.get(10)?,
        synopsis: row.get(11)?,
        metadata_fetched_at: row.get(12)?,
        parent_id: row.get(13)?,
    })
}

fn to_fts_query(query: &str) -> String {
    let terms: Vec<&str> = query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect();

    if terms.is_empty() {
        return String::new();
    }

    terms
        .iter()
        .map(|t| format!("\"{}\"*", t))
        .collect::<Vec<_>>()
        .join(" OR ")
}

fn escape_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        if matches!(c, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_db;
    use tempfile::TempDir;

    fn temp_db() -> (String, TempDir) {
        let dir = TempDir::new().expect("crear directorio temporal");
        let path = dir.path().join("test.db");
        (path.to_str().unwrap().to_string(), dir)
    }

    fn setup_media_and_file(conn: &mut rusqlite::Connection) -> i64 {
        // Migración mínima para probar: crear tablas necesarias
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS media (
                id          INTEGER PRIMARY KEY,
                type        TEXT    NOT NULL,
                title       TEXT    NOT NULL,
                sort_title  TEXT    NOT NULL,
                year        INTEGER,
                folder_path TEXT    NOT NULL UNIQUE,
                created_at  TEXT    NOT NULL DEFAULT (datetime('now')),
                updated_at  TEXT    NOT NULL DEFAULT (datetime('now'))
            );
             CREATE TABLE IF NOT EXISTS media_files (
                id          INTEGER PRIMARY KEY,
                media_id    INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
                file_path   TEXT    NOT NULL UNIQUE
            );
             CREATE TABLE IF NOT EXISTS media_streams (
                id          INTEGER PRIMARY KEY,
                media_id    INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
                file_path   TEXT    NOT NULL,
                stream_type TEXT    NOT NULL CHECK (stream_type IN ('audio', 'subtitle')),
                stream_index INTEGER NOT NULL,
                language    TEXT,
                codec       TEXT,
                title       TEXT,
                indexed_at  TEXT    NOT NULL DEFAULT (datetime('now'))
             )",
        )
        .unwrap();

        conn.execute(
            "INSERT INTO media (type, title, sort_title, folder_path) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params!["movie", "Test Movie", "test movie", "/tmp/test"],
        )
        .unwrap();
        let media_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO media_files (media_id, file_path) VALUES (?1, ?2)",
            rusqlite::params![media_id, "/tmp/test/video.mkv"],
        )
        .unwrap();

        media_id
    }

    #[test]
    fn replace_streams_for_file_y_list_streams_por_media() {
        let (db_path, _dir) = temp_db();
        let mut conn = open_db(&db_path).expect("abrir BD");
        let media_id = setup_media_and_file(&mut conn);
        let mut repo = MediaRepository::new(&mut conn);

        let streams = vec![
            NewMediaStream {
                file_path: "/tmp/test/video.mkv".to_string(),
                stream_type: "audio".to_string(),
                stream_index: 0,
                language: Some("spa".to_string()),
                codec: Some("aac".to_string()),
                title: Some("Español".to_string()),
            },
            NewMediaStream {
                file_path: "/tmp/test/video.mkv".to_string(),
                stream_type: "audio".to_string(),
                stream_index: 1,
                language: Some("eng".to_string()),
                codec: Some("opus".to_string()),
                title: Some("English".to_string()),
            },
            NewMediaStream {
                file_path: "/tmp/test/video.mkv".to_string(),
                stream_type: "subtitle".to_string(),
                stream_index: 0,
                language: Some("spa".to_string()),
                codec: Some("subrip".to_string()),
                title: Some("Subtítulos ES".to_string()),
            },
        ];

        repo.replace_streams_for_file(media_id, "/tmp/test/video.mkv", &streams)
            .expect("insertar streams");

        // Listar por media
        let by_media = repo
            .list_streams_by_media(media_id)
            .expect("listar por media");
        assert_eq!(by_media.len(), 3);
        let audios: Vec<_> = by_media
            .iter()
            .filter(|s| s.stream_type == "audio")
            .collect();
        assert_eq!(audios.len(), 2);
        assert_eq!(audios[0].stream_index, 0);
        assert_eq!(audios[1].stream_index, 1);

        // Listar por archivo
        let by_file = repo
            .list_streams_by_file("/tmp/test/video.mkv")
            .expect("listar por archivo");
        assert_eq!(by_file.len(), 3);
    }

    #[test]
    fn escape_like_conserva_unicode_y_escapa_comodines() {
        assert_eq!(
            escape_like("película 100%_x\\y"),
            "película 100\\%\\_x\\\\y"
        );
    }

    #[test]
    fn replace_streams_for_file_es_idempotente() {
        let (db_path, _dir) = temp_db();
        let mut conn = open_db(&db_path).expect("abrir BD");
        let media_id = setup_media_and_file(&mut conn);
        let mut repo = MediaRepository::new(&mut conn);

        let streams_v1 = vec![NewMediaStream {
            file_path: "/tmp/test/video.mkv".to_string(),
            stream_type: "audio".to_string(),
            stream_index: 0,
            language: Some("spa".to_string()),
            codec: None,
            title: None,
        }];
        repo.replace_streams_for_file(media_id, "/tmp/test/video.mkv", &streams_v1)
            .expect("insertar v1");

        let streams_v2 = vec![
            NewMediaStream {
                file_path: "/tmp/test/video.mkv".to_string(),
                stream_type: "audio".to_string(),
                stream_index: 0,
                language: Some("eng".to_string()),
                codec: None,
                title: None,
            },
            NewMediaStream {
                file_path: "/tmp/test/video.mkv".to_string(),
                stream_type: "subtitle".to_string(),
                stream_index: 0,
                language: None,
                codec: None,
                title: None,
            },
        ];
        repo.replace_streams_for_file(media_id, "/tmp/test/video.mkv", &streams_v2)
            .expect("reemplazar v2");

        let by_file = repo
            .list_streams_by_file("/tmp/test/video.mkv")
            .expect("listar");
        assert_eq!(by_file.len(), 2);
        assert!(!by_file.iter().any(|s| s.language.as_deref() == Some("spa")));
        assert!(by_file.iter().any(|s| s.stream_type == "subtitle"));
    }
}
