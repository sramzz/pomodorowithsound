use focus_planner_lib::core::{goal_service, project_service};
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

async fn seed_project(pool: &SqlitePool) {
    project_service::create_project(pool, "p1", "P", None).await.unwrap();
}

#[sqlx::test]
async fn create_goal_appends_sort_order(pool: SqlitePool) {
    seed_project(&pool).await;
    goal_service::create_goal(&pool, "g1", "p1", "First", None, None, 0).await.unwrap();
    goal_service::create_goal(&pool, "g2", "p1", "Second", Some("desc"), Some("2026-07-01T00:00:00Z"), 2)
        .await
        .unwrap();

    let rows = sqlx::query!(
        "SELECT id, sort_order, priority, deadline FROM goals ORDER BY sort_order"
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!((rows[0].id.as_deref(), rows[0].sort_order), (Some("g1"), 0));
    assert_eq!((rows[1].id.as_deref(), rows[1].sort_order), (Some("g2"), 1));
    assert_eq!(rows[1].priority, 2);
    assert_eq!(rows[1].deadline.as_deref(), Some("2026-07-01T00:00:00Z"));
}

#[sqlx::test]
async fn create_goal_unknown_project_is_not_found(pool: SqlitePool) {
    let err = goal_service::create_goal(&pool, "g1", "ghost", "G", None, None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { entity: "project", .. }));
}

#[sqlx::test]
async fn create_goal_rejects_blank_title(pool: SqlitePool) {
    seed_project(&pool).await;
    let err = goal_service::create_goal(&pool, "g1", "p1", "  ", None, None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));
}

#[sqlx::test]
async fn update_goal_sets_absolute_values(pool: SqlitePool) {
    seed_project(&pool).await;
    goal_service::create_goal(&pool, "g1", "p1", "Old", Some("d"), Some("2026-07-01T00:00:00Z"), 1)
        .await
        .unwrap();
    goal_service::update_goal(&pool, "g1", "New", None, None, 5).await.unwrap();

    let row = sqlx::query!("SELECT title, description, deadline, priority FROM goals WHERE id = 'g1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.title, "New");
    assert!(row.description.is_none());
    assert!(row.deadline.is_none());
    assert_eq!(row.priority, 5);
}

#[sqlx::test]
async fn update_goal_unknown_id_is_not_found(pool: SqlitePool) {
    let err = goal_service::update_goal(&pool, "ghost", "X", None, None, 0)
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}

#[sqlx::test]
async fn archive_and_delete_goal(pool: SqlitePool) {
    seed_project(&pool).await;
    goal_service::create_goal(&pool, "g1", "p1", "G", None, None, 0).await.unwrap();

    goal_service::archive_goal(&pool, "g1").await.unwrap();
    let row = sqlx::query!("SELECT is_archived FROM goals WHERE id = 'g1'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.is_archived, 1);

    goal_service::delete_goal(&pool, "g1").await.unwrap();
    let cnt = sqlx::query!(r#"SELECT COUNT(*) as "cnt: i64" FROM goals"#)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(cnt.cnt, 0);

    let err = goal_service::delete_goal(&pool, "g1").await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { .. }));
}
