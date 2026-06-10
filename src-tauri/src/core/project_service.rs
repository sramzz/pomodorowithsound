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
