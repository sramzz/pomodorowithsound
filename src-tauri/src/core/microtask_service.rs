use crate::core::time::now_iso8601;
use crate::error::AppError;
use sqlx::SqlitePool;

fn validate_microtask_fields(
    title: &str,
    estimated_minutes: i64,
    pomodoro_count: i64,
) -> Result<(), AppError> {
    if title.trim().is_empty() {
        tracing::warn!("validation: microtask title must not be empty");
        return Err(AppError::Validation("microtask title must not be empty".into()));
    }
    if estimated_minutes < 1 {
        tracing::warn!(estimated_minutes, "validation: estimated_minutes must be >= 1");
        return Err(AppError::Validation("estimated_minutes must be >= 1".into()));
    }
    if pomodoro_count < 1 {
        tracing::warn!(pomodoro_count, "validation: pomodoro_count must be >= 1");
        return Err(AppError::Validation("pomodoro_count must be >= 1".into()));
    }
    Ok(())
}

async fn ensure_pomodoro_type_exists(
    pool: &SqlitePool,
    pomodoro_type_id: Option<&str>,
) -> Result<(), AppError> {
    if let Some(type_id) = pomodoro_type_id {
        let exists = sqlx::query!("SELECT id FROM pomodoro_types WHERE id = ?", type_id)
            .fetch_optional(pool)
            .await?;
        if exists.is_none() {
            return Err(AppError::NotFound { entity: "pomodoro_type", id: type_id.to_string() });
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn create_microtask(
    pool: &SqlitePool,
    id: &str,
    task_id: &str,
    title: &str,
    estimated_minutes: i64,
    pomodoro_count: i64,
    pomodoro_type_id: Option<&str>,
    deadline: Option<&str>,
    priority: i64,
) -> Result<(), AppError> {
    validate_microtask_fields(title, estimated_minutes, pomodoro_count)?;
    let title = title.trim();
    let parent = sqlx::query!("SELECT id FROM tasks WHERE id = ?", task_id)
        .fetch_optional(pool)
        .await?;
    if parent.is_none() {
        return Err(AppError::NotFound { entity: "task", id: task_id.to_string() });
    }
    ensure_pomodoro_type_exists(pool, pomodoro_type_id).await?;
    let now = now_iso8601();
    sqlx::query!(
        "INSERT INTO microtasks (id, task_id, title, estimated_minutes, pomodoro_count,
                                 pomodoro_type_id, deadline, priority, sort_order,
                                 status, is_archived, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?,
                 (SELECT COALESCE(MAX(sort_order) + 1, 0) FROM microtasks WHERE task_id = ?),
                 'open', 0, ?, ?)",
        id, task_id, title, estimated_minutes, pomodoro_count,
        pomodoro_type_id, deadline, priority, task_id, now, now
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn update_microtask(
    pool: &SqlitePool,
    id: &str,
    title: &str,
    estimated_minutes: i64,
    pomodoro_count: i64,
    pomodoro_type_id: Option<&str>,
    deadline: Option<&str>,
    priority: i64,
) -> Result<(), AppError> {
    validate_microtask_fields(title, estimated_minutes, pomodoro_count)?;
    let title = title.trim();
    ensure_pomodoro_type_exists(pool, pomodoro_type_id).await?;
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE microtasks SET title = ?, estimated_minutes = ?, pomodoro_count = ?,
                               pomodoro_type_id = ?, deadline = ?, priority = ?, updated_at = ?
         WHERE id = ?",
        title, estimated_minutes, pomodoro_count, pomodoro_type_id, deadline, priority, now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "microtask", id: id.to_string() });
    }
    Ok(())
}

pub async fn archive_microtask(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE microtasks SET is_archived = 1, updated_at = ? WHERE id = ?",
        now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "microtask", id: id.to_string() });
    }
    Ok(())
}

pub async fn delete_microtask(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let result = sqlx::query!("DELETE FROM microtasks WHERE id = ?", id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "microtask", id: id.to_string() });
    }
    Ok(())
}

/// Roll-up rule (spec §3), one transaction: completing the last open,
/// non-archived microtask of a task completes the task, which may complete
/// the goal when its last open task completes. Stops at the goal.
pub async fn complete_microtask(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    let micro = sqlx::query!("SELECT task_id, status FROM microtasks WHERE id = ?", id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound { entity: "microtask", id: id.to_string() })?;
    if micro.status == "completed" {
        tracing::info!(microtask_id = id, "already completed - no-op");
        return Ok(());
    }

    let now = now_iso8601();
    sqlx::query!(
        "UPDATE microtasks SET status = 'completed', completed_at = ?, updated_at = ? WHERE id = ?",
        now, now, id
    )
    .execute(&mut *tx)
    .await?;
    let mut chain = format!("microtask {id} completed");

    let open_siblings = sqlx::query!(
        r#"SELECT COUNT(*) as "cnt: i64" FROM microtasks
           WHERE task_id = ? AND status = 'open' AND is_archived = 0"#,
        micro.task_id
    )
    .fetch_one(&mut *tx)
    .await?;

    if open_siblings.cnt == 0 {
        sqlx::query!(
            "UPDATE tasks SET status = 'completed', completed_at = ?, updated_at = ?
             WHERE id = ? AND status = 'open'",
            now, now, micro.task_id
        )
        .execute(&mut *tx)
        .await?;
        chain.push_str(&format!(" -> task {} completed", micro.task_id));

        let goal_id = sqlx::query!("SELECT goal_id FROM tasks WHERE id = ?", micro.task_id)
            .fetch_one(&mut *tx)
            .await?
            .goal_id;
        let open_tasks = sqlx::query!(
            r#"SELECT COUNT(*) as "cnt: i64" FROM tasks
               WHERE goal_id = ? AND status = 'open' AND is_archived = 0"#,
            goal_id
        )
        .fetch_one(&mut *tx)
        .await?;
        if open_tasks.cnt == 0 {
            sqlx::query!(
                "UPDATE goals SET status = 'completed', completed_at = ?, updated_at = ?
                 WHERE id = ? AND status = 'open'",
                now, now, goal_id
            )
            .execute(&mut *tx)
            .await?;
            chain.push_str(&format!(" -> goal {goal_id} completed"));
        }
    }

    tx.commit().await?;
    tracing::info!("{chain}");
    Ok(())
}

/// Reverses the roll-up, one transaction: a task with an open microtask
/// cannot stay completed, nor can its goal — both reopen if completed.
pub async fn uncomplete_microtask(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    let micro = sqlx::query!("SELECT task_id, status FROM microtasks WHERE id = ?", id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| AppError::NotFound { entity: "microtask", id: id.to_string() })?;
    if micro.status == "open" {
        tracing::info!(microtask_id = id, "already open - no-op");
        return Ok(());
    }

    let now = now_iso8601();
    sqlx::query!(
        "UPDATE microtasks SET status = 'open', completed_at = NULL, updated_at = ? WHERE id = ?",
        now, id
    )
    .execute(&mut *tx)
    .await?;
    let mut chain = format!("microtask {id} reopened");

    let task_reopened = sqlx::query!(
        "UPDATE tasks SET status = 'open', completed_at = NULL, updated_at = ?
         WHERE id = ? AND status = 'completed'",
        now, micro.task_id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if task_reopened > 0 {
        chain.push_str(&format!(" -> task {} reopened", micro.task_id));
    }

    let goal_id = sqlx::query!("SELECT goal_id FROM tasks WHERE id = ?", micro.task_id)
        .fetch_one(&mut *tx)
        .await?
        .goal_id;
    let goal_reopened = sqlx::query!(
        "UPDATE goals SET status = 'open', completed_at = NULL, updated_at = ?
         WHERE id = ? AND status = 'completed'",
        now, goal_id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if goal_reopened > 0 {
        chain.push_str(&format!(" -> goal {goal_id} reopened"));
    }

    tx.commit().await?;
    tracing::info!("{chain}");
    Ok(())
}
