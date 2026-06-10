use focus_planner_lib::core::pomodoro_type_service;
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

const SEEDED_STANDARD: &str = "a0000000-0000-4000-8000-000000000001";

#[sqlx::test]
async fn create_and_list_pomodoro_types(pool: SqlitePool) {
    pomodoro_type_service::create_pomodoro_type(&pool, "pt1", "Deep Work", 50, 10, Some(30), Some(2))
        .await
        .unwrap();

    let types = pomodoro_type_service::list_pomodoro_types(&pool).await.unwrap();
    // the seed migration already provides "Standard"
    assert_eq!(types.len(), 2);
    let deep = types.iter().find(|t| t.id == "pt1").unwrap();
    assert_eq!(deep.name, "Deep Work");
    assert_eq!(deep.work_minutes, 50);
    assert_eq!(deep.rest_minutes, 10);
    assert_eq!(deep.long_break_minutes, Some(30));
    assert_eq!(deep.long_break_every, Some(2));
    assert!(!deep.is_default, "new types are never default");
}

#[sqlx::test]
async fn create_rejects_bad_values(pool: SqlitePool) {
    let err = pomodoro_type_service::create_pomodoro_type(&pool, "x", " ", 20, 5, None, None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));

    let err = pomodoro_type_service::create_pomodoro_type(&pool, "x", "T", 0, 5, None, None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));

    // long break fields must come as a pair
    let err = pomodoro_type_service::create_pomodoro_type(&pool, "x", "T", 20, 5, Some(15), None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn update_pomodoro_type_sets_absolute_values(pool: SqlitePool) {
    pomodoro_type_service::create_pomodoro_type(&pool, "pt1", "Old", 25, 5, Some(20), Some(4))
        .await
        .unwrap();
    pomodoro_type_service::update_pomodoro_type(&pool, "pt1", "New", 45, 15, None, None)
        .await
        .unwrap();

    let row = sqlx::query!(
        "SELECT name, work_minutes, rest_minutes, long_break_minutes, long_break_every
         FROM pomodoro_types WHERE id = 'pt1'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.name, "New");
    assert_eq!(row.work_minutes, 45);
    assert_eq!(row.rest_minutes, 15);
    assert!(row.long_break_minutes.is_none());
    assert!(row.long_break_every.is_none());

    let err = pomodoro_type_service::update_pomodoro_type(&pool, "ghost", "X", 20, 5, None, None)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}

#[sqlx::test]
async fn set_default_is_exclusive(pool: SqlitePool) {
    pomodoro_type_service::create_pomodoro_type(&pool, "pt1", "Deep", 50, 10, None, None)
        .await
        .unwrap();
    pomodoro_type_service::set_default_pomodoro_type(&pool, "pt1").await.unwrap();

    let defaults: Vec<String> =
        sqlx::query_scalar("SELECT id FROM pomodoro_types WHERE is_default = 1")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(defaults, ["pt1"], "exactly one default; the seeded Standard lost the flag");

    let err = pomodoro_type_service::set_default_pomodoro_type(&pool, "ghost")
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
    // a failed set_default must not have cleared the existing default (transaction)
    let defaults: Vec<String> =
        sqlx::query_scalar("SELECT id FROM pomodoro_types WHERE is_default = 1")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(defaults, ["pt1"]);
}

#[sqlx::test]
async fn delete_pomodoro_type_nulls_microtask_references(pool: SqlitePool) {
    use focus_planner_lib::core::{goal_service, microtask_service, project_service, task_service};
    project_service::create_project(&pool, "p1", "P", None).await.unwrap();
    goal_service::create_goal(&pool, "g1", "p1", "G", None, None, 0).await.unwrap();
    task_service::create_task(&pool, "t1", "g1", "T", None, None, 0).await.unwrap();
    microtask_service::create_microtask(&pool, "m1", "t1", "M", 20, 1, Some(SEEDED_STANDARD), None, 0)
        .await
        .unwrap();

    pomodoro_type_service::delete_pomodoro_type(&pool, SEEDED_STANDARD).await.unwrap();

    let row = sqlx::query!("SELECT pomodoro_type_id FROM microtasks WHERE id = 'm1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(row.pomodoro_type_id.is_none(), "ON DELETE SET NULL must apply");

    let err = pomodoro_type_service::delete_pomodoro_type(&pool, SEEDED_STANDARD)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}
