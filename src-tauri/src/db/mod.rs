//! Módulo de acceso a datos (db).
//!
//! Re-exporta las funciones de apertura/cierre y los repositorios para que el
//! resto del backend no importe rusqlite directamente.

pub mod connection;
pub mod file_repo;
pub mod media_repo;
pub mod migrations;

pub use connection::{close_db, open_db};
pub use file_repo::FileRepository;
pub use media_repo::{ListMediaOptions, MediaRepository, SearchMediaOptions};
