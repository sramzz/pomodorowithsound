use crate::core::time::now_iso8601;
use crate::error::AppError;
use sqlx::SqlitePool;

pub async fn create_task(
    pool: &SqlitePool,
    id: &str,
    goal_id: &str,
    title: &str,
    description: Option<&str>,
    deadline: Option<&str>,
    priority: i64,
) -> Result<(), AppError> {
    let title = title.trim();
    if title.is_empty() {
        tracing::warn!(id, "validation: task title must not be empty");
        return Err(AppError::Validation("task title must not be empty".into()));
    }
    let parent = sqlx::query!("SELECT id FROM goals WHERE id = ?", goal_id)
        .fetch_optional(pool)
        .await?;
    if parent.is_none() {
        return Err(AppError::NotFound { entity: "goal", id: goal_id.to_string() });
    }
    let now = now_iso8601();
    sqlx::query!(
        "INSERT INTO tasks (id, goal_id, title, description, deadline, priority, sort_order,
                            status, is_archived, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?,
                 (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM tasks WHERE goal_id = ?),
                 'open', 0, ?, ?)",
        id, goal_id, title, description, deadline, priority, goal_id, now, now
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_task(
    pool: &SqlitePool,
    id: &str,
    title: &str,
    description: Option<&str>,
    deadline: Option<&str>,
    priority: i64,
) -> Result<(), AppError> {
    let title = title.trim();
    if title.is_empty() {
        tracing::warn!(id, "validation: task title must not be empty");
        return Err(AppError::Validation("task title must not be empty".into()));
    }
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE tasks SET title = ?, description = ?, deadline = ?, priority = ?, updated_at = ?
         WHERE id = ?",
        title, description, deadline, priority, now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "task", id: id.to_string() });
    }
    Ok(())
}

pub async fn archive_task(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE tasks SET is_archived = 1, updated_at = ? WHERE id = ?",
        now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "task", id: id.to_string() });
    }
    Ok(())
}

pub async fn delete_task(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let result = sqlx::query!("DELETE FROM tasks WHERE id = ?", id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "task", id: id.to_string() });
    }
    Ok(())
}
