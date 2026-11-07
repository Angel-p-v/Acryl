//! Capa de dominio.

pub mod http_client;
pub mod language;
pub mod library;
pub mod library_watcher;
pub mod metadata;
pub mod player;
pub mod poster_service;
pub mod progress;

pub use library::LibraryService;
pub use library_watcher::{FolderWatcher, WatcherOutcome};
