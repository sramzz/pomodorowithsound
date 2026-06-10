use focus_planner_lib::core::project_service;
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

#[sqlx::test]
async fn create_project_inserts_open_unarchived_row_with_timestamps(pool: SqlitePool) {
    project_service::create_project(&pool, "p1", "Write the book", Some("a novel"))
        .await
        .unwrap();

    let row = sqlx::query!(
        r#"SELECT name, description, status, is_archived, completed_at, created_at, updated_at
           FROM projects WHERE id = 'p1'"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.name, "Write the book");
    assert_eq!(row.description.as_deref(), Some("a novel"));
    assert_eq!(row.status, "open");
    assert_eq!(row.is_archived, 0);
    assert!(row.completed_at.is_none());
    assert_eq!(row.created_at, row.updated_at);
    assert!(row.created_at.ends_with('Z'));
}

#[sqlx::test]
async fn create_project_rejects_blank_name(pool: SqlitePool) {
    let err = project_service::create_project(&pool, "p1", "   ", None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn update_project_sets_absolute_values_and_bumps_updated_at(pool: SqlitePool) {
    project_service::create_project(&pool, "p1", "Old", Some("old desc"))
        .await
        .unwrap();
    // description = None is an absolute write: it clears the column
    project_service::update_project(&pool, "p1", "New", None)
        .await
        .unwrap();

    let row = sqlx::query!("SELECT name, description FROM projects WHERE id = 'p1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.name, "New");
    assert!(row.description.is_none());
}

#[sqlx::test]
async fn update_project_unknown_id_is_not_found(pool: SqlitePool) {
    let err = project_service::update_project(&pool, "ghost", "X", None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}

#[sqlx::test]
async fn archive_project_sets_flag(pool: SqlitePool) {
    project_service::create_project(&pool, "p1", "P", None).await.unwrap();
    project_service::archive_project(&pool, "p1").await.unwrap();
    let row = sqlx::query!("SELECT is_archived FROM projects WHERE id = 'p1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.is_archived, 1);
}

#[sqlx::test]
async fn delete_project_cascades_to_goals(pool: SqlitePool) {
    project_service::create_project(&pool, "p1", "P", None).await.unwrap();
    sqlx::query(
        "INSERT INTO goals (id, project_id, title, priority, sort_order, status, is_archived, created_at, updated_at)
         VALUES ('g1', 'p1', 'G', 0, 0, 'open', 0, '2026-06-09T08:00:00Z', '2026-06-09T08:00:00Z')",
    )
    .execute(&pool)
    .await
    .unwrap();

    project_service::delete_project(&pool, "p1").await.unwrap();

    let goals = sqlx::query!(r#"SELECT COUNT(*) as "cnt: i64" FROM goals"#)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(goals.cnt, 0, "goal must be cascade-deleted");
}

#[sqlx::test]
async fn delete_project_unknown_id_is_not_found(pool: SqlitePool) {
    let err = project_service::delete_project(&pool, "ghost").await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}
