use focus_planner_lib::core::{goal_service, microtask_service, project_service, task_service};
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

async fn seed_task(pool: &SqlitePool) {
    project_service::create_project(pool, "p1", "P", None).await.unwrap();
    goal_service::create_goal(pool, "g1", "p1", "G", None, None, 0).await.unwrap();
    task_service::create_task(pool, "t1", "g1", "T", None, None, 0).await.unwrap();
}

#[sqlx::test]
async fn create_microtask_appends_sort_order_and_stores_given_count(pool: SqlitePool) {
    seed_task(&pool).await;
    microtask_service::create_microtask(&pool, "m1", "t1", "Outline", 50, 3, None, None, 0)
        .await
        .unwrap();
    microtask_service::create_microtask(&pool, "m2", "t1", "Draft", 20, 1, None, None, 0)
        .await
        .unwrap();

    let rows = sqlx::query!(
        "SELECT id, estimated_minutes, pomodoro_count, sort_order, status FROM microtasks ORDER BY sort_order"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id.as_deref(), Some("m1"));
    assert_eq!(rows[0].estimated_minutes, 50);
    assert_eq!(rows[0].pomodoro_count, 3);
    assert_eq!(rows[0].sort_order, 0);
    assert_eq!(rows[0].status, "open");
    assert_eq!(rows[1].sort_order, 1);
}

#[sqlx::test]
async fn create_microtask_accepts_the_seeded_pomodoro_type(pool: SqlitePool) {
    seed_task(&pool).await;
    // 'a0000000-0000-4000-8000-000000000001' is the Standard type seeded by migration 0002
    microtask_service::create_microtask(
        &pool, "m1", "t1", "Outline", 40, 2,
        Some("a0000000-0000-4000-8000-000000000001"), None, 0,
    )
    .await
    .unwrap();

    let row = sqlx::query!("SELECT pomodoro_type_id FROM microtasks WHERE id = 'm1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.pomodoro_type_id.as_deref(), Some("a0000000-0000-4000-8000-000000000001"));
}

#[sqlx::test]
async fn create_microtask_unknown_type_is_not_found(pool: SqlitePool) {
    seed_task(&pool).await;
    let err = microtask_service::create_microtask(&pool, "m1", "t1", "X", 20, 1, Some("ghost"), None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { entity: "pomodoro_type", .. }));
}

#[sqlx::test]
async fn create_microtask_rejects_nonpositive_estimate_and_count(pool: SqlitePool) {
    seed_task(&pool).await;
    let err = microtask_service::create_microtask(&pool, "m1", "t1", "X", 0, 1, None, None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));

    let err = microtask_service::create_microtask(&pool, "m1", "t1", "X", 20, 0, None, None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn create_microtask_rejects_completed_task(pool: SqlitePool) {
    seed_task(&pool).await;
    sqlx::query("UPDATE tasks SET status = 'completed' WHERE id = 't1'")
        .execute(&pool)
        .await
        .unwrap();

    let err = microtask_service::create_microtask(
        &pool, "m1", "t1", "M", 20, 1, None, None, 0,
    )
    .await
    .unwrap_err();

    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn create_microtask_rejects_archived_task(pool: SqlitePool) {
    seed_task(&pool).await;
    task_service::archive_task(&pool, "t1").await.unwrap();

    let err = microtask_service::create_microtask(
        &pool, "m1", "t1", "M", 20, 1, None, None, 0,
    )
    .await
    .unwrap_err();

    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn update_archive_delete_microtask(pool: SqlitePool) {
    seed_task(&pool).await;
    microtask_service::create_microtask(&pool, "m1", "t1", "Old", 20, 1, None, None, 0)
        .await
        .unwrap();

    microtask_service::update_microtask(&pool, "m1", "New", 60, 3, None, Some("2026-08-01T00:00:00Z"), 2)
        .await
        .unwrap();
    let row = sqlx::query!(
        "SELECT title, estimated_minutes, pomodoro_count, pomodoro_type_id, deadline, priority
         FROM microtasks WHERE id = 'm1'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.title, "New");
    assert_eq!(row.estimated_minutes, 60);
    assert_eq!(row.pomodoro_count, 3);
    assert!(row.pomodoro_type_id.is_none());
    assert_eq!(row.deadline.as_deref(), Some("2026-08-01T00:00:00Z"));
    assert_eq!(row.priority, 2);

    microtask_service::archive_microtask(&pool, "m1").await.unwrap();
    let row = sqlx::query!("SELECT is_archived FROM microtasks WHERE id = 'm1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.is_archived, 1);

    microtask_service::delete_microtask(&pool, "m1").await.unwrap();
    let err = microtask_service::delete_microtask(&pool, "m1").await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}

#[sqlx::test]
async fn get_microtask_returns_the_row_or_not_found(pool: SqlitePool) {
    seed_task(&pool).await;
    microtask_service::create_microtask(&pool, "m1", "t1", "Read", 40, 2, None, None, 0)
        .await
        .unwrap();

    let m = microtask_service::get_microtask(&pool, "m1").await.unwrap();
    assert_eq!(m.title, "Read");
    assert_eq!(m.estimated_minutes, 40);
    assert_eq!(m.pomodoro_count, 2);

    let err = microtask_service::get_microtask(&pool, "ghost").await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { entity: "microtask", .. }));
}
