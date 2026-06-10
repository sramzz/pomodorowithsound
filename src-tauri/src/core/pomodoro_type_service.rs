use crate::core::time::now_iso8601;
use crate::error::AppError;
use crate::models::pomodoro_type::PomodoroType;
use sqlx::SqlitePool;

fn validate_type_fields(
    name: &str,
    work_minutes: i64,
    rest_minutes: i64,
    long_break_minutes: Option<i64>,
    long_break_every: Option<i64>,
) -> Result<(), AppError> {
    if name.trim().is_empty() {
        tracing::warn!("validation: pomodoro type name must not be empty");
        return Err(AppError::Validation("pomodoro type name must not be empty".into()));
    }
    if work_minutes < 1 || rest_minutes < 1 {
        tracing::warn!(work_minutes, rest_minutes, "validation: minutes must be >= 1");
        return Err(AppError::Validation("work_minutes and rest_minutes must be >= 1".into()));
    }
    match (long_break_minutes, long_break_every) {
        (None, None) => Ok(()),
        (Some(m), Some(e)) if m >= 1 && e >= 1 => Ok(()),
        _ => {
            tracing::warn!(
                ?long_break_minutes,
                ?long_break_every,
                "validation: long break fields must come as a pair, each >= 1"
            );
            Err(AppError::Validation(
                "long_break_minutes and long_break_every must both be set (each >= 1) or both be empty".into(),
            ))
        }
    }
}

pub async fn list_pomodoro_types(pool: &SqlitePool) -> Result<Vec<PomodoroType>, AppError> {
    let types = sqlx::query_as!(
        PomodoroType,
        r#"SELECT id as "id!: String", name as "name!: String",
                  work_minutes as "work_minutes!: i64", rest_minutes as "rest_minutes!: i64",
                  long_break_minutes as "long_break_minutes?: i64",
                  long_break_every as "long_break_every?: i64",
                  is_default as "is_default!: bool",
                  created_at as "created_at!: String", updated_at as "updated_at!: String"
           FROM pomodoro_types ORDER BY created_at"#
    )
    .fetch_all(pool)
    .await?;
    Ok(types)
}

pub async fn create_pomodoro_type(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    work_minutes: i64,
    rest_minutes: i64,
    long_break_minutes: Option<i64>,
    long_break_every: Option<i64>,
) -> Result<(), AppError> {
    validate_type_fields(name, work_minutes, rest_minutes, long_break_minutes, long_break_every)?;
    let name = name.trim();
    let now = now_iso8601();
    sqlx::query!(
        "INSERT INTO pomodoro_types (id, name, work_minutes, rest_minutes, long_break_minutes,
                                     long_break_every, is_default, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?)",
        id, name, work_minutes, rest_minutes, long_break_minutes, long_break_every, now, now
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_pomodoro_type(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    work_minutes: i64,
    rest_minutes: i64,
    long_break_minutes: Option<i64>,
    long_break_every: Option<i64>,
) -> Result<(), AppError> {
    validate_type_fields(name, work_minutes, rest_minutes, long_break_minutes, long_break_every)?;
    let name = name.trim();
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE pomodoro_types SET name = ?, work_minutes = ?, rest_minutes = ?,
                                   long_break_minutes = ?, long_break_every = ?, updated_at = ?
         WHERE id = ?",
        name, work_minutes, rest_minutes, long_break_minutes, long_break_every, now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "pomodoro_type", id: id.to_string() });
    }
    Ok(())
}

pub async fn delete_pomodoro_type(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let result = sqlx::query!("DELETE FROM pomodoro_types WHERE id = ?", id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "pomodoro_type", id: id.to_string() });
    }
    tracing::info!(id, "pomodoro type deleted; microtasks referencing it fell back to NULL (default type applies)");
    Ok(())
}

/// One transaction: clear every default flag, then set the new one.
/// Rolls back (keeping the old default) when the id is unknown.
pub async fn set_default_pomodoro_type(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;
    let now = now_iso8601();
    sqlx::query!(
        "UPDATE pomodoro_types SET is_default = 0, updated_at = ? WHERE is_default = 1",
        now
    )
    .execute(&mut *tx)
    .await?;
    let result = sqlx::query!(
        "UPDATE pomodoro_types SET is_default = 1, updated_at = ? WHERE id = ?",
        now, id
    )
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "pomodoro_type", id: id.to_string() });
        // tx drops here -> rollback, the old default survives
    }
    tx.commit().await?;
    Ok(())
}
