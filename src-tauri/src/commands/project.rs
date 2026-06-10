use crate::core::project_service;
use crate::db::Db;
use crate::error::AppError;
use crate::models::project::Project;

#[tauri::command]
#[tracing::instrument(skip(db))]
pub async fn list_projects(
    db: tauri::State<'_, Db>,
    include_archived: bool,
) -> Result<Vec<Project>, AppError> {
    let result = project_service::list_projects(&db.0, include_archived).await;
    match &result {
        Ok(p) => tracing::info!(count = p.len(), "ok"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}
