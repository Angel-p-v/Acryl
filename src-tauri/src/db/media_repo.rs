//! Repositorio de títulos (`media`).
//!
//! Puerto de `src/core/db/repositories/mediaRepo.ts` a Rust sobre `rusqlite`.
//! Recibe la conexión SQLite como parámetro (inyección de dependencias), sin
//! estado global.

use crate::db::file_repo::FileRepository;
use rusqlite::{Connection, Result as RusqliteResult, params};

/// Entidad de dominio: un título de la biblioteca (película, serie o anime).
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
    pub is_favorite: bool,
    pub is_watched: bool,
    pub poster_path: Option<String>,
    pub synopsis: Option<String>,
    pub metadata_fetched_at: Option<String>,
}

/// Detalle completo de un título: el medio más sus archivos indexados.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MediaDetails {
    pub media: Media,
    pub files: Vec<crate::db::file_repo::MediaFile>,
}

/// Datos de entrada para insertar o actualizar un título.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MediaInput {
    pub r#type: String,
    pub title: String,
    pub sort_title: String,
    pub year: Option<i32>,
    pub folder_path: String,
}

/// Opciones de listado de títulos.
#[derive(Debug, Clone, Default)]
pub struct ListMediaOptions {
    pub r#type: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub order_by: Option<String>,
}

/// Opciones de búsqueda de títulos.
#[derive(Debug, Clone, Default)]
pub struct SearchMediaOptions {
    pub r#type: Option<String>,
    pub limit: Option<i64>,
}

/// Repositorio de títulos sobre una conexión SQLite dada.
pub struct MediaRepository<'a> {
    conn: &'a mut Connection,
}

impl<'a> MediaRepository<'a> {
    #[allow(dead_code)]
    /// Crea el repositorio sobre la conexión dada.
    pub fn new(conn: &'a mut Connection) -> Self {
        Self { conn }
    }

    #[allow(dead_code)]
    /// Inserta o actualiza un título y devuelve la fila guardada.
    pub fn upsert(&mut self, input: &MediaInput) -> RusqliteResult<Media> {
        let sort_title = input.sort_title.clone();

        // Intentar INSERT simple. Si falla por violación de UNIQUE en
        // folder_path, hacemos UPDATE sobre la fila existente.
        let insert_result = self.conn.execute(
            "INSERT INTO media (type, title, sort_title, year, folder_path)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                input.r#type,
                input.title,
                sort_title,
                input.year,
                input.folder_path
            ],
        );

        let id: i64 = match insert_result {
            Ok(_) => self.conn.last_insert_rowid(),
            Err(rusqlite::Error::SqliteFailure(_, _)) => {
                self.conn.execute(
                    "UPDATE media SET type = ?1, title = ?2, sort_title = ?3,
                     year = ?4, updated_at = datetime('now')
                     WHERE folder_path = ?5",
                    params![
                        input.r#type,
                        input.title,
                        sort_title,
                        input.year,
                        input.folder_path
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

    /// Busca un título por su id.
    pub fn find_by_id(&mut self, id: i64) -> RusqliteResult<Option<Media>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, type, title, sort_title, year, folder_path, created_at, updated_at,
                    is_favorite, is_watched, poster_path, synopsis, metadata_fetched_at
             FROM media WHERE id = ?1",
        )?;
        let row = stmt.query_row(params![id], map_media_row);
        match row {
            Ok(media) => Ok(Some(media)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Busca un título por ruta de carpeta.
    pub fn find_by_folder_path(&mut self, folder_path: &str) -> RusqliteResult<Option<Media>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, type, title, sort_title, year, folder_path, created_at, updated_at,
                    is_favorite, is_watched, poster_path, synopsis, metadata_fetched_at
             FROM media WHERE folder_path = ?1",
        )?;
        let row = stmt.query_row(params![folder_path], map_media_row);
        match row {
            Ok(media) => Ok(Some(media)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Lista títulos con filtros opcionales (tipo, orden, límite, offset).
    pub fn list(&mut self, options: ListMediaOptions) -> RusqliteResult<Vec<Media>> {
        let mut sql = String::from(
            "SELECT id, type, title, sort_title, year, folder_path, created_at, updated_at,
                    is_favorite, is_watched, poster_path, synopsis, metadata_fetched_at
             FROM media",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(ref t) = options.r#type {
            sql.push_str(" WHERE type = ?");
            params_vec.push(Box::new(t.clone()));
        }

        let order_by = options.order_by.as_deref().unwrap_or("title");
        let order_clause = match order_by {
            "added" => "ORDER BY created_at DESC",
            "recent" => "ORDER BY updated_at DESC",
            _ => "ORDER BY sort_title ASC",
        };
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

    /// Busca títulos por texto (FTS5 con fallback LIKE).
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
                        m.metadata_fetched_at
                 FROM media m
                 JOIN media_fts f ON f.rowid = m.id
                 WHERE media_fts MATCH ?1",
            );
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(fts_query)];

            if let Some(ref t) = options.r#type {
                sql.push_str(" AND m.type = ?");
                params.push(Box::new(t.clone()));
            }

            sql.push_str(" ORDER BY bm25(media_fts), m.sort_title");
            sql.push_str(" LIMIT ?");
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

        // Fallback LIKE si FTS5 no devuelve nada.
        if results.is_empty() {
            let like_pattern = format!("%{}%", escape_like(trimmed));
            let mut sql = String::from(
                "SELECT id, type, title, sort_title, year, folder_path, created_at, updated_at,
                        is_favorite, is_watched, poster_path, synopsis, metadata_fetched_at
                 FROM media
                 WHERE sort_title LIKE ?1",
            );
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(like_pattern)];

            if let Some(ref t) = options.r#type {
                sql.push_str(" AND type = ?");
                params.push(Box::new(t.clone()));
            }

            sql.push_str(" ORDER BY sort_title ASC");
            sql.push_str(" LIMIT ?");
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

    #[allow(dead_code)]
    /// Elimina un título por id. Devuelve true si se eliminó.
    pub fn remove(&mut self, id: i64) -> RusqliteResult<bool> {
        let changes = self
            .conn
            .execute("DELETE FROM media WHERE id = ?1", params![id])?;
        Ok(changes > 0)
    }

    #[allow(dead_code)]
    /// Elimina los títulos que se han quedado sin archivos.
    pub fn remove_empty(&mut self) -> RusqliteResult<i64> {
        let changes = self.conn.execute(
            "DELETE FROM media
             WHERE NOT EXISTS (SELECT 1 FROM media_files WHERE media_files.media_id = media.id)",
            [],
        )?;
        Ok(changes as i64)
    }

    /// Cuenta títulos, opcionalmente filtrados por tipo.
    pub fn count(&mut self, r#type: Option<&str>) -> RusqliteResult<i64> {
        match r#type {
            Some(t) => {
                let row: i64 = self.conn.query_row(
                    "SELECT COUNT(*) FROM media WHERE type = ?1",
                    params![t],
                    |row| row.get(0),
                )?;
                Ok(row)
            }
            None => {
                let row: i64 = self
                    .conn
                    .query_row("SELECT COUNT(*) FROM media", [], |row| row.get(0))?;
                Ok(row)
            }
        }
    }

    /// Lista todas las rutas de carpeta.
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

    #[allow(dead_code)]
    /// Actualiza la ruta de una carpeta (rename/move). Devuelve true si se actualizó.
    pub fn update_folder_path(&mut self, old_path: &str, new_path: &str) -> RusqliteResult<bool> {
        let changes = self.conn.execute(
            "UPDATE media SET folder_path = ?1, updated_at = datetime('now') WHERE folder_path = ?2",
            params![new_path, old_path],
        )?;
        Ok(changes > 0)
    }

    /// Invierte el estado de favorito de un título. Devuelve el nuevo valor.
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

    /// Invierte el estado de visto de un título. Devuelve el nuevo valor.
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

    /// Obtiene el detalle completo de un título (medio + archivos indexados).
    pub fn get_details(&mut self, id: i64) -> RusqliteResult<Option<MediaDetails>> {
        let media = self.find_by_id(id)?;
        let media = match media {
            Some(m) => m,
            None => return Ok(None),
        };
        let mut file_repo = FileRepository::new(self.conn);
        let files = file_repo.list_by_media(id)?;
        Ok(Some(MediaDetails { media, files }))
    }

    /// Actualiza los metadatos obtenidos de TMDB/TVDB en un título.
    pub fn update_metadata(
        &mut self,
        id: i64,
        poster_path: Option<&str>,
        synopsis: Option<&str>,
    ) -> RusqliteResult<()> {
        self.conn.execute(
            "UPDATE media
             SET poster_path = ?1, synopsis = ?2, metadata_fetched_at = datetime('now'),
                 updated_at = datetime('now')
             WHERE id = ?3",
            params![poster_path, synopsis, id],
        )?;
        Ok(())
    }

    /// Registra una reproducción en el historial.
    pub fn record_playback(
        &mut self,
        media_id: i64,
        position: f64,
        duration: f64,
    ) -> RusqliteResult<()> {
        self.conn.execute(
            "INSERT INTO playback_history (media_id, played_at, position, duration)
             VALUES (?1, datetime('now'), ?2, ?3)",
            params![media_id, position, duration],
        )?;
        Ok(())
    }

    /// Lista el historial de reproducción (más recientes primero).
    pub fn list_playback_history(
        &mut self,
        limit: Option<i64>,
    ) -> RusqliteResult<Vec<PlaybackRecord>> {
        let mut sql = String::from(
            "SELECT ph.id, ph.media_id, m.title, ph.played_at, ph.position, ph.duration
             FROM playback_history ph
             JOIN media m ON m.id = ph.media_id
             ORDER BY ph.played_at DESC",
        );
        if let Some(l) = limit {
            sql.push_str(" LIMIT ?");
            let mut stmt = self.conn.prepare(&sql)?;
            let rows: Vec<PlaybackRecord> = stmt
                .query_map(params![l], map_playback_row)?
                .filter_map(Result::ok)
                .collect();
            Ok(rows)
        } else {
            let mut stmt = self.conn.prepare(&sql)?;
            let rows: Vec<PlaybackRecord> = stmt
                .query_map([], map_playback_row)?
                .filter_map(Result::ok)
                .collect();
            Ok(rows)
        }
    }
}

/// Registro de reproducción individual.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PlaybackRecord {
    pub id: i64,
    pub media_id: i64,
    pub title: String,
    pub played_at: String,
    pub position: f64,
    pub duration: f64,
}

fn map_playback_row(row: &rusqlite::Row<'_>) -> RusqliteResult<PlaybackRecord> {
    Ok(PlaybackRecord {
        id: row.get(0)?,
        media_id: row.get(1)?,
        title: row.get(2)?,
        played_at: row.get(3)?,
        position: row.get(4)?,
        duration: row.get(5)?,
    })
}

/// Convierte una fila de SQLite a la entidad Media.
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
    })
}

/// Convierte una consulta del usuario en una expresión FTS5 segura.
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

/// Escapa caracteres especiales de LIKE.
fn escape_like(value: &str) -> String {
    value
        .chars()
        .flat_map(|c| match c {
            '\\' | '%' | '_' => format!("\\{}", c).into_bytes().to_vec(),
            _ => vec![c as u8],
        })
        .map(|b| b as char)
        .collect::<String>()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::open_db;

    fn new_repo() -> (MediaRepository<'static>, String) {
        let db_path = ":memory:".to_string();
        let conn = open_db(&db_path).expect("debe abrir BD en memoria");
        // `open_db` devuelve una Connection owned; para los tests la envolvemos
        // en un bloque estático usando Box::leak (solo tests).
        let conn_ref: &'static mut Connection = Box::leak(Box::new(conn));
        (MediaRepository::new(conn_ref), db_path)
    }

    #[test]
    fn upsert_y_find_by_id() {
        let (mut repo, _db) = new_repo();
        let input = MediaInput {
            r#type: "movie".to_string(),
            title: "Interesting".to_string(),
            sort_title: "interesting".to_string(),
            year: Some(2014),
            folder_path: "/media/movies/interesting".to_string(),
        };
        let media = repo.upsert(&input).expect("debe insertar");
        assert!(media.id > 0);

        let found = repo
            .find_by_id(media.id)
            .expect("debe encontrar")
            .expect("debe ser Some");
        assert_eq!(found.title, "Interesting");
        assert_eq!(found.r#type, "movie");
    }

    #[test]
    fn upsert_actualiza_por_folder_path() {
        let (mut repo, _db) = new_repo();
        let input = MediaInput {
            r#type: "movie".to_string(),
            title: "Viejo".to_string(),
            sort_title: "viejo".to_string(),
            year: Some(2000),
            folder_path: "/media/movies/viejo".to_string(),
        };
        let media = repo.upsert(&input).expect("debe insertar");

        let input2 = MediaInput {
            r#type: "movie".to_string(),
            title: "Nuevo".to_string(),
            sort_title: "nuevo".to_string(),
            year: Some(2020),
            folder_path: "/media/movies/viejo".to_string(),
        };
        let updated = repo.upsert(&input2).expect("debe actualizar");
        assert_eq!(updated.id, media.id);
        assert_eq!(updated.title, "Nuevo");

        let count = repo.count(None).expect("debe contar");
        assert_eq!(count, 1);
    }

    #[test]
    fn list_filtra_por_tipo() {
        let (mut repo, _db) = new_repo();
        repo.upsert(&MediaInput {
            r#type: "movie".to_string(),
            title: "Película A".to_string(),
            sort_title: "pelicula a".to_string(),
            year: None,
            folder_path: "/a".to_string(),
        })
        .expect("insertar");
        repo.upsert(&MediaInput {
            r#type: "anime".to_string(),
            title: "Anime B".to_string(),
            sort_title: "anime b".to_string(),
            year: None,
            folder_path: "/b".to_string(),
        })
        .expect("insertar");

        let movies = repo
            .list(ListMediaOptions {
                r#type: Some("movie".to_string()),
                ..Default::default()
            })
            .expect("listar");
        assert_eq!(movies.len(), 1);
        assert_eq!(movies[0].title, "Película A");
    }

    #[test]
    fn search_devuelve_resultados() {
        let (mut repo, _db) = new_repo();
        repo.upsert(&MediaInput {
            r#type: "anime".to_string(),
            title: "Cowboy Bebop".to_string(),
            sort_title: "cowboy bebop".to_string(),
            year: Some(1998),
            folder_path: "/anime/cowboy-bebop".to_string(),
        })
        .expect("insertar");

        let resultados = repo
            .search("bebop", SearchMediaOptions::default())
            .expect("buscar");
        assert!(!resultados.is_empty());
        assert!(resultados.iter().any(|m| m.title == "Cowboy Bebop"));
    }

    #[test]
    fn toggle_favorite_cambia_estado() {
        let (mut repo, _db) = new_repo();
        let media = repo
            .upsert(&MediaInput {
                r#type: "movie".to_string(),
                title: "Test".to_string(),
                sort_title: "test".to_string(),
                year: None,
                folder_path: "/media/movies/test".to_string(),
            })
            .expect("insertar");

        assert!(!media.is_favorite);

        let activado = repo.toggle_favorite(media.id).expect("toggle favorite");
        assert!(activado);

        let desactivado = repo.toggle_favorite(media.id).expect("toggle favorite");
        assert!(!desactivado);
    }

    #[test]
    fn toggle_watched_cambia_estado() {
        let (mut repo, _db) = new_repo();
        let media = repo
            .upsert(&MediaInput {
                r#type: "movie".to_string(),
                title: "Test".to_string(),
                sort_title: "test".to_string(),
                year: None,
                folder_path: "/media/movies/test".to_string(),
            })
            .expect("insertar");

        assert!(!media.is_watched);

        let activado = repo.toggle_watched(media.id).expect("toggle watched");
        assert!(activado);

        let desactivado = repo.toggle_watched(media.id).expect("toggle watched");
        assert!(!desactivado);
    }

    #[test]
    fn get_details_devuelve_media_y_archivos() {
        let conn_ref: &'static mut rusqlite::Connection = {
            let db_path = ":memory:".to_string();
            let conn = open_db(&db_path).expect("debe abrir BD en memoria");
            Box::leak(Box::new(conn))
        };

        let media_id = {
            let mut media_repo = MediaRepository::new(conn_ref);
            let media = media_repo
                .upsert(&MediaInput {
                    r#type: "movie".to_string(),
                    title: "Test Media".to_string(),
                    sort_title: "test media".to_string(),
                    year: None,
                    folder_path: "/media/movies/test".to_string(),
                })
                .expect("insertar media");
            media.id
        };
        // media_repo se descarta aquí: el préstamo termina.

        {
            let mut file_repo = FileRepository::new(conn_ref);
            file_repo
                .upsert_many(&[crate::db::file_repo::MediaFileInput {
                    media_id,
                    season: Some(1),
                    episode: Some(1),
                    title: Some("Piloto".to_string()),
                    file_path: "/media/movies/test/file.mkv".to_string(),
                    size_bytes: Some(1_000_000),
                    mtime_ms: None,
                }])
                .expect("insertar archivo");
        }
        // file_repo se descarta aquí.

        let details = {
            let mut media_repo = MediaRepository::new(conn_ref);
            media_repo.get_details(media_id).expect("obtener detalles")
        };

        let details = details.expect("debe ser Some");
        assert_eq!(details.media.id, media_id);
        assert_eq!(details.media.title, "Test Media");
        assert_eq!(details.files.len(), 1);
        assert_eq!(details.files[0].episode, Some(1));
    }
}
