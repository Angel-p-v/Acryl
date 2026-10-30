//! Capa de dominio: servicios de negocio.

use crate::db::media_repo::{MediaInput, NewMediaStream};
use crate::db::{MediaRepository, close_db, open_db};
use serde_json::Value;
use std::process::Command;

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

/// Extrae las pistas de audio y subtítulos de un archivo de vídeo mediante ffprobe.
///
/// Devuelve los streams con `stream_index` relativo al tipo (audio o subtitle),
/// listo para usarse con `--aid` / `--sid` de mpv.
pub fn extract_media_streams(
    file_path: &str,
) -> Result<Vec<NewMediaStream>, Box<dyn std::error::Error>> {
    let output = Command::new("ffprobe")
        .arg("-v")
        .arg("quiet")
        .arg("-print_format")
        .arg("json")
        .arg("-show_streams")
        .arg(file_path)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "ffprobe falló para {} (código {}): stderr={} stdout={}",
            file_path,
            output.status.code().unwrap_or(-1),
            stderr,
            stdout
        )
        .into());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let data: Value = serde_json::from_str(&stdout)?;

    let streams: Vec<Value> = data
        .get("streams")
        .and_then(|s| s.as_array())
        .cloned()
        .unwrap_or_default();

    let mut audio_index = 0i32;
    let mut subtitle_index = 0i32;
    let mut result = Vec::new();

    for stream in &streams {
        let codec_type = stream
            .get("codec_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let tags = stream.get("tags").and_then(|v| v.as_object());
        let language = tags
            .and_then(|t| t.get("language").and_then(|v| v.as_str()))
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string());
        let title = tags
            .and_then(|t| t.get("title").and_then(|v| v.as_str()))
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string());
        let codec = stream
            .get("codec_name")
            .and_then(|v| v.as_str())
            .map(|c| c.to_string());

        match codec_type {
            "audio" => {
                result.push(NewMediaStream {
                    file_path: file_path.to_string(),
                    stream_type: "audio".to_string(),
                    stream_index: audio_index,
                    language,
                    codec,
                    title,
                });
                audio_index += 1;
            }
            "subtitle" => {
                result.push(NewMediaStream {
                    file_path: file_path.to_string(),
                    stream_type: "subtitle".to_string(),
                    stream_index: subtitle_index,
                    language,
                    codec,
                    title,
                });
                subtitle_index += 1;
            }
            _ => {}
        }
    }

    Ok(result)
}

/// Extrae y guarda los streams de audio/subtítulo de un archivo en la base de datos.
/// Es idempotente: reemplaza los streams previos de ese archivo.
pub fn index_media_streams_for_file(
    db_path: &str,
    media_id: i64,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let streams = extract_media_streams(file_path)?;

    let mut conn = open_db(db_path)?;
    let mut media_repo = MediaRepository::new(&mut conn);
    media_repo.replace_streams_for_file(media_id, file_path, &streams)?;
    close_db(conn);

    Ok(())
}

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "webm", "mov", "flv", "wmv"];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::connection::open_db;
    use tempfile::TempDir;

    fn disponible(bin: &str) -> bool {
        std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {}", bin))
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn temp_db() -> (String, TempDir) {
        let dir = TempDir::new().expect("crear directorio temporal");
        let path = dir.path().join("test.db");
        (path.to_str().unwrap().to_string(), dir)
    }

    /// Genera un video de prueba con una pista de audio usando ffmpeg.
    fn generar_video_prueba(dir: &std::path::Path) -> std::path::PathBuf {
        let salida = dir.join("prueba.mkv");
        let estado = std::process::Command::new("ffmpeg")
            .arg("-y")
            .arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg("testsrc=duration=2:size=320x240:rate=30")
            .arg("-f")
            .arg("lavfi")
            .arg("-i")
            .arg("sine=frequency=440:duration=2")
            .arg("-c:v")
            .arg("libx264")
            .arg("-c:a")
            .arg("aac")
            .arg("-shortest")
            .arg("-loglevel")
            .arg("error")
            .arg(&salida)
            .status()
            .expect("ejecutar ffmpeg para generar video de prueba");
        assert!(estado.success(), "ffmpeg no pudo generar el video");
        salida
    }

    #[test]
    fn extract_media_streams_detecta_audio_de_video_real() {
        if !disponible("ffmpeg") || !disponible("ffprobe") {
            eprintln!("test salteado: se requiere ffmpeg y ffprobe");
            return;
        }

        let dir = TempDir::new().expect("crear directorio temporal");
        let video = generar_video_prueba(dir.path());

        let streams = extract_media_streams(video.to_str().unwrap()).expect("extraer streams");

        assert!(
            streams.iter().any(|s| s.stream_type == "audio"),
            "debería detectar al menos una pista de audio"
        );
        let audios: Vec<_> = streams
            .iter()
            .filter(|s| s.stream_type == "audio")
            .collect();
        assert_eq!(audios[0].stream_index, 0);
    }

    #[test]
    fn index_media_streams_for_file_guarda_en_bd() {
        if !disponible("ffmpeg") || !disponible("ffprobe") {
            eprintln!("test salteado: se requiere ffmpeg y ffprobe");
            return;
        }

        let (db_path, _dir) = temp_db();
        let conn = open_db(&db_path).expect("abrir BD");

        // Crear medio y archivo en BD
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
        drop(conn);

        let dir = TempDir::new().expect("crear directorio temporal");
        let video = generar_video_prueba(dir.path());
        let video_str = video.to_str().unwrap().to_string();

        // Actualizar file_path a la ruta real del video generado
        let conn = open_db(&db_path).expect("abrir BD");
        conn.execute(
            "UPDATE media_files SET file_path = ?1 WHERE media_id = ?2",
            rusqlite::params![&video_str, media_id],
        )
        .unwrap();
        drop(conn);

        index_media_streams_for_file(&db_path, media_id, &video_str).expect("indexar streams");

        // Verificar que se guardaron
        let mut conn = open_db(&db_path).expect("abrir BD");
        let mut repo = MediaRepository::new(&mut conn);
        let streams = repo
            .list_streams_by_file(&video_str)
            .expect("listar streams");
        assert!(!streams.is_empty(), "debería haber streams guardados");
        assert!(
            streams.iter().any(|s| s.stream_type == "audio"),
            "debería guardar al menos una pista de audio"
        );
    }
}
