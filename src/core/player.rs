//! Servicio de reproductor: bindings a libmpv.

use crate::db::{MediaRepository, close_db, open_db};

/// Servicio de reproductor: gestiona reproducción de archivos de vídeo.
pub struct PlayerService;

impl PlayerService {
    /// Abre un archivo de vídeo para reproducción.
    pub fn open(
        db_path: &str,
        media_id: i64,
        file_path: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut conn = open_db(db_path)?;
        let mut media_repo = MediaRepository::new(&mut conn);

        // Registrar reproducción
        media_repo.record_playback(media_id, 0.0, 0.0)?;

        close_db(conn);

        // Aquí se abriría el archivo con libmpv vía FFI
        // Por ahora solo registramos la intención de reproducir
        eprintln!("Reproduciendo: {}", file_path);

        Ok(())
    }

    /// Obtiene la lista de archivos de vídeo de un medio.
    pub fn get_playable_files(
        db_path: &str,
        media_id: i64,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut conn = open_db(db_path)?;
        let mut media_repo = MediaRepository::new(&mut conn);
        let files = media_repo.list_files_by_media(media_id)?;
        close_db(conn);

        Ok(files.into_iter().map(|f| f.file_path).collect())
    }
}

/// Lista los archivos de vídeo de un medio.
pub fn list_media_files(
    db_path: &str,
    media_id: i64,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    PlayerService::get_playable_files(db_path, media_id)
}

/// Reproduce un archivo de vídeo.
pub fn play_file(
    db_path: &str,
    media_id: i64,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    PlayerService::open(db_path, media_id, file_path)
}
