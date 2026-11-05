//! Capa de dominio.

pub mod http_client;
pub mod library;
pub mod library_watcher;
pub mod metadata;
pub mod player;
pub mod poster_service;

pub use library::LibraryService;
pub use library_watcher::{FolderWatcher, WatcherOutcome};
