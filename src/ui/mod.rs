//! Interfaz de usuario GTK4 / Libadwaita.

pub mod i18n;
pub mod main_window;
pub mod media_details;
pub mod media_list;
pub mod poster;
pub mod search_bar;
pub mod settings_window;
pub mod theme;
pub mod track_dialog;

pub use i18n::{TranslationsRef, load_translations};
pub use main_window::MainWindow;
