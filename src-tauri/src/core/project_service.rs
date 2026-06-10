use crate::core::time::now_iso8601;
use crate::error::AppError;
use crate::models::project::Project;
use sqlx::SqlitePool;

pub async fn list_projects(pool: &SqlitePool, include_archived: bool) -> Result<Vec<Project>, AppError> {
    let projects = sqlx::query_as!(
        Project,
        r#"SELECT
                  id as "id!",
                  name as "name!",
                  description,
                  status as "status!",
                  is_archived as "is_archived!: bool",
                  completed_at,
                  created_at as "created_at!",
                  updated_at as "updated_at!"
           FROM projects
           WHERE is_archived = 0 OR ?1 = 1
           ORDER BY created_at"#,
        include_archived
    )
    .fetch_all(pool)
    .await?;
    Ok(projects)
}

pub async fn create_project(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<(), AppError> {
    let name = name.trim();
    if name.is_empty() {
        tracing::warn!(id, "validation: project name must not be empty");
        return Err(AppError::Validation("project name must not be empty".into()));
    }
    let now = now_iso8601();
    sqlx::query!(
        "INSERT INTO projects (id, name, description, status, is_archived, created_at, updated_at)
         VALUES (?, ?, ?, 'open', 0, ?, ?)",
        id, name, description, now, now
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_project(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<(), AppError> {
    let name = name.trim();
    if name.is_empty() {
        tracing::warn!(id, "validation: project name must not be empty");
        return Err(AppError::Validation("project name must not be empty".into()));
    }
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE projects SET name = ?, description = ?, updated_at = ? WHERE id = ?",
        name, description, now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "project", id: id.to_string() });
    }
    Ok(())
}

pub async fn archive_project(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE projects SET is_archived = 1, updated_at = ? WHERE id = ?",
        now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "project", id: id.to_string() });
    }
    Ok(())
}

pub async fn delete_project(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let result = sqlx::query!("DELETE FROM projects WHERE id = ?", id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "project", id: id.to_string() });
    }
    Ok(())
}
