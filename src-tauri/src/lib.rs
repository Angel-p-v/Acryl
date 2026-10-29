//! Punto de entrada de la biblioteca Tauri.
//! Registra los comandos expuestos al frontend y el plugin de logs.

mod commands;
mod config;
mod db;
mod player;

use crate::config::ConfigState;
use crate::player::PlayerState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .manage(PlayerState::new())
        .manage(ConfigState::new().expect("no se pudo cargar la configuración"))
        .invoke_handler(tauri::generate_handler![
            commands::list_media_cmd,
            commands::search_media_cmd,
            commands::get_media_cmd,
            commands::get_media_by_folder_cmd,
            commands::count_media_cmd,
            commands::list_folders_cmd,
            commands::list_files_by_media_cmd,
            commands::get_media_details_cmd,
            commands::toggle_favorite_cmd,
            commands::toggle_watched_cmd,
            commands::player_open_cmd,
            commands::player_play_cmd,
            commands::player_pause_cmd,
            commands::player_stop_cmd,
            commands::player_seek_cmd,
            commands::player_volume_cmd,
            commands::player_status_cmd,
            commands::config_get_cmd,
            commands::config_set_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
