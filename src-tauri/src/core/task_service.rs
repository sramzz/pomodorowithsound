use crate::core::time::now_iso8601;
use crate::error::AppError;
use sqlx::SqlitePool;
use std::collections::HashSet;

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
    let now = now_iso8601();
    let mut tx = pool.begin().await?;
    let result = sqlx::query!(
        "INSERT INTO tasks (id, goal_id, title, description, deadline, priority, sort_order,
                            status, is_archived, created_at, updated_at)
         SELECT ?1, id, ?2, ?3, ?4, ?5,
                (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM tasks WHERE goal_id = ?6),
                'open', 0, ?7, ?8
         FROM goals WHERE id = ?9 AND status = 'open' AND is_archived = 0",
        id, title, description, deadline, priority, goal_id, now, now, goal_id
    )
    .execute(&mut *tx)
    .await?;

    if result.rows_affected() == 0 {
        let goal = sqlx::query!("SELECT status, is_archived FROM goals WHERE id = ?", goal_id)
            .fetch_optional(&mut *tx)
            .await?;
        if let Some(g) = goal {
            if g.status == "completed" {
                return Err(AppError::Validation("cannot create a task under a completed goal".into()));
            } else {
                return Err(AppError::Validation("cannot create a task under an archived goal".into()));
            }
        } else {
            return Err(AppError::NotFound { entity: "goal", id: goal_id.to_string() });
        }
    }
    tx.commit().await?;
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

pub async fn reorder_tasks(
    pool: &SqlitePool,
    goal_id: &str,
    ordered_ids: &[String],
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let existing: Vec<String> = sqlx::query_scalar!(
        r#"SELECT id as "id!: String" FROM tasks WHERE goal_id = ? AND is_archived = 0"#,
        goal_id
    )
    .fetch_all(&mut *tx)
    .await?;
    let unique_ids: HashSet<&String> = ordered_ids.iter().collect();
    if existing.len() != ordered_ids.len()
        || unique_ids.len() != ordered_ids.len()
        || !ordered_ids.iter().all(|id| existing.contains(id))
    {
        tracing::warn!(
            goal_id,
            expected = existing.len(),
            got = ordered_ids.len(),
            "validation: reorder_tasks needs the full ordered list of the goal's non-archived tasks"
        );
        return Err(AppError::Validation(
            "ordered_ids must contain exactly the goal's non-archived tasks".into(),
        ));
    }
    let now = now_iso8601();
    for (index, id) in ordered_ids.iter().enumerate() {
        let index = index as i64;
        sqlx::query!(
            "UPDATE tasks SET sort_order = ?, updated_at = ? WHERE id = ?",
            index, now, id
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
