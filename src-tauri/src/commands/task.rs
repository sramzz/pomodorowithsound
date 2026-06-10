use crate::commands::log_outcome;
use crate::core::task_service;
use crate::db::Db;
use crate::error::AppError;

#[tauri::command]
#[tracing::instrument(skip(db, description))]
pub async fn create_task(
    db: tauri::State<'_, Db>,
    id: String,
    goal_id: String,
    title: String,
    description: Option<String>,
    deadline: Option<String>,
    priority: Option<i64>,
) -> Result<(), AppError> {
    let result = task_service::create_task(
        &db.0,
        &id,
        &goal_id,
        &title,
        description.as_deref(),
        deadline.as_deref(),
        priority.unwrap_or(0),
    )
    .await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db, description))]
pub async fn update_task(
    db: tauri::State<'_, Db>,
    id: String,
    title: String,
    description: Option<String>,
    deadline: Option<String>,
    priority: i64,
) -> Result<(), AppError> {
    let result = task_service::update_task(
        &db.0,
        &id,
        &title,
        description.as_deref(),
        deadline.as_deref(),
        priority,
    )
    .await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn archive_task(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = task_service::archive_task(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn delete_task(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = task_service::delete_task(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn reorder_tasks(
    db: tauri::State<'_, Db>,
    goal_id: String,
    ordered_ids: Vec<String>,
) -> Result<(), AppError> {
    let result = task_service::reorder_tasks(&db.0, &goal_id, &ordered_ids).await;
    log_outcome(&result);
    result
}
