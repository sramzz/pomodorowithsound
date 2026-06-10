use crate::commands::log_outcome;
use crate::core::goal_service;
use crate::db::Db;
use crate::error::AppError;

#[tauri::command]
#[tracing::instrument(skip(db, description))]
pub async fn create_goal(
    db: tauri::State<'_, Db>,
    id: String,
    project_id: String,
    title: String,
    description: Option<String>,
    deadline: Option<String>,
    priority: Option<i64>,
) -> Result<(), AppError> {
    let result = goal_service::create_goal(
        &db.0,
        &id,
        &project_id,
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
pub async fn update_goal(
    db: tauri::State<'_, Db>,
    id: String,
    title: String,
    description: Option<String>,
    deadline: Option<String>,
    priority: i64,
) -> Result<(), AppError> {
    let result = goal_service::update_goal(
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
pub async fn archive_goal(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = goal_service::archive_goal(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn delete_goal(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = goal_service::delete_goal(&db.0, &id).await;
    log_outcome(&result);
    result
}
