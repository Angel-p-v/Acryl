//! Módulo de acceso a datos (db).
//!
//! Re-exporta las funciones de apertura/cierre y los repositorios para que el
//! resto del backend no importe rusqlite directamente.

pub mod connection;
pub mod migrations;
pub mod media_repo;
pub mod file_repo;

pub use connection::{close_db, open_db};
pub use media_repo::{ListMediaOptions, MediaRepository, SearchMediaOptions};
pub use file_repo::FileRepository;
