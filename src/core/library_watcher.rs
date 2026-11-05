//! Vigilador de carpetas de biblioteca con `notify`.
//!
//! Detecta cambios (creación, modificación, eliminación de archivos) en las
//! carpetas indexadas y dispara un reindexado automático. Incluye un debounce
//! de 2 segundos para evitar reindexados excesivos al copiar muchos archivos.

use crate::core::library::LibraryService;
use notify::{Event, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

/// Resultado de un intento de reindexado automático.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatcherOutcome {
    /// Se disparó un reindexado de la carpeta.
    Reindexed { folder: String, files_added: usize },
    /// El evento fue ignorado (no es un cambio relevante).
    Ignored,
    /// Error al reindexar.
    Error { folder: String, message: String },
}

/// Extensiones de vídeo relevantes para el watcher.
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mkv", "avi", "webm", "mov", "flv", "wmv"];

/// Comprueba si una ruta de archivo tiene una extensión de vídeo relevante.
pub fn is_video_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| VIDEO_EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)))
        .unwrap_or(false)
}

/// Comprueba si un evento de `notify` contiene al menos un archivo de vídeo.
fn is_video_event(event: &notify::Event) -> bool {
    event.paths.iter().any(|path| is_video_path(path))
}

/// Vigilador de una carpeta de biblioteca.
pub struct FolderWatcher {
    #[allow(dead_code)]
    watcher: notify::RecommendedWatcher,
    folder: String,
    db_path: String,
    rx: mpsc::Receiver<notify::Result<Event>>,
    debounce_secs: u64,
}

impl FolderWatcher {
    /// Crea un vigilador para `folder_path`. Cuando detecta cambios relevantes,
    /// reindexa la carpeta usando `LibraryService::index_folder`.
    ///
    /// El `debounce_secs` evita reindexar múltiples veces cuando se copian
    /// muchos archivos a la vez.
    pub fn new(
        folder_path: &str,
        db_path: &str,
        debounce_secs: u64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let (tx, rx) = mpsc::channel();
        let mut watcher: notify::RecommendedWatcher = notify::RecommendedWatcher::new(
            move |res| {
                let _ = tx.send(res);
            },
            notify::Config::default(),
        )?;
        watcher.watch(Path::new(folder_path), RecursiveMode::Recursive)?;

        Ok(Self {
            watcher,
            folder: folder_path.to_string(),
            db_path: db_path.to_string(),
            rx,
            debounce_secs,
        })
    }

    /// Ejecuta el bucle de eventos del watcher. Debe llamarse en un hilo
    /// secundario para no bloquear la UI. Devuelve cuando el watcher se detiene.
    pub fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        let mut pending: Option<PathBuf> = None;
        let mut last_event: Option<std::time::Instant> = None;

        // Bucle principal: recibe eventos con timeout para poder hacer debounce.
        // Cuando no hay eventos durante `debounce_secs`, se ejecuta el
        // reindexado pendiente.
        loop {
            let event_ok = self.rx.recv_timeout(Duration::from_millis(200));
            let now = std::time::Instant::now();

            if let Ok(Ok(event)) = event_ok
                && is_video_event(&event)
            {
                for path in &event.paths {
                    pending = Some(path.clone());
                    last_event = Some(now);
                }
            }

            // Si ya pasó el tiempo de debounce desde el último evento,
            // ejecutar el reindexado pendiente.
            #[allow(clippy::collapsible_if)]
            if let Some(last) = last_event {
                if now.duration_since(last) >= Duration::from_secs(self.debounce_secs) {
                    if let Some(folder) = pending.take() {
                        let _ = Self::reindex_folder(
                            &self.db_path,
                            folder.to_str().unwrap_or(&self.folder),
                        );
                    }
                    last_event = None;
                }
            }
        }
    }

    /// Reindexa la carpeta y devuelve el resultado.
    fn reindex_folder(
        db_path: &str,
        folder: &str,
    ) -> Result<WatcherOutcome, Box<dyn std::error::Error>> {
        let result = LibraryService::index_folder(db_path, folder)?;
        Ok(WatcherOutcome::Reindexed {
            folder: folder.to_string(),
            files_added: result.files_added as usize,
        })
    }
}

impl Drop for FolderWatcher {
    fn drop(&mut self) {
        // El watcher se detiene al ser dropeado.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_video_path_reconoce_extensiones_de_video() {
        assert!(is_video_path(std::path::Path::new("/tmp/test/video.mkv")));
        assert!(is_video_path(std::path::Path::new("/tmp/test/video.MP4")));
        assert!(!is_video_path(std::path::Path::new(
            "/tmp/test/documento.txt"
        )));
        assert!(!is_video_path(std::path::Path::new("/tmp/test/imagen.jpg")));
        assert!(!is_video_path(std::path::Path::new(
            "/tmp/test/sin_extension"
        )));
    }

    #[test]
    fn watcher_outcome_variantes() {
        let _ = WatcherOutcome::Ignored;
        let _ = WatcherOutcome::Reindexed {
            folder: "/tmp/test".to_string(),
            files_added: 5,
        };
        let _ = WatcherOutcome::Error {
            folder: "/tmp/test".to_string(),
            message: "error de prueba".to_string(),
        };
    }

    #[test]
    fn is_video_event_detecta_archivos_de_video() {
        use notify::Event;
        use notify::EventKind;

        // Evento con ruta de vídeo.
        let evento_video = Event {
            kind: EventKind::Any,
            paths: vec![std::path::PathBuf::from("/tmp/test/video.mkv")],
            attrs: Default::default(),
        };
        assert!(is_video_event(&evento_video));

        // Evento con ruta de vídeo en mayúsculas.
        let evento_video_may = Event {
            kind: EventKind::Any,
            paths: vec![std::path::PathBuf::from("/tmp/test/video.MP4")],
            attrs: Default::default(),
        };
        assert!(is_video_event(&evento_video_may));

        // Evento con ruta de no-vídeo.
        let evento_no_video = Event {
            kind: EventKind::Any,
            paths: vec![std::path::PathBuf::from("/tmp/test/documento.txt")],
            attrs: Default::default(),
        };
        assert!(!is_video_event(&evento_no_video));

        // Evento con múltiples rutas, una es vídeo.
        let evento_mixto = Event {
            kind: EventKind::Any,
            paths: vec![
                std::path::PathBuf::from("/tmp/test/documento.txt"),
                std::path::PathBuf::from("/tmp/test/pelicula.mkv"),
            ],
            attrs: Default::default(),
        };
        assert!(is_video_event(&evento_mixto));
    }
}
