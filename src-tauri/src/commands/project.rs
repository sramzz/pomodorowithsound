use crate::commands::log_outcome;
use crate::core::project_service;
use crate::db::Db;
use crate::error::AppError;
use crate::models::project::ProjectSummary;

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn list_projects(
    db: tauri::State<'_, Db>,
    include_archived: bool,
) -> Result<Vec<ProjectSummary>, AppError> {
    // Command template every later phase copies: thin wrapper over the Core
    // service that logs the outcome (the #[instrument] span already records args).
    let result = project_service::list_projects(&db.0, include_archived).await;
    match &result {
        Ok(p) => tracing::info!(count = p.len(), "ok"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}

#[tauri::command]
#[tracing::instrument(skip(db, description))]
pub async fn create_project(
    db: tauri::State<'_, Db>,
    id: String,
    name: String,
    description: Option<String>,
) -> Result<(), AppError> {
    let result = project_service::create_project(&db.0, &id, &name, description.as_deref()).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db, description))]
pub async fn update_project(
    db: tauri::State<'_, Db>,
    id: String,
    name: String,
    description: Option<String>,
) -> Result<(), AppError> {
    let result = project_service::update_project(&db.0, &id, &name, description.as_deref()).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn archive_project(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = project_service::archive_project(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn delete_project(db: tauri::State<'_, Db>, id: String) -> Result<(), AppError> {
    let result = project_service::delete_project(&db.0, &id).await;
    log_outcome(&result);
    result
}

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn get_project_tree(
    db: tauri::State<'_, Db>,
    project_id: String,
) -> Result<crate::models::tree::ProjectTree, AppError> {
    let result = project_service::get_project_tree(&db.0, &project_id).await;
    log_outcome(&result);
    result
}
