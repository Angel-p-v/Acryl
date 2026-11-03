//! Repositorio de archivos multimedia.

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
    pub audio_stream_index: Option<i32>,
    pub subtitle_stream_index: Option<i32>,
    pub subtitles_disabled: bool,
    pub streams_indexed_at: Option<String>,
    /// Ruta de la imagen específica de este episodio (thumbnail), si existe.
    pub episode_image_path: Option<String>,
    /// Momento en que se intentó obtener el thumbnail del episodio. Evita
    /// repetir peticiones cuando la fuente no ofrece imagen.
    pub episode_image_checked_at: Option<String>,
}

impl MediaFile {
    /// Indica si el archivo usa la selección automática de pistas de mpv.
    pub fn is_automatic_tracks(&self) -> bool {
        self.audio_stream_index.is_none()
            && self.subtitle_stream_index.is_none()
            && !self.subtitles_disabled
    }
}

/// Preferencia de pistas de audio y subtítulos de un archivo.
///
/// Los índices son relativos al tipo y empiezan en 0, igual que los devuelve
/// `ffprobe`. La conversión a los identificadores de mpv (`--aid` / `--sid`,
/// que empiezan en 1) se hace al construir los argumentos del reproductor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TrackSelection {
    /// Pista de audio elegida; `None` deja que mpv use la predeterminada.
    pub audio_stream_index: Option<i32>,
    /// Pista de subtítulos elegida; `None` con `subtitles_disabled = false`
    /// deja que mpv use la predeterminada.
    pub subtitle_stream_index: Option<i32>,
    /// `true` desactiva los subtítulos (`--sid=no`).
    pub subtitles_disabled: bool,
}

impl TrackSelection {
    /// Indica si la selección representa el comportamiento automático de mpv.
    pub fn is_automatic(self) -> bool {
        self.audio_stream_index.is_none()
            && self.subtitle_stream_index.is_none()
            && !self.subtitles_disabled
    }
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
                   indexed_at = datetime('now'),
                   episode_image_path = CASE
                     WHEN media_files.media_id = excluded.media_id
                      AND media_files.season IS excluded.season
                      AND media_files.episode IS excluded.episode
                     THEN media_files.episode_image_path
                     ELSE NULL
                   END,
                   episode_image_checked_at = CASE
                     WHEN media_files.media_id = excluded.media_id
                      AND media_files.season IS excluded.season
                      AND media_files.episode IS excluded.episode
                     THEN media_files.episode_image_checked_at
                     ELSE NULL
                   END",
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
            "SELECT id, media_id, season, episode, title, file_path, size_bytes, mtime_ms,
                    indexed_at, audio_stream_index, subtitle_stream_index, subtitles_disabled,
                    streams_indexed_at, episode_image_path, episode_image_checked_at
             FROM media_files WHERE media_id = ?1 ORDER BY season, episode, file_path",
        )?;
        let rows: Vec<MediaFile> = stmt
            .query_map(params![media_id], map_file_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    /// Devuelve el medio al que pertenece un archivo, si está indexado.
    pub fn media_id_for_file(&mut self, file_path: &str) -> RusqliteResult<Option<i64>> {
        let mut stmt = self
            .conn
            .prepare("SELECT media_id FROM media_files WHERE file_path = ?1 LIMIT 1")?;
        match stmt.query_row(params![file_path], |row| row.get(0)) {
            Ok(media_id) => Ok(Some(media_id)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(error),
        }
    }

    /// Devuelve la preferencia de pistas de un archivo. Si el archivo no
    /// existe en la biblioteca, devuelve la selección automática.
    pub fn track_selection(&mut self, file_path: &str) -> RusqliteResult<TrackSelection> {
        let mut stmt = self.conn.prepare(
            "SELECT audio_stream_index, subtitle_stream_index, subtitles_disabled
             FROM media_files WHERE file_path = ?1 LIMIT 1",
        )?;
        let row = stmt.query_row(params![file_path], |row| {
            Ok(TrackSelection {
                audio_stream_index: row.get(0)?,
                subtitle_stream_index: row.get(1)?,
                subtitles_disabled: row.get::<_, i64>(2)? != 0,
            })
        });
        match row {
            Ok(selection) => Ok(selection),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(TrackSelection::default()),
            Err(error) => Err(error),
        }
    }

    /// Guarda la preferencia de pistas de un archivo.
    pub fn set_track_selection(
        &mut self,
        file_path: &str,
        selection: TrackSelection,
    ) -> RusqliteResult<usize> {
        self.conn.execute(
            "UPDATE media_files
             SET audio_stream_index = ?2, subtitle_stream_index = ?3, subtitles_disabled = ?4
             WHERE file_path = ?1",
            params![
                file_path,
                selection.audio_stream_index,
                selection.subtitle_stream_index,
                i64::from(selection.subtitles_disabled),
            ],
        )
    }

    /// Marca un archivo como analizado por `ffprobe` para no repetir la
    /// extracción en cada indexación.
    pub fn mark_streams_indexed(&mut self, file_path: &str) -> RusqliteResult<usize> {
        self.conn.execute(
            "UPDATE media_files SET streams_indexed_at = datetime('now') WHERE file_path = ?1",
            params![file_path],
        )
    }

    /// Devuelve las rutas de los archivos cuyo árbol de pistas aún no se ha
    /// extraído. Se usa al indexar para completar bibliotecas existentes.
    pub fn list_files_without_streams_index(&mut self) -> RusqliteResult<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT file_path FROM media_files
             WHERE streams_indexed_at IS NULL
             ORDER BY file_path",
        )?;
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    /// Lista todos los archivos de un show, incluidos los de sus temporadas
    /// hijas. Se usa para asociar thumbnails de episodios sin mezclar medios.
    pub fn list_by_show(&mut self, root_media_id: i64) -> RusqliteResult<Vec<MediaFile>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, media_id, season, episode, title, file_path, size_bytes, mtime_ms,
                    indexed_at, audio_stream_index, subtitle_stream_index, subtitles_disabled,
                    streams_indexed_at, episode_image_path, episode_image_checked_at
             FROM media_files
             WHERE media_id = ?1 OR media_id IN (
                 SELECT id FROM media WHERE parent_id = ?1
             )
             ORDER BY season, episode, file_path",
        )?;
        let rows: Vec<MediaFile> = stmt
            .query_map(params![root_media_id], map_file_row)?
            .filter_map(Result::ok)
            .collect();
        Ok(rows)
    }

    /// Guarda la URL de la imagen de un episodio y marca la comprobación como
    /// realizada. La UI resuelve la URL a la caché local con `cached_path`.
    pub fn set_episode_image(
        &mut self,
        file_path: &str,
        episode_image_url: &str,
    ) -> RusqliteResult<usize> {
        self.conn.execute(
            "UPDATE media_files
             SET episode_image_path = ?2, episode_image_checked_at = datetime('now')
             WHERE file_path = ?1",
            params![file_path, episode_image_url],
        )
    }

    /// Marca que ya se intentó obtener el thumbnail de un episodio aunque la
    /// fuente no ofreciera ninguna imagen.
    pub fn mark_episode_image_checked(&mut self, file_path: &str) -> RusqliteResult<usize> {
        self.conn.execute(
            "UPDATE media_files SET episode_image_checked_at = datetime('now')
             WHERE file_path = ?1",
            params![file_path],
        )
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
        audio_stream_index: row.get(9)?,
        subtitle_stream_index: row.get(10)?,
        subtitles_disabled: row.get::<_, i64>(11)? != 0,
        streams_indexed_at: row.get(12)?,
        episode_image_path: row.get(13)?,
        episode_image_checked_at: row.get(14)?,
    })
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

    fn insert_media_and_file(conn: &mut Connection) {
        conn.execute(
            "INSERT INTO media (type, title, sort_title, folder_path)
             VALUES ('movie', 'Pelicula', 'pelicula', '/tmp/pelicula')",
            [],
        )
        .expect("insertar medio");
        conn.execute(
            "INSERT INTO media_files (media_id, file_path)
             VALUES (1, '/tmp/pelicula/video.mkv')",
            [],
        )
        .expect("insertar archivo");
    }

    #[test]
    fn track_selection_por_defecto_es_automatica() {
        let (db_path, _dir) = temp_db();
        let mut conn = open_db(&db_path).expect("abrir BD");
        insert_media_and_file(&mut conn);

        let mut repo = FileRepository::new(&mut conn);
        let selection = repo
            .track_selection("/tmp/pelicula/video.mkv")
            .expect("leer selección");

        assert!(selection.is_automatic());
        assert_eq!(selection.audio_stream_index, None);
        assert_eq!(selection.subtitle_stream_index, None);
        assert!(!selection.subtitles_disabled);
    }

    #[test]
    fn set_track_selection_persiste_audio_y_subtitulos() {
        let (db_path, _dir) = temp_db();
        let mut conn = open_db(&db_path).expect("abrir BD");
        insert_media_and_file(&mut conn);

        let selection = TrackSelection {
            audio_stream_index: Some(1),
            subtitle_stream_index: Some(0),
            subtitles_disabled: false,
        };
        {
            let mut repo = FileRepository::new(&mut conn);
            let updated = repo
                .set_track_selection("/tmp/pelicula/video.mkv", selection)
                .expect("guardar selección");
            assert_eq!(updated, 1);
        }

        let mut repo = FileRepository::new(&mut conn);
        let stored = repo
            .track_selection("/tmp/pelicula/video.mkv")
            .expect("leer selección");
        assert_eq!(stored, selection);
    }

    #[test]
    fn subtitulos_desactivados_se_distinguen_de_automatico() {
        let (db_path, _dir) = temp_db();
        let mut conn = open_db(&db_path).expect("abrir BD");
        insert_media_and_file(&mut conn);

        let selection = TrackSelection {
            audio_stream_index: Some(0),
            subtitle_stream_index: None,
            subtitles_disabled: true,
        };
        {
            let mut repo = FileRepository::new(&mut conn);
            repo.set_track_selection("/tmp/pelicula/video.mkv", selection)
                .expect("guardar selección");
        }

        let mut repo = FileRepository::new(&mut conn);
        let stored = repo
            .track_selection("/tmp/pelicula/video.mkv")
            .expect("leer selección");
        assert_eq!(stored, selection);
        assert!(!stored.is_automatic());
    }

    #[test]
    fn is_automatic_tracks_solo_cuando_no_hay_preferencia() {
        let mut file = MediaFile {
            id: 1,
            media_id: 1,
            season: Some(1),
            episode: Some(1),
            title: None,
            file_path: "/tmp/video.mkv".to_string(),
            size_bytes: None,
            mtime_ms: None,
            indexed_at: String::new(),
            audio_stream_index: None,
            subtitle_stream_index: None,
            subtitles_disabled: false,
            streams_indexed_at: None,
            episode_image_path: None,
            episode_image_checked_at: None,
        };
        assert!(file.is_automatic_tracks());

        file.subtitles_disabled = true;
        assert!(
            !file.is_automatic_tracks(),
            "desactivar subtítulos es una preferencia"
        );

        file.subtitles_disabled = false;
        file.audio_stream_index = Some(0);
        assert!(!file.is_automatic_tracks());
    }

    #[test]
    fn archivo_desconocido_no_falla_al_leer_preferencia() {
        let (db_path, _dir) = temp_db();
        let mut conn = open_db(&db_path).expect("abrir BD");

        let mut repo = FileRepository::new(&mut conn);
        let selection = repo
            .track_selection("/tmp/desconocido.mkv")
            .expect("leer selección");
        assert!(selection.is_automatic());
        assert_eq!(
            repo.set_track_selection("/tmp/desconocido.mkv", TrackSelection::default())
                .expect("actualizar selección"),
            0
        );
    }

    fn file(media_id: i64, season: i32, episode: i32, path: &str) -> MediaFileInput {
        MediaFileInput {
            media_id,
            season: Some(season),
            episode: Some(episode),
            title: Some(format!("Episodio {episode}")),
            file_path: path.to_string(),
            size_bytes: Some(1024),
            mtime_ms: Some(1_700_000_000_000),
        }
    }

    #[test]
    fn upsert_many_inserta_lista_y_actualiza_sin_duplicar() {
        let (db_path, _dir) = temp_db();
        let mut conn = open_db(&db_path).expect("abrir BD");
        insert_media_and_file(&mut conn);

        let files = vec![
            file(1, 1, 1, "/tmp/pelicula/s01e01.mkv"),
            file(1, 1, 2, "/tmp/pelicula/s01e02.mkv"),
        ];
        {
            let mut repo = FileRepository::new(&mut conn);
            assert_eq!(repo.upsert_many(&files).expect("insertar archivos"), 2);
            assert_eq!(repo.upsert_many(&[]).expect("lista vacía"), 0);
        }

        {
            let mut repo = FileRepository::new(&mut conn);
            let listed = repo.list_by_media(1).expect("listar archivos");
            assert_eq!(
                listed.len(),
                3,
                "se conserva el archivo previo y los nuevos"
            );
            assert!(
                listed
                    .iter()
                    .any(|f| f.file_path == "/tmp/pelicula/s01e01.mkv")
            );
            assert_eq!(
                repo.media_id_for_file("/tmp/pelicula/s01e02.mkv")
                    .expect("buscar medio del archivo"),
                Some(1)
            );
            assert_eq!(
                repo.media_id_for_file("/tmp/desconocido.mkv")
                    .expect("buscar archivo inexistente"),
                None
            );
        }

        let replaced = vec![file(1, 1, 1, "/tmp/pelicula/s01e01.mkv")];
        {
            let mut repo = FileRepository::new(&mut conn);
            assert_eq!(repo.upsert_many(&replaced).expect("actualizar archivo"), 1);
        }

        let mut repo = FileRepository::new(&mut conn);
        let listed = repo.list_by_media(1).expect("listar archivos");
        assert_eq!(listed.len(), 3, "el upsert no duplica el archivo");
    }

    #[test]
    fn list_by_show_incluye_las_temporadas_hijas() {
        let (db_path, _dir) = temp_db();
        let mut conn = open_db(&db_path).expect("abrir BD");
        conn.execute(
            "INSERT INTO media (type, title, sort_title, folder_path)
             VALUES ('series', 'Serie', 'serie', '/tmp/serie')",
            [],
        )
        .expect("insertar serie");
        conn.execute(
            "INSERT INTO media (type, title, sort_title, folder_path, parent_id)
             VALUES ('series', 'Serie T1', 'serie t1', '/tmp/serie/t1', 1)",
            [],
        )
        .expect("insertar temporada");
        conn.execute(
            "INSERT INTO media (type, title, sort_title, folder_path)
             VALUES ('series', 'Otra serie', 'otra serie', '/tmp/otra')",
            [],
        )
        .expect("insertar otra serie");

        let files = vec![
            file(1, 1, 1, "/tmp/serie/s01e01.mkv"),
            file(2, 1, 2, "/tmp/serie/t1/s01e02.mkv"),
            file(3, 1, 1, "/tmp/otra/s01e01.mkv"),
        ];
        let mut repo = FileRepository::new(&mut conn);
        repo.upsert_many(&files).expect("insertar archivos");

        let listed = repo.list_by_show(1).expect("listar archivos del show");
        let paths: Vec<&str> = listed.iter().map(|f| f.file_path.as_str()).collect();
        assert_eq!(
            paths,
            vec!["/tmp/serie/s01e01.mkv", "/tmp/serie/t1/s01e02.mkv"],
            "incluye la temporada hija y excluye otros shows"
        );
    }

    #[test]
    fn la_imagen_del_episodio_se_conserva_solo_si_no_cambia_la_posicion() {
        let (db_path, _dir) = temp_db();
        let mut conn = open_db(&db_path).expect("abrir BD");
        insert_media_and_file(&mut conn);

        let path = "/tmp/pelicula/s01e01.mkv";
        {
            let mut repo = FileRepository::new(&mut conn);
            repo.upsert_many(&[file(1, 1, 1, path)])
                .expect("insertar episodio");
            assert_eq!(
                repo.set_episode_image(path, "https://example.com/thumb.jpg")
                    .expect("guardar imagen"),
                1
            );
        }

        {
            let mut repo = FileRepository::new(&mut conn);
            let stored = repo
                .list_by_media(1)
                .expect("listar archivos")
                .into_iter()
                .find(|f| f.file_path == path)
                .expect("episodio indexado");
            assert_eq!(
                stored.episode_image_path.as_deref(),
                Some("https://example.com/thumb.jpg")
            );
            assert!(stored.episode_image_checked_at.is_some());
        }

        {
            let mut repo = FileRepository::new(&mut conn);
            repo.upsert_many(&[file(1, 1, 1, path)])
                .expect("reindexar el mismo episodio");
        }
        {
            let mut repo = FileRepository::new(&mut conn);
            let stored = repo
                .list_by_media(1)
                .expect("listar archivos")
                .into_iter()
                .find(|f| f.file_path == path)
                .expect("episodio indexado");
            assert_eq!(
                stored.episode_image_path.as_deref(),
                Some("https://example.com/thumb.jpg"),
                "reindexar la misma posición conserva la imagen"
            );
        }

        {
            let mut repo = FileRepository::new(&mut conn);
            repo.upsert_many(&[file(1, 1, 2, path)])
                .expect("mover el archivo de episodio");
        }
        let mut repo = FileRepository::new(&mut conn);
        let stored = repo
            .list_by_media(1)
            .expect("listar archivos")
            .into_iter()
            .find(|f| f.file_path == path)
            .expect("episodio indexado");
        assert_eq!(stored.episode_image_path, None);
        assert_eq!(stored.episode_image_checked_at, None);
    }

    #[test]
    fn mark_episode_image_checked_registra_la_comprobacion_sin_imagen() {
        let (db_path, _dir) = temp_db();
        let mut conn = open_db(&db_path).expect("abrir BD");
        insert_media_and_file(&mut conn);

        let mut repo = FileRepository::new(&mut conn);
        assert_eq!(
            repo.mark_episode_image_checked("/tmp/pelicula/video.mkv")
                .expect("marcar comprobación"),
            1
        );
        let stored = repo
            .list_by_media(1)
            .expect("listar archivos")
            .into_iter()
            .find(|f| f.file_path == "/tmp/pelicula/video.mkv")
            .expect("archivo indexado");
        assert_eq!(stored.episode_image_path, None);
        assert!(stored.episode_image_checked_at.is_some());
    }

    #[test]
    fn mark_streams_indexed_excluye_el_archivo_de_pendientes() {
        let (db_path, _dir) = temp_db();
        let mut conn = open_db(&db_path).expect("abrir BD");
        insert_media_and_file(&mut conn);

        {
            let mut repo = FileRepository::new(&mut conn);
            assert_eq!(
                repo.list_files_without_streams_index()
                    .expect("listar pendientes"),
                vec!["/tmp/pelicula/video.mkv".to_string()]
            );
            assert_eq!(
                repo.mark_streams_indexed("/tmp/pelicula/video.mkv")
                    .expect("marcar analizado"),
                1
            );
            assert!(
                repo.list_files_without_streams_index()
                    .expect("listar pendientes")
                    .is_empty()
            );
        }
    }
}
