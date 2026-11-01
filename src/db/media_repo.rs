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
    pub banner_path: Option<String>,
    pub synopsis: Option<String>,
    pub metadata_fetched_at: Option<String>,
    pub is_favorite: bool,
    pub is_watched: bool,
    pub parent_id: Option<i64>,
    pub rating: i32,
    /// Puntuación obtenida de los metadatos (vote_average de TVmaze, 0-10).
    pub vote_average: Option<f64>,
    /// Identificador del show en TVmaze, usado para pedir episodios e imágenes.
    pub metadata_source_id: Option<i64>,
    /// Momento en que se intentó extraer categorías de los metadatos.
    pub categories_fetched_at: Option<String>,
    /// Categorías asociadas al medio (cargadas por separado con `load_categories`).
    #[serde(skip)]
    pub categories: Vec<String>,
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

/// Metadatos completos con los que sobrescribir un medio.
#[derive(Debug, Clone, Default)]
pub struct MediaMetadataInput<'a> {
    pub title: &'a str,
    pub sort_title: &'a str,
    pub year: Option<i32>,
    pub poster_path: Option<&'a str>,
    pub banner_path: Option<&'a str>,
    pub synopsis: Option<&'a str>,
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
                // Reindexar no debe destruir metadatos ya sincronizados: se
                // conservan título, año y orden si `metadata_fetched_at` existe.
                self.conn.execute(
                    "UPDATE media SET
                       type = ?1,
                       title = CASE WHEN metadata_fetched_at IS NOT NULL THEN title ELSE ?2 END,
                       sort_title = CASE WHEN metadata_fetched_at IS NOT NULL THEN sort_title ELSE ?3 END,
                       year = COALESCE(?4, year),
                       updated_at = datetime('now'),
                       parent_id = ?6
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
                    is_favorite, is_watched, poster_path, banner_path, synopsis,
                    metadata_fetched_at, parent_id, rating, vote_average,
                    metadata_source_id, categories_fetched_at
             FROM media WHERE id = ?1",
        )?;
        let row = stmt.query_row(params![id], map_media_row);
        match row {
            Ok(media) => Ok(Some(media)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Obtiene el póster de un medio por su ID.
    pub fn get_poster_by_id(&mut self, id: i64) -> RusqliteResult<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT poster_path FROM media WHERE id = ?1")?;
        let row = stmt.query_row(params![id], |row| row.get::<_, Option<String>>(0));
        match row {
            Ok(poster) => Ok(poster),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Obtiene la ruta del primer archivo de un medio o de sus episodios
    /// (para usarse como objetivo predeterminado al seleccionar pistas).
    pub fn first_file_path_for_media(&mut self, media_id: i64) -> RusqliteResult<Option<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT mf.file_path FROM media_files mf
             WHERE mf.media_id = ?1 OR mf.media_id IN (
                 SELECT id FROM media WHERE parent_id = ?1
             )
             ORDER BY mf.id LIMIT 1",
        )?;
        let row = stmt.query_row(params![media_id], |row| row.get(0));
        match row {
            Ok(path) => Ok(Some(path)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Obtiene el siguiente episodio no visto de una serie/anime.
    /// Busca el primer archivo (ordenado por temporada, episodio) que no tenga
    /// registro en playback_history.
    pub fn get_next_episode_to_watch(
        &mut self,
        media_id: i64,
    ) -> RusqliteResult<Option<(i64, String, String)>> {
        // (id, file_path, episode_label)
        let mut stmt = self.conn.prepare(
            "SELECT mf.id, mf.file_path,
                    mf.season, mf.episode, mf.title
             FROM media_files mf
             WHERE mf.media_id = ?1 OR mf.media_id IN (
                 SELECT id FROM media WHERE parent_id = ?1
             )
             AND mf.id NOT IN (
                 SELECT DISTINCT ph.media_id FROM playback_history ph WHERE ph.media_id = mf.id
             )
             ORDER BY mf.season NULLS LAST, mf.episode NULLS LAST, mf.id ASC
             LIMIT 1",
        )?;
        let row = stmt.query_row(params![media_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                format!(
                    "S{}E{} - {}",
                    row.get::<_, Option<i32>>(2)?.unwrap_or(0),
                    row.get::<_, Option<i32>>(3)?.unwrap_or(0),
                    row.get::<_, Option<String>>(4)?.unwrap_or_default()
                ),
            ))
        });
        match row {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Califica un medio (1-5 estrellas).
    pub fn set_rating(&mut self, media_id: i64, rating: i32) -> RusqliteResult<()> {
        self.conn.execute(
            "UPDATE media SET rating = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![rating, media_id],
        )?;
        Ok(())
    }

    /// Obtiene la calificación de un medio (0 si no tiene).
    pub fn get_rating(&mut self, media_id: i64) -> RusqliteResult<i32> {
        let mut stmt = self
            .conn
            .prepare("SELECT rating FROM media WHERE id = ?1")?;
        let row = stmt.query_row(params![media_id], |row| row.get::<_, i32>(0));
        match row {
            Ok(rating) => Ok(rating),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(e) => Err(e),
        }
    }

    pub fn list(&mut self, options: ListMediaOptions) -> RusqliteResult<Vec<Media>> {
        let mut sql = String::from(
            "SELECT id, type, title, sort_title, year, folder_path, created_at, updated_at,
                    is_favorite, is_watched, poster_path, banner_path, synopsis,
                    metadata_fetched_at, parent_id, rating, vote_average,
                    metadata_source_id, categories_fetched_at
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
                        m.is_favorite, m.is_watched, m.poster_path, m.banner_path, m.synopsis,
                        m.metadata_fetched_at, m.parent_id, m.rating, m.vote_average,
                        m.metadata_source_id, m.categories_fetched_at
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
                        m.is_favorite, m.is_watched, m.poster_path, m.banner_path, m.synopsis,
                        m.metadata_fetched_at, m.parent_id, m.rating, m.vote_average,
                        m.metadata_source_id, m.categories_fetched_at
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
                    is_favorite, is_watched, poster_path, banner_path, synopsis,
                    metadata_fetched_at, parent_id, rating, vote_average,
                    metadata_source_id, categories_fetched_at
             FROM media WHERE parent_id = ?1 ORDER BY sort_title ASC",
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows: Vec<Media> = stmt
            .query_map(params![parent_id], map_media_row)?
            .filter_map(Result::ok)
            .collect();

        Ok(rows)
    }
}

// ---------------------------------------------------------------------------
// Categoría de medio (género/etiqueta) para filtrado de la biblioteca.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub slug: String,
    /// Número de medios asociados; se rellena en `list_categories`.
    pub media_count: i64,
}

impl<'a> MediaRepository<'a> {
    /// Lista todas las categorías disponibles ordenadas por nombre, con el
    /// número de medios asociados a cada una.
    pub fn list_categories(&mut self) -> RusqliteResult<Vec<Category>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.id, c.name, c.slug, COUNT(mc.media_id)
             FROM categories c
             LEFT JOIN media_categories mc ON mc.category_id = c.id
             GROUP BY c.id, c.name, c.slug
             ORDER BY c.name ASC",
        )?;
        let rows: Vec<Category> = stmt
            .query_map([], |row| {
                Ok(Category {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    slug: row.get(2)?,
                    media_count: row.get(3)?,
                })
            })?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    /// Crea (si no existe) y devuelve el identificador de una categoría,
    /// normalizando el nombre y generando un slug único.
    ///
    /// Devuelve `None` cuando el nombre está vacío después de normalizarlo:
    /// no se inventan categorías para metadatos ausentes.
    pub fn get_or_create_category(&mut self, name: &str) -> RusqliteResult<Option<i64>> {
        let Some(name) = normalize_category_name(name) else {
            return Ok(None);
        };
        let slug = slugify(&name);

        // Si el slug ya existe se reutiliza la categoría, aunque el nombre
        // difiera en mayúsculas, tildes o separadores.
        let existing = {
            let mut stmt = self
                .conn
                .prepare("SELECT id FROM categories WHERE slug = ?1")?;
            let row = stmt.query_row(params![slug], |row| row.get::<_, i64>(0));
            match row {
                Ok(id) => Some(id),
                Err(rusqlite::Error::QueryReturnedNoRows) => None,
                Err(error) => return Err(error),
            }
        };
        if let Some(id) = existing {
            return Ok(Some(id));
        }

        self.conn.execute(
            "INSERT INTO categories (name, slug) VALUES (?1, ?2)",
            params![name, slug],
        )?;
        Ok(Some(self.conn.last_insert_rowid()))
    }

    /// Asocia un medio con un conjunto de categorías, reemplazando las
    /// asociaciones previas (idempotente).
    pub fn set_media_categories(
        &mut self,
        media_id: i64,
        category_ids: &[i64],
    ) -> RusqliteResult<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM media_categories WHERE media_id = ?1",
            params![media_id],
        )?;
        for &category_id in category_ids {
            tx.execute(
                "INSERT INTO media_categories (media_id, category_id) VALUES (?1, ?2)",
                params![media_id, category_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Devuelve los nombres de las categorías asociadas a un medio.
    pub fn load_categories_for_media(&mut self, media_id: i64) -> RusqliteResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT c.name FROM categories c
             JOIN media_categories mc ON mc.category_id = c.id
             WHERE mc.media_id = ?1 ORDER BY c.name ASC",
        )?;
        let rows: Vec<String> = stmt
            .query_map(params![media_id], |row| row.get(0))?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    /// Lista los medios padre asociados a una categoría.
    pub fn list_by_category(
        &mut self,
        category_id: i64,
        options: ListMediaOptions,
    ) -> RusqliteResult<Vec<Media>> {
        let mut sql = String::from(
            "SELECT m.id, m.type, m.title, m.sort_title, m.year, m.folder_path,
                        m.created_at, m.updated_at,
                        m.is_favorite, m.is_watched, m.poster_path, m.banner_path, m.synopsis,
                        m.metadata_fetched_at, m.parent_id, m.rating, m.vote_average,
                        m.metadata_source_id, m.categories_fetched_at
             FROM media m
             JOIN media_categories mc ON mc.media_id = m.id
             WHERE mc.category_id = ?1 AND m.parent_id IS NULL",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(category_id)];

        if let Some(ref t) = options.r#type {
            sql.push_str(" AND m.type = ?");
            params.push(Box::new(t.clone()));
        }

        let order_clause = match options.order_by.as_deref().unwrap_or("title") {
            "added" => "ORDER BY m.created_at DESC",
            "recent" => "ORDER BY m.updated_at DESC",
            _ => "ORDER BY m.sort_title ASC",
        };
        sql.push(' ');
        sql.push_str(order_clause);
        sql.push_str(" LIMIT ?");
        params.push(Box::new(options.limit.unwrap_or(500)));
        sql.push_str(" OFFSET ?");
        params.push(Box::new(options.offset.unwrap_or(0)));

        let params_slice: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let mut stmt = self.conn.prepare(&sql)?;
        let rows: Vec<Media> = stmt
            .query_map(params_slice.as_slice(), map_media_row)?
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

    pub fn list_files_for_show(
        &mut self,
        media_id: i64,
    ) -> RusqliteResult<Vec<crate::db::file_repo::MediaFile>> {
        let mut file_repo = FileRepository::new(self.conn);
        file_repo.list_by_show(media_id)
    }

    pub fn set_episode_image(
        &mut self,
        file_path: &str,
        episode_image_url: &str,
    ) -> RusqliteResult<usize> {
        let mut file_repo = FileRepository::new(self.conn);
        file_repo.set_episode_image(file_path, episode_image_url)
    }

    pub fn update_metadata(
        &mut self,
        media_id: i64,
        year: Option<i32>,
        poster_path: Option<&str>,
        banner_path: Option<&str>,
        synopsis: Option<&str>,
    ) -> RusqliteResult<()> {
        self.conn.execute(
            "UPDATE media SET year = COALESCE(?1, year), poster_path = ?2,
             banner_path = COALESCE(?3, banner_path), synopsis = ?4,
             metadata_fetched_at = datetime('now'), updated_at = datetime('now')
             WHERE id = ?5",
            params![year, poster_path, banner_path, synopsis, media_id],
        )?;
        Ok(())
    }

    /// Guarda la puntuación obtenida de los metadatos (vote_average, 0-10).
    pub fn set_vote_average(
        &mut self,
        media_id: i64,
        vote_average: Option<f64>,
    ) -> RusqliteResult<()> {
        self.conn.execute(
            "UPDATE media SET vote_average = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![vote_average, media_id],
        )?;
        Ok(())
    }

    /// Guarda el identificador del show en TVmaze para no repetir búsquedas
    /// por título al pedir episodios e imágenes.
    pub fn set_metadata_source_id(
        &mut self,
        media_id: i64,
        source_id: Option<i64>,
    ) -> RusqliteResult<()> {
        self.conn.execute(
            "UPDATE media SET metadata_source_id = ?1, updated_at = datetime('now') WHERE id = ?2",
            params![source_id, media_id],
        )?;
        Ok(())
    }

    /// Marca que ya se intentó extraer categorías de los metadatos.
    pub fn mark_categories_fetched(&mut self, media_id: i64) -> RusqliteResult<()> {
        self.conn.execute(
            "UPDATE media SET categories_fetched_at = datetime('now'),
             updated_at = datetime('now') WHERE id = ?1",
            params![media_id],
        )?;
        Ok(())
    }

    /// Devuelve los medios padre a los que todavía no se les intentó extraer
    /// categorías, para rellenar bibliotecas antiguas.
    pub fn list_media_pending_categories(&mut self) -> RusqliteResult<Vec<Media>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, type, title, sort_title, year, folder_path, created_at, updated_at,
                    is_favorite, is_watched, poster_path, banner_path, synopsis,
                    metadata_fetched_at, parent_id, rating, vote_average,
                    metadata_source_id, categories_fetched_at
             FROM media
             WHERE parent_id IS NULL AND categories_fetched_at IS NULL
             ORDER BY sort_title ASC",
        )?;
        let rows: Vec<Media> = stmt
            .query_map([], map_media_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    /// Obtiene la puntuación de metadatos de un medio (vote_average, 0-10).
    pub fn get_vote_average(&mut self, media_id: i64) -> RusqliteResult<Option<f64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT vote_average FROM media WHERE id = ?1")?;
        let row = stmt.query_row(params![media_id], |row| Ok(row.get::<_, f64>(0).ok()));
        match row {
            Ok(vote) => Ok(vote),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Sobrescribe todos los metadatos de un medio con los datos obtenidos de TVmaze.
    ///
    /// A diferencia de `update_metadata`, no conserva los valores anteriores: el
    /// título, el título de ordenación, el año, el póster, el banner y la
    /// sinopsis se reemplazan por los nuevos valores.
    pub fn overwrite_metadata(
        &mut self,
        media_id: i64,
        metadata: &MediaMetadataInput<'_>,
    ) -> RusqliteResult<()> {
        self.conn.execute(
            "UPDATE media SET title = ?1, sort_title = ?2, year = ?3, poster_path = ?4,
             banner_path = ?5, synopsis = ?6, metadata_fetched_at = datetime('now'),
             updated_at = datetime('now')
             WHERE id = ?7",
            params![
                metadata.title,
                metadata.sort_title,
                metadata.year,
                metadata.poster_path,
                metadata.banner_path,
                metadata.synopsis,
                media_id
            ],
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
        banner_path: row.get(11)?,
        synopsis: row.get(12)?,
        metadata_fetched_at: row.get(13)?,
        parent_id: row.get(14)?,
        rating: row.get::<_, i32>(15).unwrap_or(0),
        vote_average: row.get::<_, f64>(16).ok(),
        metadata_source_id: row.get(17)?,
        categories_fetched_at: row.get(18)?,
        categories: Vec::new(),
    })
}

/// Normaliza un nombre de categoría: recorta extremos y colapsa espacios.
/// Devuelve `None` si no queda ningún carácter visible.
pub(crate) fn normalize_category_name(name: &str) -> Option<String> {
    let normalized = name.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

/// Convierte un nombre de categoría en un slug ASCII seguro para consultas.
///
/// Pliega las tildes y caracteres latinos habituales (`Acción` → `accion`,
/// `Animación` → `animacion`, `Ñ` → `n`) para que dos categorías que solo
/// difieren en acentos compartan slug y no se dupliquen.
fn slugify(name: &str) -> String {
    let mut slug = String::new();
    for character in name.to_lowercase().chars() {
        let folded = fold_latin(character);
        if folded.is_ascii_alphanumeric() {
            slug.push(folded);
        } else if (folded.is_whitespace() || folded == '-' || folded == '_')
            && !slug.ends_with('-')
            && !slug.is_empty()
        {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "categoria".to_string()
    } else {
        slug
    }
}

/// Pliega los caracteres latinos acentuados más habituales a ASCII.
fn fold_latin(character: char) -> char {
    match character {
        'á' | 'à' | 'ä' | 'â' | 'ã' | 'å' | 'ā' | 'ă' | 'ą' => 'a',
        'é' | 'è' | 'ë' | 'ê' | 'ē' | 'ĕ' | 'ė' | 'ę' => 'e',
        'í' | 'ì' | 'ï' | 'î' | 'ī' | 'į' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' | 'õ' | 'ø' | 'ō' | 'ő' => 'o',
        'ú' | 'ù' | 'ü' | 'û' | 'ū' | 'ů' | 'ű' | 'ų' => 'u',
        'ñ' | 'ń' | 'ň' => 'n',
        'ç' | 'ć' | 'č' => 'c',
        'ý' | 'ÿ' => 'y',
        'š' => 's',
        'ž' => 'z',
        'ł' => 'l',
        'đ' => 'd',
        'ß' => 's',
        _ => character,
    }
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
                updated_at  TEXT    NOT NULL DEFAULT (datetime('now')),
                vote_average REAL
            );
             CREATE TABLE IF NOT EXISTS media_files (
                id          INTEGER PRIMARY KEY,
                media_id    INTEGER NOT NULL REFERENCES media(id) ON DELETE CASCADE,
                file_path   TEXT    NOT NULL UNIQUE,
                 episode_image_path TEXT
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
    fn overwrite_metadata_reemplaza_todos_los_campos() {
        let (db_path, _dir) = temp_db();
        let mut conn = open_db(&db_path).expect("abrir BD");
        let media_id = setup_media_and_file(&mut conn);
        let mut repo = MediaRepository::new(&mut conn);

        repo.update_metadata(
            media_id,
            Some(1999),
            Some("https://example.com/old.jpg"),
            Some("https://example.com/old-banner.jpg"),
            Some("Sinopsis antigua"),
        )
        .expect("guardar metadatos antiguos");

        repo.overwrite_metadata(
            media_id,
            &MediaMetadataInput {
                title: "Nuevo título",
                sort_title: "nuevo título",
                year: Some(2024),
                poster_path: Some("https://example.com/new.jpg"),
                banner_path: Some("https://example.com/new-banner.jpg"),
                synopsis: Some("Sinopsis nueva"),
            },
        )
        .expect("sobrescribir metadatos");

        let media = repo
            .find_by_id(media_id)
            .expect("buscar medio")
            .expect("el medio existe");
        assert_eq!(media.title, "Nuevo título");
        assert_eq!(media.sort_title, "nuevo título");
        assert_eq!(media.year, Some(2024));
        assert_eq!(
            media.poster_path.as_deref(),
            Some("https://example.com/new.jpg")
        );
        assert_eq!(
            media.banner_path.as_deref(),
            Some("https://example.com/new-banner.jpg")
        );
        assert_eq!(media.synopsis.as_deref(), Some("Sinopsis nueva"));
        assert!(media.metadata_fetched_at.is_some());
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
