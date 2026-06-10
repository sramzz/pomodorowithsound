use crate::commands::log_outcome;
use crate::core::pomodoro_type_service;
use crate::db::Db;
use crate::error::AppError;
use crate::models::pomodoro_type::PomodoroType;

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn list_pomodoro_types(
    db: tauri::State<'_, Db>,
) -> Result<Vec<PomodoroType>, AppError> {
    let result = pomodoro_type_service::list_pomodoro_types(&db.0).await;
    match &result {
        Ok(types) => tracing::info!(count = types.len(), "ok"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn create_pomodoro_type(
    db: tauri::State<'_, Db>,
    id: String,
    name: String,
    work_minutes: i64,
    rest_minutes: i64,
    long_break_minutes: Option<i64>,
    long_break_every: Option<i64>,
) -> Result<(), AppError> {
    let result = pomodoro_type_service::create_pomodoro_type(
        &db.0, &id, &name, work_minutes, rest_minutes, long_break_minutes, long_break_every,
    )
    .await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn update_pomodoro_type(
    db: tauri::State<'_, Db>,
    id: String,
    name: String,
    work_minutes: i64,
    rest_minutes: i64,
    long_break_minutes: Option<i64>,
    long_break_every: Option<i64>,
) -> Result<(), AppError> {
    let result = pomodoro_type_service::update_pomodoro_type(
        &db.0, &id, &name, work_minutes, rest_minutes, long_break_minutes, long_break_every,
    )
    .await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn delete_pomodoro_type(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = pomodoro_type_service::delete_pomodoro_type(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn set_default_pomodoro_type(
    db: tauri::State<'_, Db>,
    id: String,
) -> Result<(), AppError> {
    let result = pomodoro_type_service::set_default_pomodoro_type(&db.0, &id).await;
    log_outcome(&result);
    result
}
