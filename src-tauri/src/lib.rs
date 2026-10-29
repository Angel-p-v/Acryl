mod commands;
mod db;
mod media;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_log::Builder::default().build())
    .invoke_handler(tauri::generate_handler![commands::list_media_cmd])
    .run(tauri::generate_context!())
    .expect("error while building tauri application");
}
