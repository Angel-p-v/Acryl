//! Punto de entrada de la biblioteca Tauri.
//! Registra los comandos expuestos al frontend y el plugin de logs.

mod commands;
mod db;
mod player;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            commands::list_media_cmd,
            commands::search_media_cmd,
            commands::get_media_cmd,
            commands::get_media_by_folder_cmd,
            commands::count_media_cmd,
            commands::list_folders_cmd,
            commands::list_files_by_media_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}
