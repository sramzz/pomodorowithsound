use focus_planner_lib::core::{goal_service, project_service, task_service};
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

async fn seed_goal(pool: &SqlitePool) {
    project_service::create_project(pool, "p1", "P", None).await.unwrap();
    goal_service::create_goal(pool, "g1", "p1", "G", None, None, 0).await.unwrap();
}

#[sqlx::test]
async fn create_task_appends_sort_order_within_goal(pool: SqlitePool) {
    seed_goal(&pool).await;
    task_service::create_task(&pool, "t1", "g1", "First", None, None, 0).await.unwrap();
    task_service::create_task(&pool, "t2", "g1", "Second", None, None, 0).await.unwrap();

    let rows = sqlx::query!("SELECT id, sort_order FROM tasks ORDER BY sort_order")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!((rows[0].id.as_deref(), rows[0].sort_order), (Some("t1"), 0));
    assert_eq!((rows[1].id.as_deref(), rows[1].sort_order), (Some("t2"), 1));
}

#[sqlx::test]
async fn create_task_unknown_goal_is_not_found(pool: SqlitePool) {
    let err = task_service::create_task(&pool, "t1", "ghost", "T", None, None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { entity: "goal", .. }));
}

#[sqlx::test]
async fn create_task_rejects_blank_title(pool: SqlitePool) {
    seed_goal(&pool).await;
    let err = task_service::create_task(&pool, "t1", "g1", " ", None, None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn create_task_rejects_completed_goal(pool: SqlitePool) {
    seed_goal(&pool).await;
    sqlx::query("UPDATE goals SET status = 'completed' WHERE id = 'g1'")
        .execute(&pool)
        .await
        .unwrap();

    let err = task_service::create_task(&pool, "t1", "g1", "T", None, None, 0)
        .await
        .unwrap_err();

    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn create_task_rejects_archived_goal(pool: SqlitePool) {
    seed_goal(&pool).await;
    goal_service::archive_goal(&pool, "g1").await.unwrap();

    let err = task_service::create_task(&pool, "t1", "g1", "T", None, None, 0)
        .await
        .unwrap_err();

    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn update_archive_delete_task(pool: SqlitePool) {
    seed_goal(&pool).await;
    task_service::create_task(&pool, "t1", "g1", "Old", Some("d"), None, 1).await.unwrap();

    task_service::update_task(&pool, "t1", "New", None, Some("2026-08-01T00:00:00Z"), 3)
        .await
        .unwrap();
    let row = sqlx::query!("SELECT title, description, deadline, priority FROM tasks WHERE id = 't1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.title, "New");
    assert!(row.description.is_none());
    assert_eq!(row.deadline.as_deref(), Some("2026-08-01T00:00:00Z"));
    assert_eq!(row.priority, 3);

    task_service::archive_task(&pool, "t1").await.unwrap();
    let row = sqlx::query!("SELECT is_archived FROM tasks WHERE id = 't1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.is_archived, 1);

    task_service::delete_task(&pool, "t1").await.unwrap();
    let err = task_service::delete_task(&pool, "t1").await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}
