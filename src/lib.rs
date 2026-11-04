//! Acryl - Biblioteca de multimedia para Linux (GTK4 versión)

pub mod config;
pub mod core;
pub mod db;
pub mod ui;

pub use config::Config;
pub use core::library_watcher::{FolderWatcher, WatcherOutcome};
