pub mod commands;
pub mod core;
pub mod db;
pub mod error;
pub mod logging;
pub mod models;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let guard = logging::init(app.handle());
            app.manage(guard);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
