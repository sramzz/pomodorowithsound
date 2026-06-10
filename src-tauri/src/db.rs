use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use tauri::Manager;

/// Tauri managed state wrapper for the single app-wide pool.
pub struct Db(pub SqlitePool);

pub async fn init(app: &tauri::AppHandle) -> Result<SqlitePool, sqlx::Error> {
    let dir = app.path().app_data_dir().expect("no app data dir");
    std::fs::create_dir_all(&dir).ok();
    let path = dir.join("focus-planner.sqlite");
    tracing::info!(db_path = %path.display(), "opening database");

    let options = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new().connect_with(options).await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("migrations applied");
    Ok(pool)
}
