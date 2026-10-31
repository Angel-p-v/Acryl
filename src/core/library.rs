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
use crate::db::media_repo::{MediaInput, MediaStream, NewMediaStream};
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
                title: Some(media_file_title(video)),
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
                title: Some(media_file_title(video)),
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
                title: Some(media_file_title(video)),
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
    let raw = file_stem(path);
    let cleaned = clean_title(&raw);
    let stem = if cleaned.is_empty() { raw } else { cleaned };
    !stem.is_empty() && stem.len() <= 3 && stem.chars().all(|character| character.is_ascii_digit())
}

/// Detecta carpetas de temporada de forma flexible: `Season 1`, `season1`,
/// `Temporada 2`, `S01`, `s 1`, `2`, `Solo Leveling S01`, `Show Season 2`,
/// `1ª Temporada`, `Solo Leveling S01 [1080p]`, etc.
fn is_season_dir(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default()
        .trim()
        .to_lowercase();

    is_season_name(&name)
}

/// Núcleo de detección de temporadas sobre un nombre ya normalizado a
/// minúsculas. Se reutiliza tanto para carpetas como para grupos entre
/// corchetes o paréntesis del título.
fn is_season_name(name: &str) -> bool {
    // "1", "01", "12"
    if name.len() <= 2 && name.chars().all(|character| character.is_ascii_digit()) {
        return true;
    }

    // "Season 1", "Temporada 2", "Solo Leveling Season 2", "TEMP 1".
    if ["season", "temporada", "temp"]
        .iter()
        .any(|keyword| has_season_number_after(name, keyword))
    {
        return true;
    }

    // "1ª Temporada", "1a temporada", "2º Temporada".
    if ["season", "temporada", "temp"]
        .iter()
        .any(|keyword| has_season_number_before(name, keyword))
    {
        return true;
    }

    // "S01", "T2", "Solo Leveling S01", "Solo Leveling - S02".
    ['s', 't']
        .iter()
        .any(|marker| has_short_season_marker(name, *marker))
}

/// Busca `<keyword> <número>` en cualquier posición, aceptando separadores
/// opcionales (`season1`, `season.2`, `season 3`, `season-4`).
fn has_season_number_after(name: &str, keyword: &str) -> bool {
    name.match_indices(keyword).any(|(position, _)| {
        if !starts_word(name, position) {
            return false;
        }

        let rest = &name[position + keyword.len()..];
        let rest = rest.trim_start_matches(is_season_separator);
        has_season_number(rest)
    })
}

/// Busca `<número> <keyword>` (`1ª temporada`, `1a temporada`) permitiendo el
/// sufijo ordinal español.
fn has_season_number_before(name: &str, keyword: &str) -> bool {
    name.match_indices(keyword).any(|(position, _)| {
        if !starts_word(name, position) {
            return false;
        }

        let prefix = name[..position].trim_end_matches(is_season_separator);
        let prefix = prefix.trim_end_matches(['ª', 'º', '°', 'a', 'o']);
        let prefix = prefix.trim_end_matches(is_season_separator);

        let digits = prefix
            .chars()
            .rev()
            .take_while(|character| character.is_ascii_digit())
            .count();
        if digits == 0 || digits > 3 {
            return false;
        }

        let digits_start = prefix.len() - digits;
        digits_start == 0
            || !prefix[..digits_start]
                .chars()
                .next_back()
                .is_some_and(|character| character.is_alphanumeric())
    })
}

/// Busca una marca corta (`s` o `t`) seguida de un número de temporada.
///
/// Exige inicio de palabra para no confundir `friends` o `star wars` con una
/// temporada, y que después del número no haya más letras para descartar
/// nombres de episodio como `S01E01`.
fn has_short_season_marker(name: &str, marker: char) -> bool {
    name.char_indices().any(|(position, character)| {
        if character != marker || !starts_word(name, position) {
            return false;
        }

        let rest = &name[position + marker.len_utf8()..];
        let rest = rest.trim_start_matches(is_season_separator);
        has_season_number(rest)
    })
}

/// `true` si `rest` empieza por 1-3 dígitos y después no sigue una letra.
fn has_season_number(rest: &str) -> bool {
    let digits = rest
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .count();
    if digits == 0 || digits > 3 {
        return false;
    }

    rest[digits..]
        .chars()
        .next()
        .is_none_or(|character| !character.is_alphanumeric())
}

/// Indica si la posición dada empieza una palabra: inicio del nombre o justo
/// después de un separador.
fn starts_word(name: &str, position: usize) -> bool {
    position == 0
        || name[..position]
            .chars()
            .next_back()
            .is_some_and(is_season_separator)
}

fn is_season_separator(character: char) -> bool {
    character.is_whitespace()
        || matches!(
            character,
            '-' | '_' | '.' | ':' | ',' | '#' | '(' | ')' | '[' | ']' | '·'
        )
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

/// Elimina etiquetas técnicas habituales para obtener el título real:
/// resolución (`1080p`, `2160p`, `4K`), códecs (`x265`, `HEVC`, `H.264`),
/// fuentes (`BluRay`, `WEB-DL`), audio (`DDP5.1`, `DTS`, `AAC`) y grupos de
/// release entre corchetes (`[YTS.MX]`, `[SubsPlease]`).
fn clean_title(raw: &str) -> String {
    let stripped = strip_metadata_groups(raw);

    // Se separa por espacios y guiones bajos; los puntos solo separan cuando
    // forman parte de un nombre de release (`Movie.2024.1080p`), no en
    // abreviaturas como `Dr. Strangelove`.
    let mut parts: Vec<&str> = Vec::new();
    for word in stripped.split(|character: char| character.is_whitespace() || character == '_') {
        if word.is_empty() {
            continue;
        }
        let without_trailing_dot = word.trim_end_matches('.');
        if without_trailing_dot.contains('.') {
            parts.extend(
                without_trailing_dot
                    .split('.')
                    .filter(|part| !part.is_empty()),
            );
        } else {
            parts.push(word);
        }
    }

    let mut tokens: Vec<&str> = Vec::new();
    let mut index = 0;
    while index < parts.len() {
        let token = trim_title_token(parts[index]);
        index += 1;
        if token.is_empty() {
            continue;
        }

        // H.264 / H.265 / X.264 quedan partidos por el punto.
        if (token.eq_ignore_ascii_case("h") || token.eq_ignore_ascii_case("x"))
            && index < parts.len()
        {
            let next = trim_title_token(parts[index]);
            if next.starts_with("264") || next.starts_with("265") {
                break;
            }
        }

        if is_strong_technical_token(token) {
            // A partir de la resolución o el códec, el resto suele ser release
            // (fuente, audio, grupo): se descarta.
            break;
        }
        if is_weak_technical_token(token) {
            // Al principio de la cadena podría ser parte del título real
            // (por ejemplo la película «Dual»): solo se descarta si ya hay
            // título antes.
            if tokens.is_empty() {
                tokens.push(token);
            }
            continue;
        }

        tokens.push(token);
    }

    tokens.join(" ").trim().to_string()
}

/// Quita grupos entre corchetes o paréntesis que no aportan al título.
/// Se conservan los años (`(1999)`) y las marcas de temporada (`[S01]`), que
/// sí son útiles para nombrar el medio.
fn strip_metadata_groups(raw: &str) -> String {
    let mut output = String::with_capacity(raw.len());
    let mut chars = raw.char_indices().peekable();

    while let Some((index, character)) = chars.next() {
        let closing = match character {
            '[' => ']',
            '(' => ')',
            _ => {
                output.push(character);
                continue;
            }
        };

        let Some(relative_end) = raw[index + character.len_utf8()..].find(closing) else {
            output.push(character);
            continue;
        };
        let content_start = index + character.len_utf8();
        let content_end = content_start + relative_end;
        let content = raw[content_start..content_end].trim();

        if is_year(content) {
            output.push(' ');
            if character == '(' {
                output.push('(');
            }
            output.push_str(content);
            if character == '(' {
                output.push(')');
            }
            output.push(' ');
        } else if is_season_name(&content.to_lowercase()) {
            output.push(' ');
            output.push_str(content);
            output.push(' ');
        } else {
            output.push(' ');
        }

        let end = content_end + closing.len_utf8();
        while let Some((position, _)) = chars.peek() {
            if *position < end {
                chars.next();
            } else {
                break;
            }
        }
    }

    output
}

fn is_year(value: &str) -> bool {
    value.len() == 4 && value.chars().all(|character| character.is_ascii_digit())
}

fn trim_title_token(token: &str) -> &str {
    token.trim_matches(['{', '}', ',', ';', '!', '?', '"', '\'', '·'])
}

/// Etiquetas que marcan el inicio de la parte técnica del nombre.
fn is_strong_technical_token(token: &str) -> bool {
    let lower = token.to_lowercase();
    let core =
        lower.trim_matches(|character: char| !character.is_alphanumeric() && character != '-');
    if core.is_empty() {
        return false;
    }

    let base = core.split('-').next().unwrap_or(core);
    let compact = core.replace('-', "");

    is_resolution(base)
        || base.starts_with("x26")
        || base.starts_with("h26")
        || matches!(
            base,
            "hevc"
                | "avc"
                | "av1"
                | "vp9"
                | "xvid"
                | "divx"
                | "10bit"
                | "8bit"
                | "hi10p"
                | "hdr"
                | "hdr10"
                | "dovi"
                | "dolby"
                | "vision"
                | "sdr"
                | "3d"
                | "remux"
                | "bdremux"
                | "dvd"
                | "hdtv"
                | "dvdrip"
                | "hdrip"
                | "camrip"
                | "cam"
                | "ts"
                | "tc"
                | "truehd"
                | "atmos"
                | "flac"
                | "mp3"
                | "opus"
        )
        || matches!(
            compact.as_str(),
            "bluray" | "bdrip" | "brrip" | "webdl" | "webrip" | "web"
        )
        || matches!(compact.as_str(), "aac" | "ac3" | "eac3" | "dts")
        || base.starts_with("ddp")
        || base.starts_with("dts")
        || base.starts_with("aac")
        || base.starts_with("ac3")
        || base.starts_with("eac3")
        || base
            .strip_prefix("dd")
            .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
}

fn is_resolution(base: &str) -> bool {
    if matches!(base, "4k" | "8k" | "uhd" | "fhd" | "qhd" | "hd" | "sd") {
        return true;
    }

    if let Some(digits) = base.strip_suffix(['p', 'i'])
        && matches!(digits.len(), 3 | 4)
        && digits.chars().all(|character| character.is_ascii_digit())
    {
        return true;
    }

    base.strip_suffix("fps").is_some_and(|digits| {
        !digits.is_empty() && digits.chars().all(|character| character.is_ascii_digit())
    })
}

/// Etiquetas que se pueden eliminar sin cortar el resto del título.
fn is_weak_technical_token(token: &str) -> bool {
    let lower = token.to_lowercase();
    let core = lower.trim_matches(|character: char| !character.is_alphanumeric());
    matches!(
        core,
        "audio"
            | "subs"
            | "sub"
            | "multi"
            | "vose"
            | "dual"
            | "proper"
            | "repack"
            | "nfofix"
            | "internal"
            | "dubbed"
            | "subtitled"
    )
}

fn folder_title(path: &Path) -> String {
    let raw = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| path.to_string_lossy().to_string());

    let cleaned = clean_title(&raw);
    if cleaned.is_empty() { raw } else { cleaned }
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

/// Título visible de un archivo de vídeo, sin extensión ni etiquetas técnicas.
fn media_file_title(path: &Path) -> String {
    let raw = file_stem(path);
    let cleaned = clean_title(&raw);
    if cleaned.is_empty() {
        file_name(path)
    } else {
        cleaned
    }
}

fn movie_title(video: &Path) -> String {
    let raw_stem = file_stem(video);
    let cleaned = clean_title(&raw_stem);
    let stem = if cleaned.is_empty() {
        raw_stem
    } else {
        cleaned
    };

    if GENERIC_MOVIE_STEMS.contains(&stem.to_lowercase().as_str())
        && let Some(parent) = video.parent()
    {
        let parent_title = folder_title(parent);
        if !parent_title.is_empty() {
            return parent_title;
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
/// Devuelve los streams con `stream_index` relativo al tipo (audio o subtitle)
/// y empezando en 0, en el orden en que `ffprobe` los encuentra. Para mpv hay
/// que sumar 1, porque numera cada tipo de pista por separado desde 1.
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

/// Extrae y guarda las pistas de los archivos de la biblioteca que todavía no
/// se han analizado. Si se indica `folder_path`, se limita a esa carpeta.
///
/// Se ejecuta en segundo plano desde la indexación para no bloquear la UI. Un
/// archivo que falla (por ejemplo, porque `ffprobe` no entiende el contenedor)
/// no se marca como analizado y se reintentará en la siguiente indexación.
pub fn index_missing_streams(
    db_path: &str,
    folder_path: Option<&str>,
) -> Result<usize, Box<dyn std::error::Error>> {
    if Command::new("ffprobe")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| !status.success())
        .unwrap_or(true)
    {
        return Err("ffprobe no está disponible".into());
    }

    let mut conn = open_db(db_path)?;
    let pending = {
        let mut repo = FileRepository::new(&mut conn);
        repo.list_files_without_streams_index()?
    };

    let mut indexed = 0usize;
    for file_path in pending {
        if let Some(folder) = folder_path
            && !Path::new(&file_path).starts_with(folder)
        {
            continue;
        }
        if !Path::new(&file_path).is_file() {
            continue;
        }

        match index_streams_with_connection(&mut conn, &file_path) {
            Ok(()) => indexed += 1,
            Err(error) => eprintln!(
                "No se pudieron extraer las pistas de '{}': {}",
                file_path, error
            ),
        }
    }

    close_db(conn);
    Ok(indexed)
}

/// Devuelve las pistas de un archivo, extrayéndolas con `ffprobe` si aún no
/// están en la base de datos. Pensado para abrir el selector de pistas de
/// archivos indexados antes de que existiera la extracción automática.
pub fn ensure_media_streams(
    db_path: &str,
    file_path: &str,
) -> Result<Vec<MediaStream>, Box<dyn std::error::Error>> {
    let mut conn = open_db(db_path)?;
    let existing = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.list_streams_by_file(file_path)?
    };
    if !existing.is_empty() {
        close_db(conn);
        return Ok(existing);
    }

    index_streams_with_connection(&mut conn, file_path)?;

    let streams = {
        let mut repo = MediaRepository::new(&mut conn);
        repo.list_streams_by_file(file_path)?
    };
    close_db(conn);
    Ok(streams)
}

/// Extrae los streams de un archivo y los guarda usando una conexión ya
/// abierta. Marca el archivo como analizado solo si la extracción tuvo éxito.
fn index_streams_with_connection(
    conn: &mut rusqlite::Connection,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let media_id = {
        let mut repo = FileRepository::new(conn);
        repo.media_id_for_file(file_path)?
    }
    .ok_or_else(|| {
        format!(
            "el archivo no está indexado en la biblioteca: {}",
            file_path
        )
    })?;

    let streams = extract_media_streams(file_path)?;
    {
        let mut repo = MediaRepository::new(conn);
        repo.replace_streams_for_file(media_id, file_path, &streams)?;
    }
    {
        let mut repo = FileRepository::new(conn);
        repo.mark_streams_indexed(file_path)?;
    }

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
    fn indexa_nombres_con_codecs_y_resolucion() {
        let (db_path, _db_dir) = temp_db();
        let library = TempDir::new().expect("biblioteca temporal");
        let root = library.path();

        touch(
            &root
                .join("Peliculas")
                .join("Dune Part Two (2024) [2160p]")
                .join("movie.1080p.mkv"),
        );
        touch(
            &root
                .join("Series")
                .join("Solo Leveling [1080p]")
                .join("Solo Leveling S01 1080p x265")
                .join("S01E01.1080p.mkv"),
        );
        touch(&root.join("Anime").join("Frieren").join("01.1080p.mkv"));
        touch(&root.join("Anime").join("Frieren").join("02.1080p.mkv"));

        LibraryService::index_folder(&db_path, root.to_str().unwrap()).expect("indexar");

        let mut conn = open_db(&db_path).expect("abrir BD");
        let titles = list_titles(&mut conn);
        assert!(titles.contains(&(
            "Dune Part Two (2024)".to_string(),
            "movie".to_string(),
            None
        )));
        assert!(titles.contains(&("Solo Leveling".to_string(), "series".to_string(), None)));
        assert!(titles.contains(&("Frieren".to_string(), "anime".to_string(), None)));
        assert_eq!(titles.len(), 3, "solo deben mostrarse los medios padre");

        let series_id: i64 = conn
            .query_row(
                "SELECT id FROM media WHERE title = 'Solo Leveling'",
                [],
                |row| row.get(0),
            )
            .expect("id de la serie");
        let anime_id: i64 = conn
            .query_row("SELECT id FROM media WHERE title = 'Frieren'", [], |row| {
                row.get(0)
            })
            .expect("id del anime");

        let mut repo = MediaRepository::new(&mut conn);
        let seasons = repo.list_children(series_id).expect("temporadas");
        assert_eq!(seasons.len(), 1);
        assert_eq!(seasons[0].title, "Solo Leveling S01");

        let episodes = repo.list_files_by_media(anime_id).expect("episodios");
        assert_eq!(episodes.len(), 2);
        assert_eq!(episodes[0].title.as_deref(), Some("01"));
        assert_eq!(episodes[1].title.as_deref(), Some("02"));
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

        let season_folders = [
            "Season 1",
            "season1",
            "SEASON 01",
            "Season.2",
            "Season-3",
            "Season_4",
            "Temporada 1",
            "temporada2",
            "TEMP 1",
            "S01",
            "s01",
            "S1",
            "s 1",
            "S-01",
            "S_01",
            "Solo Leveling S01",
            "Solo Leveling S02",
            "Solo Leveling - S02",
            "Solo Leveling Season 2",
            "Solo Leveling Temporada 2",
            "Solo Leveling S01 [1080p]",
            "Solo Leveling (S02)",
            "Solo Leveling [S02]",
            "1ª Temporada",
            "1a temporada",
            "T2",
            "t 2",
            "1",
            "01",
            "12",
        ];
        for name in season_folders {
            assert!(
                is_season_dir(Path::new(name)),
                "debería detectarse como temporada: {name}"
            );
        }

        let not_season_folders = [
            "Breaking Bad",
            "Friends",
            "1917",
            "S.W.A.T.",
            "Se7en",
            "A Season in Hell",
            "Season of the Witch",
            "Star Wars",
            "S01E01",
            "Show 1x02",
            "Specials",
        ];
        for name in not_season_folders {
            assert!(
                !is_season_dir(Path::new(name)),
                "no debería detectarse como temporada: {name}"
            );
        }
    }

    #[test]
    fn limpia_codecs_resolucion_y_grupos_de_release() {
        let cases = [
            ("Inception.2010.1080p.BluRay.x264-GROUP", "Inception 2010"),
            (
                "Dune Part Two 2024 2160p WEB-DL DDP5.1 H.265",
                "Dune Part Two 2024",
            ),
            ("Movie.2024.H.264-GROUP", "Movie 2024"),
            ("Movie 2024 BluRay x265", "Movie 2024"),
            ("Movie 2024 WEBDL", "Movie 2024"),
            ("Movie 2024 AAC 5.1", "Movie 2024"),
            ("Solo Leveling S01 1080p x265", "Solo Leveling S01"),
            ("Solo Leveling [1080p x265]", "Solo Leveling"),
            ("Solo Leveling (S01) [1080p]", "Solo Leveling S01"),
            ("[SubsPlease] Frieren - 01 [1080p]", "Frieren - 01"),
            ("The Matrix (1999)", "The Matrix (1999)"),
            (
                "Spider-Man: No Way Home (2021)",
                "Spider-Man: No Way Home (2021)",
            ),
            ("Dr. Strangelove", "Dr. Strangelove"),
            ("Dual", "Dual"),
        ];
        for (raw, expected) in cases {
            assert_eq!(clean_title(raw), expected, "limpieza de '{raw}'");
        }
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

    #[test]
    fn index_missing_streams_procesa_pendientes_y_no_repite() {
        if !disponible("ffmpeg") || !disponible("ffprobe") {
            eprintln!("test salteado: se requiere ffmpeg y ffprobe");
            return;
        }

        let (db_path, _db_dir) = temp_db();
        let media_dir = TempDir::new().expect("crear directorio temporal");
        let video = generar_video_prueba(media_dir.path());
        let video_str = video.to_str().unwrap().to_string();

        {
            let conn = open_db(&db_path).expect("abrir BD");
            conn.execute(
                "INSERT INTO media (type, title, sort_title, folder_path)
                 VALUES ('movie', 'Pelicula', 'pelicula', ?1)",
                rusqlite::params![media_dir.path().to_str().unwrap()],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO media_files (media_id, file_path) VALUES (1, ?1)",
                rusqlite::params![&video_str],
            )
            .unwrap();
        }

        assert_eq!(
            index_missing_streams(&db_path, None).expect("indexar pistas"),
            1
        );

        let mut conn = open_db(&db_path).expect("abrir BD");
        let streams = {
            let mut repo = MediaRepository::new(&mut conn);
            repo.list_streams_by_file(&video_str).expect("listar")
        };
        assert!(!streams.is_empty());
        close_db(conn);

        // El archivo ya está marcado como analizado: la segunda pasada no
        // vuelve a ejecutar ffprobe.
        assert_eq!(
            index_missing_streams(&db_path, None).expect("segunda indexación"),
            0
        );
    }

    #[test]
    fn ensure_media_streams_extrae_las_pistas_si_faltan() {
        if !disponible("ffmpeg") || !disponible("ffprobe") {
            eprintln!("test salteado: se requiere ffmpeg y ffprobe");
            return;
        }

        let (db_path, _db_dir) = temp_db();
        let media_dir = TempDir::new().expect("crear directorio temporal");
        let video = generar_video_prueba(media_dir.path());
        let video_str = video.to_str().unwrap().to_string();

        {
            let conn = open_db(&db_path).expect("abrir BD");
            conn.execute(
                "INSERT INTO media (type, title, sort_title, folder_path)
                 VALUES ('movie', 'Pelicula', 'pelicula', ?1)",
                rusqlite::params![media_dir.path().to_str().unwrap()],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO media_files (media_id, file_path) VALUES (1, ?1)",
                rusqlite::params![&video_str],
            )
            .unwrap();
        }

        let streams = ensure_media_streams(&db_path, &video_str).expect("detectar pistas");
        assert!(streams.iter().any(|s| s.stream_type == "audio"));

        // Una segunda llamada reutiliza las pistas guardadas sin invocar de
        // nuevo a ffprobe.
        let again = ensure_media_streams(&db_path, &video_str).expect("reutilizar pistas");
        assert_eq!(again.len(), streams.len());
    }
}
