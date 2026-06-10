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
        .invoke_handler(tauri::generate_handler![
            commands::project::list_projects,
            commands::project::create_project,
            commands::project::update_project,
            commands::project::archive_project,
            commands::project::delete_project,
            commands::goal::create_goal,
            commands::goal::update_goal,
            commands::goal::archive_goal,
            commands::goal::delete_goal,
            commands::task::create_task,
            commands::task::update_task,
            commands::task::archive_task,
            commands::task::delete_task,
            commands::microtask::create_microtask,
            commands::microtask::update_microtask,
            commands::microtask::complete_microtask,
            commands::microtask::uncomplete_microtask,
            commands::microtask::archive_microtask,
            commands::microtask::delete_microtask,
            commands::frontend_log::log_frontend,
        ])
        .setup(|app| {
            let guard = logging::init(app.handle());
            app.manage(guard);
            let handle = app.handle().clone();
            let pool = tauri::async_runtime::block_on(db::init(&handle))
                .expect("database initialization failed");
            app.manage(db::Db(pool));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
