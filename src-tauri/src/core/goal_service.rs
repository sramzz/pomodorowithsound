use crate::core::time::now_iso8601;
use crate::error::AppError;
use sqlx::SqlitePool;
use std::collections::HashSet;

pub async fn create_goal(
    pool: &SqlitePool,
    id: &str,
    project_id: &str,
    title: &str,
    description: Option<&str>,
    deadline: Option<&str>,
    priority: i64,
) -> Result<(), AppError> {
    let title = title.trim();
    if title.is_empty() {
        tracing::warn!(id, "validation: goal title must not be empty");
        return Err(AppError::Validation("goal title must not be empty".into()));
    }
    let parent = sqlx::query!("SELECT id FROM projects WHERE id = ?", project_id)
        .fetch_optional(pool)
        .await?;
    if parent.is_none() {
        return Err(AppError::NotFound { entity: "project", id: project_id.to_string() });
    }
    let now = now_iso8601();
    sqlx::query!(
        "INSERT INTO goals (id, project_id, title, description, deadline, priority, sort_order,
                            status, is_archived, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?,
                 (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM goals WHERE project_id = ?),
                 'open', 0, ?, ?)",
        id, project_id, title, description, deadline, priority, project_id, now, now
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_goal(
    pool: &SqlitePool,
    id: &str,
    title: &str,
    description: Option<&str>,
    deadline: Option<&str>,
    priority: i64,
) -> Result<(), AppError> {
    let title = title.trim();
    if title.is_empty() {
        tracing::warn!(id, "validation: goal title must not be empty");
        return Err(AppError::Validation("goal title must not be empty".into()));
    }
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE goals SET title = ?, description = ?, deadline = ?, priority = ?, updated_at = ?
         WHERE id = ?",
        title, description, deadline, priority, now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "goal", id: id.to_string() });
    }
    Ok(())
}

pub async fn archive_goal(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE goals SET is_archived = 1, updated_at = ? WHERE id = ?",
        now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "goal", id: id.to_string() });
    }
    Ok(())
}

pub async fn delete_goal(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let result = sqlx::query!("DELETE FROM goals WHERE id = ?", id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "goal", id: id.to_string() });
    }
    Ok(())
}

pub async fn reorder_goals(
    pool: &SqlitePool,
    project_id: &str,
    ordered_ids: &[String],
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let existing: Vec<String> = sqlx::query_scalar!(
        r#"SELECT id as "id!: String" FROM goals WHERE project_id = ? AND is_archived = 0"#,
        project_id
    )
    .fetch_all(&mut *tx)
    .await?;
    let unique_ids: HashSet<&String> = ordered_ids.iter().collect();
    if existing.len() != ordered_ids.len()
        || unique_ids.len() != ordered_ids.len()
        || !ordered_ids.iter().all(|id| existing.contains(id))
    {
        tracing::warn!(
            project_id,
            expected = existing.len(),
            got = ordered_ids.len(),
            "validation: reorder_goals needs the full ordered list of the project's non-archived goals"
        );
        return Err(AppError::Validation(
            "ordered_ids must contain exactly the project's non-archived goals".into(),
        ));
    }
    let now = now_iso8601();
    for (index, id) in ordered_ids.iter().enumerate() {
        let index = index as i64;
        sqlx::query!(
            "UPDATE goals SET sort_order = ?, updated_at = ? WHERE id = ?",
            index, now, id
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
