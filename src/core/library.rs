//! Capa de dominio: servicios de negocio.
//!
//! El indexador recorre la carpeta raíz configurada y aplica estas reglas:
//! - Los vídeos sueltos de la raíz se indexan como películas.
//! - Las subcarpetas se indexan como series (o anime si la ruta contiene un
//!   componente `Anime`/`Animes`).
//! - Una carpeta con un único vídeo cuyo nombre no parece un episodio se
//!   indexa como película.
//! - Las carpetas que solo contienen otras carpetas (por ejemplo `Peliculas/`
//!   o `Series/`) actúan como contenedores y se recorren sin crear un medio.
//! - Las subcarpetas con nombre de temporada (`Season 1`, `Temporada 2`,
//!   `S01`, ...) se indexan como temporadas de la serie contenedora.

use crate::db::file_repo::{FileRepository, MediaFileInput};
use crate::db::media_repo::{MediaInput, NewMediaStream};
use crate::db::{MediaRepository, close_db, open_db};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Extensiones de vídeo que se indexan.
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "webm", "mov", "flv", "wmv"];

/// Profundidad máxima al recorrer carpetas contenedoras. Evita recorrer
/// árboles sin fin o enlazados y cubre `raíz/categoría/título/temporada`.
const MAX_SCAN_DEPTH: usize = 4;

/// Nombres de archivo genéricos que no sirven como título de una película.
const GENERIC_MOVIE_STEMS: &[&str] = &["movie", "film", "video", "index", "main", "feature"];

pub struct LibraryService;

impl LibraryService {
    /// Indexa una carpeta raíz completa.
    pub fn index_folder(
        db_path: &str,
        folder_path: &str,
    ) -> Result<IndexResult, Box<dyn std::error::Error>> {
        let root = Path::new(folder_path);
        if !root.is_dir() {
            return Err(format!(
                "la carpeta no existe o no es un directorio: {}",
                folder_path
            )
            .into());
        }

        let mut conn = open_db(db_path)?;
        let mut counters = IndexCounters::default();

        Self::scan_root(&mut conn, root, &mut counters)?;

        close_db(conn);
        Ok(counters.into_result())
    }

    /// Recorre la raíz: los vídeos sueltos son películas y cada subcarpeta se
    /// clasifica como serie, película o contenedor.
    fn scan_root(
        conn: &mut rusqlite::Connection,
        root: &Path,
        counters: &mut IndexCounters,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let entries = read_entries(root)?;

        for path in entries
            .iter()
            .filter(|path| path.is_file() && is_video(path))
        {
            Self::index_movie(conn, path, counters)?;
        }

        for path in entries.iter().filter(|path| path.is_dir()) {
            Self::scan_directory(conn, path, 1, counters)?;
        }

        Ok(())
    }

    /// Analiza una carpeta que puede ser una serie, una película o un
    /// contenedor de otros medios.
    fn scan_directory(
        conn: &mut rusqlite::Connection,
        dir: &Path,
        depth: usize,
        counters: &mut IndexCounters,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if depth > MAX_SCAN_DEPTH {
            eprintln!(
                "Se omite '{}': se superó la profundidad máxima de {} niveles",
                dir.display(),
                MAX_SCAN_DEPTH
            );
            return Ok(());
        }

        let entries = match read_entries(dir) {
            Ok(entries) => entries,
            Err(error) => {
                eprintln!("No se pudo leer '{}': {}", dir.display(), error);
                return Ok(());
            }
        };

        let videos: Vec<PathBuf> = entries
            .iter()
            .filter(|path| path.is_file() && is_video(path))
            .cloned()
            .collect();
        let dirs: Vec<PathBuf> = entries
            .iter()
            .filter(|path| path.is_dir())
            .cloned()
            .collect();

        let season_dirs: Vec<PathBuf> = dirs
            .iter()
            .filter(|path| is_season_dir(path) && has_direct_videos(path))
            .cloned()
            .collect();
        let other_dirs: Vec<PathBuf> = dirs
            .iter()
            .filter(|path| !season_dirs.contains(path))
            .cloned()
            .collect();

        // Sin vídeos directos: o es una serie con temporadas o es un contenedor.
        if videos.is_empty() {
            if !season_dirs.is_empty() {
                let series_type = detect_media_type(dir);
                let series_id = Self::upsert_named_media(
                    conn,
                    dir,
                    folder_title(dir),
                    series_type,
                    None,
                    counters,
                )?;
                counters.folders_processed += 1;

                for season in &season_dirs {
                    Self::index_season(conn, season, series_id, series_type, counters)?;
                }

                // Las carpetas que no son temporadas (extras, material adicional)
                // se recorren por si contienen medios independientes.
                for other in &other_dirs {
                    Self::scan_directory(conn, other, depth + 1, counters)?;
                }
                return Ok(());
            }

            for subdir in &other_dirs {
                Self::scan_directory(conn, subdir, depth + 1, counters)?;
            }
            return Ok(());
        }

        // ¿Película(s) o serie con episodios directos?
        if is_movie_collection(&videos, &dirs) {
            for video in &videos {
                Self::index_movie(conn, video, counters)?;
            }

            // Puede haber subcarpetas con más medios dentro de la misma carpeta.
            for subdir in &other_dirs {
                Self::scan_directory(conn, subdir, depth + 1, counters)?;
            }
            return Ok(());
        }

        let series_type = detect_media_type(dir);
        let series_id =
            Self::upsert_named_media(conn, dir, folder_title(dir), series_type, None, counters)?;
        counters.folders_processed += 1;

        let mut videos = videos;
        videos.sort();
        let files: Vec<MediaFileInput> = videos
            .iter()
            .enumerate()
            .map(|(index, video)| MediaFileInput {
                media_id: series_id,
                season: None,
                episode: Some(index as i32 + 1),
                title: Some(file_name(video)),
                file_path: path_string(video),
                size_bytes: file_size(video),
                mtime_ms: None,
            })
            .collect();

        Self::save_files(conn, &files, counters)?;

        for season in &season_dirs {
            Self::index_season(conn, season, series_id, series_type, counters)?;
        }

        // Otras subcarpetas dentro de una serie se ignoran deliberadamente:
        // su contenido suele ser material extra y no temporadas.
        for other in &other_dirs {
            eprintln!(
                "Se ignora '{}': está dentro de la serie '{}' y no parece una temporada",
                other.display(),
                dir.display()
            );
        }

        Ok(())
    }

    /// Indexa una carpeta de temporada y sus episodios directos.
    fn index_season(
        conn: &mut rusqlite::Connection,
        season_dir: &Path,
        series_id: i64,
        series_type: &str,
        counters: &mut IndexCounters,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut videos: Vec<PathBuf> = read_entries(season_dir)?
            .into_iter()
            .filter(|path| path.is_file() && is_video(path))
            .collect();
        if videos.is_empty() {
            return Ok(());
        }
        videos.sort();

        let season_id = Self::upsert_named_media(
            conn,
            season_dir,
            folder_title(season_dir),
            series_type,
            Some(series_id),
            counters,
        )?;
        counters.folders_processed += 1;

        let files: Vec<MediaFileInput> = videos
            .iter()
            .enumerate()
            .map(|(index, video)| MediaFileInput {
                media_id: season_id,
                season: None,
                episode: Some(index as i32 + 1),
                title: Some(file_name(video)),
                file_path: path_string(video),
                size_bytes: file_size(video),
                mtime_ms: None,
            })
            .collect();

        Self::save_files(conn, &files, counters)
    }

    /// Indexa un archivo suelto como película.
    fn index_movie(
        conn: &mut rusqlite::Connection,
        video: &Path,
        counters: &mut IndexCounters,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let title = movie_title(video);
        let media_id = Self::upsert_named_media(conn, video, title, "movie", None, counters)?;

        Self::save_files(
            conn,
            &[MediaFileInput {
                media_id,
                season: None,
                episode: None,
                title: Some(file_name(video)),
                file_path: path_string(video),
                size_bytes: file_size(video),
                mtime_ms: None,
            }],
            counters,
        )
    }

    /// Crea o actualiza un medio identificado por su ruta.
    fn upsert_named_media(
        conn: &mut rusqlite::Connection,
        folder_path: &Path,
        title: String,
        media_type: &str,
        parent_id: Option<i64>,
        counters: &mut IndexCounters,
    ) -> Result<i64, Box<dyn std::error::Error>> {
        let sort_title = title.to_lowercase();
        let media = {
            let mut repo = MediaRepository::new(conn);
            repo.upsert(&MediaInput {
                r#type: media_type.to_string(),
                title,
                sort_title,
                year: None,
                folder_path: path_string(folder_path),
                parent_id,
            })?
        };
        counters.media_added += 1;
        Ok(media.id)
    }

    /// Guarda los archivos de vídeo de un medio.
    fn save_files(
        conn: &mut rusqlite::Connection,
        files: &[MediaFileInput],
        counters: &mut IndexCounters,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut repo = FileRepository::new(conn);
        repo.upsert_many(files)?;
        counters.files_added += files.len() as i64;
        Ok(())
    }
}

/// Elimina de la biblioteca todos los medios que están dentro de una carpeta.
///
/// No toca los archivos del disco. Devuelve cuántos medios se han eliminado.
pub fn remove_folder(
    db_path: &str,
    folder_path: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let folder = Path::new(folder_path);
    let mut conn = open_db(db_path)?;

    let rows = load_media_rows(&conn)?;
    let seed: HashSet<i64> = rows
        .iter()
        .filter(|row| Path::new(&row.folder_path).starts_with(folder))
        .map(|row| row.id)
        .collect();

    if seed.is_empty() {
        close_db(conn);
        return Ok(0);
    }

    let deleted = delete_media_subtree(&mut conn, seed)?;
    close_db(conn);
    Ok(deleted)
}

/// Elimina medios que ya no pertenecen a ninguna carpeta configurada.
///
/// Se conserva como red de seguridad para configuraciones editadas a mano o
/// carpetas eliminadas desde versiones anteriores.
pub fn cleanup_media_not_in_folders(
    db_path: &str,
    folders: &[String],
) -> Result<usize, Box<dyn std::error::Error>> {
    if folders.is_empty() {
        return Ok(0);
    }

    let configured: Vec<PathBuf> = folders.iter().map(PathBuf::from).collect();
    let mut conn = open_db(db_path)?;
    let rows = load_media_rows(&conn)?;

    let seed: HashSet<i64> = rows
        .iter()
        .filter(|row| {
            let path = Path::new(&row.folder_path);
            !configured.iter().any(|folder| path.starts_with(folder))
        })
        .map(|row| row.id)
        .collect();

    if seed.is_empty() {
        close_db(conn);
        return Ok(0);
    }

    let deleted = delete_media_subtree(&mut conn, seed)?;
    close_db(conn);
    Ok(deleted)
}

struct MediaRow {
    id: i64,
    parent_id: Option<i64>,
    folder_path: String,
}

fn load_media_rows(
    conn: &rusqlite::Connection,
) -> Result<Vec<MediaRow>, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare("SELECT id, parent_id, folder_path FROM media")?;
    let rows = stmt
        .query_map([], |row| {
            Ok(MediaRow {
                id: row.get(0)?,
                parent_id: row.get(1)?,
                folder_path: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Borra un conjunto de medios y todos sus descendientes, hijos primero para
/// respetar la clave foránea `media.parent_id`.
fn delete_media_subtree(
    conn: &mut rusqlite::Connection,
    seed: HashSet<i64>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let rows = load_media_rows(conn)?;
    let by_id: HashMap<i64, Option<i64>> = rows.iter().map(|row| (row.id, row.parent_id)).collect();

    let mut children: HashMap<i64, Vec<i64>> = HashMap::new();
    for row in &rows {
        if let Some(parent_id) = row.parent_id {
            children.entry(parent_id).or_default().push(row.id);
        }
    }

    // Incluye los descendientes de los medios seleccionados.
    let mut doomed = seed.clone();
    let mut pending: Vec<i64> = seed.into_iter().collect();
    while let Some(id) = pending.pop() {
        if let Some(kids) = children.get(&id) {
            for kid in kids {
                if doomed.insert(*kid) {
                    pending.push(*kid);
                }
            }
        }
    }

    // Cuenta los hijos que también se van a borrar para eliminar hojas primero.
    let mut pending_children: HashMap<i64, usize> = HashMap::new();
    for id in &doomed {
        if let Some(Some(parent_id)) = by_id.get(id)
            && doomed.contains(parent_id)
        {
            *pending_children.entry(*parent_id).or_insert(0) += 1;
        }
    }

    let mut leaves: Vec<i64> = doomed
        .iter()
        .copied()
        .filter(|id| pending_children.get(id).copied().unwrap_or(0) == 0)
        .collect();

    let tx = conn.transaction()?;
    let mut deleted = 0usize;
    while let Some(id) = leaves.pop() {
        tx.execute("DELETE FROM media WHERE id = ?1", [id])?;
        deleted += 1;

        if let Some(Some(parent_id)) = by_id.get(&id)
            && doomed.contains(parent_id)
        {
            let count = pending_children.entry(*parent_id).or_insert(0);
            *count = count.saturating_sub(1);
            if *count == 0 {
                leaves.push(*parent_id);
            }
        }
    }
    tx.commit()?;

    Ok(deleted)
}

#[derive(Debug, Clone, Default)]
pub struct IndexResult {
    pub media_added: i64,
    pub files_added: i64,
    pub folders_processed: i64,
}

#[derive(Default)]
struct IndexCounters {
    media_added: i64,
    files_added: i64,
    folders_processed: i64,
}

impl IndexCounters {
    fn into_result(self) -> IndexResult {
        IndexResult {
            media_added: self.media_added,
            files_added: self.files_added,
            folders_processed: self.folders_processed,
        }
    }
}

/// Lee una carpeta omitiendo enlaces simbólicos y entradas ocultas.
fn read_entries(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        entries.push(entry.path());
    }
    entries.sort();
    Ok(entries)
}

fn is_video(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| VIDEO_EXTENSIONS.contains(&extension.to_lowercase().as_str()))
}

fn has_direct_videos(dir: &Path) -> bool {
    match read_entries(dir) {
        Ok(entries) => entries.iter().any(|path| path.is_file() && is_video(path)),
        Err(error) => {
            eprintln!("No se pudo leer '{}': {}", dir.display(), error);
            false
        }
    }
}

/// Decide si una carpeta con vídeos directos contiene películas o episodios.
fn is_movie_collection(videos: &[PathBuf], dirs: &[PathBuf]) -> bool {
    if dirs.iter().any(|dir| is_season_dir(dir)) {
        return false;
    }

    if videos
        .iter()
        .any(|video| looks_like_episode(&file_stem(video)))
    {
        return false;
    }

    // Varios archivos con nombres puramente numéricos (01.mkv, 02.mkv) son
    // episodios, no una colección de películas.
    if videos.len() > 1 && videos.iter().all(|video| is_short_numeric_stem(video)) {
        return false;
    }

    true
}

/// Detecta nombres de archivo habituales de episodios: `S01E02`, `1x02`,
/// `Episodio 3`, `Episode 3`, `Capítulo 3`.
fn looks_like_episode(name: &str) -> bool {
    let lower = name.to_lowercase();
    if lower.contains("episodio")
        || lower.contains("episode")
        || lower.contains("capitulo")
        || lower.contains("capítulo")
    {
        return true;
    }

    let bytes = lower.as_bytes();

    // S01E02 / S1E2
    for index in 0..bytes.len() {
        if bytes[index] != b's' {
            continue;
        }
        let mut cursor = index + 1;
        let digits_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor == digits_start || cursor >= bytes.len() || bytes[cursor] != b'e' {
            continue;
        }
        cursor += 1;
        let episode_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor > episode_start {
            return true;
        }
    }

    // 1x02
    for index in 0..bytes.len() {
        if !bytes[index].is_ascii_digit() {
            continue;
        }
        let mut cursor = index;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] != b'x' {
            continue;
        }
        cursor += 1;
        let episode_start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        if cursor > episode_start {
            return true;
        }
    }

    false
}

fn is_short_numeric_stem(path: &Path) -> bool {
    let stem = file_stem(path);
    !stem.is_empty() && stem.len() <= 3 && stem.chars().all(|character| character.is_ascii_digit())
}

/// Detecta carpetas de temporada: `Season 1`, `Temporada 2`, `S01`, `2` y
/// variantes con el título delante (`Solo Leveling S01`, `Solo Leveling - S02`,
/// `Solo Leveling Season 2`).
fn is_season_dir(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
        .trim()
        .to_lowercase();

    if name.is_empty() {
        return false;
    }

    // "1", "01", "12"
    if name.len() <= 2 && name.chars().all(|character| character.is_ascii_digit()) {
        return true;
    }

    // "Season 1", "Temporada 2", "Solo Leveling Season 2".
    ["season", "temporada"]
        .iter()
        .any(|keyword| ends_with_season_number(&name, keyword))

    // "S01", "T2", "Solo Leveling S01", "Solo Leveling - S02".
        || ['s', 't']
            .iter()
            .any(|keyword| ends_with_season_marker(&name, *keyword))
}

/// Comprueba si el nombre termina con `<keyword> <número>` (por ejemplo
/// `season 1` o `temporada 2`). La palabra clave debe estar al principio o
/// precedida por un separador para no confundirla con parte del título.
fn ends_with_season_number(name: &str, keyword: &str) -> bool {
    let Some(position) = name.rfind(keyword) else {
        return false;
    };

    if position > 0
        && name[..position]
            .chars()
            .next_back()
            .is_some_and(|previous| previous.is_alphanumeric())
    {
        return false;
    }

    let rest = &name[position + keyword.len()..];
    let rest = rest.trim_matches(is_season_separator);
    !rest.is_empty() && rest.len() <= 3 && rest.chars().all(|character| character.is_ascii_digit())
}

/// Comprueba si el nombre termina con una marca corta de temporada (`s01`,
/// `t2`), opcionalmente precedida por el título (`solo leveling s01`).
fn ends_with_season_marker(name: &str, marker: char) -> bool {
    let Some(position) = name.rfind(marker) else {
        return false;
    };

    // Al principio vale "S01"; en medio, exigimos un separador antes para no
    // confundir palabras como "friends" con una temporada.
    if position > 0
        && !name[..position]
            .chars()
            .next_back()
            .is_some_and(is_season_separator)
    {
        return false;
    }

    let rest = &name[position + marker.len_utf8()..];
    let rest = rest.trim_matches(is_season_separator);
    !rest.is_empty() && rest.len() <= 3 && rest.chars().all(|character| character.is_ascii_digit())
}

fn is_season_separator(character: char) -> bool {
    character.is_whitespace() || matches!(character, '-' | '_' | '.' | '(' | ')' | '[' | ']')
}

/// Las rutas que contienen un componente `Anime`/`Animes` se clasifican como
/// anime; el resto de series se clasifican como `series`.
fn detect_media_type(path: &Path) -> &'static str {
    let is_anime = path.components().any(|component| {
        component.as_os_str().to_str().is_some_and(|name| {
            name.eq_ignore_ascii_case("anime") || name.eq_ignore_ascii_case("animes")
        })
    });

    if is_anime { "anime" } else { "series" }
}

fn folder_title(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name(path))
}

fn movie_title(video: &Path) -> String {
    let stem = file_stem(video);
    if GENERIC_MOVIE_STEMS.contains(&stem.to_lowercase().as_str())
        && let Some(parent) = video.parent()
        && let Some(parent_name) = parent.file_name()
    {
        let parent_name = parent_name.to_string_lossy().to_string();
        if !parent_name.is_empty() {
            return parent_name;
        }
    }
    stem
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn file_size(path: &Path) -> Option<i64> {
    std::fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.len().try_into().ok())
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

    /// Crea un archivo vacío (el indexador solo comprueba la extensión).
    fn touch(path: &Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("crear directorio");
        }
        std::fs::write(path, b"").expect("crear archivo");
    }

    fn list_titles(conn: &mut rusqlite::Connection) -> Vec<(String, String, Option<i64>)> {
        let mut repo = MediaRepository::new(conn);
        repo.list(crate::db::media_repo::ListMediaOptions {
            r#type: None,
            limit: Some(500),
            offset: Some(0),
            order_by: None,
        })
        .expect("listar medios")
        .into_iter()
        .map(|media| (media.title, media.r#type, media.parent_id))
        .collect()
    }

    #[test]
    fn indexa_peliculas_sueltas_series_y_anime() {
        let (db_path, _db_dir) = temp_db();
        let library = TempDir::new().expect("biblioteca temporal");
        let root = library.path();

        touch(&root.join("Inception (2010).mkv"));
        touch(
            &root
                .join("Peliculas")
                .join("Matrix (1999)")
                .join("Matrix.mkv"),
        );
        touch(
            &root
                .join("Series")
                .join("Breaking Bad")
                .join("Season 1")
                .join("Breaking.Bad.S01E01.mkv"),
        );
        touch(
            &root
                .join("Series")
                .join("Breaking Bad")
                .join("Season 1")
                .join("Breaking.Bad.S01E02.mkv"),
        );
        touch(
            &root
                .join("Series")
                .join("Breaking Bad")
                .join("Season 2")
                .join("Breaking.Bad.S02E01.mkv"),
        );
        touch(&root.join("Anime").join("Frieren").join("01.mkv"));
        touch(&root.join("Anime").join("Frieren").join("02.mkv"));

        let result = LibraryService::index_folder(&db_path, root.to_str().unwrap())
            .expect("indexar biblioteca");
        assert_eq!(result.files_added, 7);

        let mut conn = open_db(&db_path).expect("abrir BD");
        let titles = list_titles(&mut conn);

        assert!(titles.contains(&("Inception (2010)".to_string(), "movie".to_string(), None)));
        assert!(titles.contains(&("Matrix".to_string(), "movie".to_string(), None)));
        assert!(titles.contains(&("Breaking Bad".to_string(), "series".to_string(), None)));
        assert!(titles.contains(&("Frieren".to_string(), "anime".to_string(), None)));

        // La serie tiene dos temporadas y tres episodios.
        let series_id: i64 = conn
            .query_row(
                "SELECT id FROM media WHERE title = 'Breaking Bad' AND parent_id IS NULL",
                [],
                |row| row.get(0),
            )
            .expect("id de serie");
        let mut repo = MediaRepository::new(&mut conn);
        let seasons = repo.list_children(series_id).expect("temporadas");
        assert_eq!(seasons.len(), 2);
        let mut total_episodes = 0;
        for season in seasons {
            total_episodes += repo
                .list_files_by_media(season.id)
                .expect("episodios")
                .len();
        }
        assert_eq!(total_episodes, 3);

        // Reindexar es idempotente: no duplica medios ni archivos.
        LibraryService::index_folder(&db_path, root.to_str().unwrap())
            .expect("reindexar biblioteca");
        let mut conn = open_db(&db_path).expect("abrir BD");
        let parents = list_titles(&mut conn);
        assert_eq!(parents.len(), 4);
        let media_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM media", [], |row| row.get(0))
            .expect("contar medios");
        assert_eq!(media_count, 6); // 4 padres + 2 temporadas
    }

    #[test]
    fn indexa_temporadas_con_titulo_delante_y_reasigna_padres_antiguos() {
        let (db_path, _db_dir) = temp_db();
        let library = TempDir::new().expect("biblioteca temporal");
        let root = library.path();
        let season1 = root.join("Solo Leveling").join("Solo Leveling S01");
        let season2 = root.join("Solo Leveling").join("Solo Leveling S02");
        touch(&season1.join("S01E01.mkv"));
        touch(&season2.join("S02E01.mkv"));

        // Simula una BD antigua donde las temporadas se indexaron como series
        // independientes (parent_id NULL), como ocurría antes de esta corrección.
        {
            let conn = open_db(&db_path).expect("abrir BD");
            for season in [&season1, &season2] {
                let title = season.file_name().unwrap().to_string_lossy().to_string();
                conn.execute(
                    "INSERT INTO media (type, title, sort_title, folder_path, parent_id)
                     VALUES ('series', ?1, ?1, ?2, NULL)",
                    rusqlite::params![title, season.to_str().unwrap()],
                )
                .expect("insertar temporada antigua");
            }
        }

        LibraryService::index_folder(&db_path, root.to_str().unwrap()).expect("indexar");

        let mut conn = open_db(&db_path).expect("abrir BD");
        let titles = list_titles(&mut conn);
        assert_eq!(
            titles,
            vec![("Solo Leveling".to_string(), "series".to_string(), None)]
        );

        let series_id: i64 = conn
            .query_row(
                "SELECT id FROM media WHERE title = 'Solo Leveling' AND parent_id IS NULL",
                [],
                |row| row.get(0),
            )
            .expect("id de la serie");
        let mut repo = MediaRepository::new(&mut conn);
        let seasons = repo.list_children(series_id).expect("temporadas");
        assert_eq!(seasons.len(), 2);
        assert!(
            seasons
                .iter()
                .all(|season| season.parent_id == Some(series_id))
        );
    }

    #[test]
    fn indexa_pelicula_con_un_unico_video_en_subcarpeta() {
        let (db_path, _db_dir) = temp_db();
        let library = TempDir::new().expect("biblioteca temporal");
        let root = library.path();
        touch(&root.join("Peliculas").join("El viaje").join("movie.mkv"));

        LibraryService::index_folder(&db_path, root.to_str().unwrap()).expect("indexar");

        let mut conn = open_db(&db_path).expect("abrir BD");
        let titles = list_titles(&mut conn);
        assert_eq!(
            titles,
            vec![("El viaje".to_string(), "movie".to_string(), None)]
        );
    }

    #[test]
    fn indexa_episodios_numerados_como_serie() {
        let (db_path, _db_dir) = temp_db();
        let library = TempDir::new().expect("biblioteca temporal");
        let root = library.path();
        touch(&root.join("Anime").join("Frieren").join("01.mkv"));
        touch(&root.join("Anime").join("Frieren").join("02.mkv"));

        LibraryService::index_folder(&db_path, root.to_str().unwrap()).expect("indexar");

        let mut conn = open_db(&db_path).expect("abrir BD");
        let titles = list_titles(&mut conn);
        assert_eq!(
            titles,
            vec![("Frieren".to_string(), "anime".to_string(), None)]
        );
    }

    #[test]
    fn detecta_nombres_de_episodio_y_temporada() {
        assert!(looks_like_episode("Breaking.Bad.S01E02"));
        assert!(looks_like_episode("Show 1x02"));
        assert!(looks_like_episode("Episodio 3"));
        assert!(looks_like_episode("Capítulo 4"));
        assert!(!looks_like_episode("Inception (2010)"));
        assert!(!looks_like_episode("1917"));

        assert!(is_season_dir(Path::new("/serie/Season 1")));
        assert!(is_season_dir(Path::new("/serie/Temporada 2")));
        assert!(is_season_dir(Path::new("/serie/S01")));
        assert!(is_season_dir(Path::new("/serie/2")));
        assert!(is_season_dir(Path::new("/serie/Solo Leveling S01")));
        assert!(is_season_dir(Path::new("/serie/Solo Leveling S02")));
        assert!(is_season_dir(Path::new("/serie/Solo Leveling - S02")));
        assert!(is_season_dir(Path::new("/serie/Solo Leveling Season 2")));
        assert!(is_season_dir(Path::new("/serie/Solo Leveling Temporada 2")));
        assert!(!is_season_dir(Path::new("/serie/Breaking Bad")));
        assert!(!is_season_dir(Path::new("/serie/Friends")));
        assert!(!is_season_dir(Path::new("/serie/1917")));
    }

    #[test]
    fn remove_folder_elimina_medios_archivos_historial_y_streams() {
        let (db_path, _db_dir) = temp_db();
        let library = TempDir::new().expect("biblioteca temporal");
        let root = library.path();
        touch(&root.join("A").join("Serie A").join("S01E01.mkv"));
        touch(&root.join("B").join("Serie B").join("S01E01.mkv"));

        LibraryService::index_folder(&db_path, root.to_str().unwrap()).expect("indexar");

        let conn = open_db(&db_path).expect("abrir BD");
        let series_a: i64 = conn
            .query_row("SELECT id FROM media WHERE title = 'Serie A'", [], |row| {
                row.get(0)
            })
            .expect("id serie A");
        let file_a: String = conn
            .query_row(
                "SELECT file_path FROM media_files WHERE media_id = ?1",
                [series_a],
                |row| row.get(0),
            )
            .expect("archivo serie A");

        conn.execute(
            "INSERT INTO playback_history (media_id) VALUES (?1)",
            [series_a],
        )
        .expect("historial");
        conn.execute(
            "INSERT INTO media_streams
                (media_id, file_path, stream_type, stream_index)
             VALUES (?1, ?2, 'audio', 0)",
            rusqlite::params![series_a, file_a],
        )
        .expect("stream");
        drop(conn);

        let deleted =
            remove_folder(&db_path, root.join("A").to_str().unwrap()).expect("eliminar carpeta A");
        assert_eq!(deleted, 1);

        let conn = open_db(&db_path).expect("abrir BD");
        let remaining: Vec<String> = conn
            .prepare("SELECT title FROM media ORDER BY title")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(Result::ok)
            .collect();
        assert_eq!(remaining, vec!["Serie B".to_string()]);
        let files: i64 = conn
            .query_row("SELECT COUNT(*) FROM media_files", [], |row| row.get(0))
            .unwrap();
        assert_eq!(files, 1);
        let history: i64 = conn
            .query_row("SELECT COUNT(*) FROM playback_history", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(history, 0);
        let streams: i64 = conn
            .query_row("SELECT COUNT(*) FROM media_streams", [], |row| row.get(0))
            .unwrap();
        assert_eq!(streams, 0);
    }

    #[test]
    fn remove_folder_no_confunde_carpetas_con_prefijos_comunes() {
        let (db_path, _db_dir) = temp_db();
        let library = TempDir::new().expect("biblioteca temporal");
        let root = library.path();
        touch(&root.join("A").join("Serie A").join("S01E01.mkv"));
        touch(&root.join("AB").join("Serie AB").join("S01E01.mkv"));

        LibraryService::index_folder(&db_path, root.to_str().unwrap()).expect("indexar");
        remove_folder(&db_path, root.join("A").to_str().unwrap()).expect("eliminar A");

        let mut conn = open_db(&db_path).expect("abrir BD");
        let titles = list_titles(&mut conn);
        assert_eq!(
            titles,
            vec![("Serie AB".to_string(), "series".to_string(), None)]
        );
    }

    #[test]
    fn cleanup_conserva_medios_de_carpetas_configuradas() {
        let (db_path, _db_dir) = temp_db();
        let library = TempDir::new().expect("biblioteca temporal");
        let root = library.path();
        touch(&root.join("A").join("Serie A").join("S01E01.mkv"));
        touch(&root.join("B").join("Serie B").join("S01E01.mkv"));

        LibraryService::index_folder(&db_path, root.to_str().unwrap()).expect("indexar");
        let folders = vec![root.join("B").to_str().unwrap().to_string()];
        let deleted = cleanup_media_not_in_folders(&db_path, &folders).expect("limpiar");

        assert_eq!(deleted, 1);
        let mut conn = open_db(&db_path).expect("abrir BD");
        let titles = list_titles(&mut conn);
        assert_eq!(
            titles,
            vec![("Serie B".to_string(), "series".to_string(), None)]
        );
    }

    #[test]
    fn index_folder_falla_si_la_carpeta_no_existe() {
        let (db_path, _db_dir) = temp_db();
        let result = LibraryService::index_folder(&db_path, "/ruta/que/no/existe/acryl");
        assert!(result.is_err());
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
