//! Módulo de acceso a datos (db).
//!
//! Re-exporta las funciones de apertura/cierre, las migraciones y los
//! repositorios para que el resto del backend no importe rusqlite directamente.

pub mod connection;
pub mod migrations;
pub mod media_repo;
pub mod file_repo;

pub use connection::{close_db, open_db, run_migrations};
pub use migrations::MIGRATIONS;
pub use media_repo::MediaRepository;
pub use file_repo::FileRepository;
