use crate::commands::log_outcome;
use crate::core::microtask_service;
use crate::db::Db;
use crate::error::AppError;

#[tauri::command]
#[tracing::instrument(skip(db))]
#[allow(clippy::too_many_arguments)]
pub async fn create_microtask(
    db: tauri::State<'_, Db>,
    id: String,
    task_id: String,
    title: String,
    estimated_minutes: i64,
    pomodoro_count: i64,
    pomodoro_type_id: Option<String>,
    deadline: Option<String>,
    priority: Option<i64>,
) -> Result<(), AppError> {
    let result = microtask_service::create_microtask(
        &db.0,
        &id,
        &task_id,
        &title,
        estimated_minutes,
        pomodoro_count,
        pomodoro_type_id.as_deref(),
        deadline.as_deref(),
        priority.unwrap_or(0),
    )
    .await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
#[allow(clippy::too_many_arguments)]
pub async fn update_microtask(
    db: tauri::State<'_, Db>,
    id: String,
    title: String,
    estimated_minutes: i64,
    pomodoro_count: i64,
    pomodoro_type_id: Option<String>,
    deadline: Option<String>,
    priority: i64,
) -> Result<(), AppError> {
    let result = microtask_service::update_microtask(
        &db.0,
        &id,
        &title,
        estimated_minutes,
        pomodoro_count,
        pomodoro_type_id.as_deref(),
        deadline.as_deref(),
        priority,
    )
    .await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn complete_microtask(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = microtask_service::complete_microtask(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn uncomplete_microtask(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = microtask_service::uncomplete_microtask(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn archive_microtask(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = microtask_service::archive_microtask(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn delete_microtask(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = microtask_service::delete_microtask(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn reorder_microtasks(
    db: tauri::State<'_, Db>,
    task_id: String,
    ordered_ids: Vec<String>,
) -> Result<(), AppError> {
    let result = microtask_service::reorder_microtasks(&db.0, &task_id, &ordered_ids).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn get_microtask(
    db: tauri::State<'_, Db>,
    id: String,
) -> Result<crate::models::microtask::Microtask, AppError> {
    let result = microtask_service::get_microtask(&db.0, &id).await;
    log_outcome(&result);
    result
}
