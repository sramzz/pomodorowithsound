use focus_planner_lib::core::{goal_service, microtask_service, project_service, task_service};
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

async fn seed(pool: &SqlitePool) {
    project_service::create_project(pool, "p1", "P", None).await.unwrap();
    for g in ["g1", "g2", "g3"] {
        goal_service::create_goal(pool, g, "p1", g, None, None, 0).await.unwrap();
    }
    task_service::create_task(pool, "t1", "g1", "t1", None, None, 0).await.unwrap();
    task_service::create_task(pool, "t2", "g1", "t2", None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m1", "t1", "m1", 20, 1, None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m2", "t1", "m2", 20, 1, None, None, 0).await.unwrap();
}

async fn order_of(pool: &SqlitePool, table: &str, parent_col: &str, parent: &str) -> Vec<String> {
    let sql = format!("SELECT id FROM {table} WHERE {parent_col} = ? ORDER BY sort_order");
    sqlx::query_scalar(&sql).bind(parent).fetch_all(pool).await.unwrap()
}

#[sqlx::test]
async fn reorder_goals_rewrites_sort_order_from_the_full_list(pool: SqlitePool) {
    seed(&pool).await;
    goal_service::reorder_goals(
        &pool,
        "p1",
        &["g3".to_string(), "g1".to_string(), "g2".to_string()],
    )
    .await
    .unwrap();
    assert_eq!(order_of(&pool, "goals", "project_id", "p1").await, ["g3", "g1", "g2"]);
}

#[sqlx::test]
async fn reorder_goals_rejects_partial_or_foreign_lists(pool: SqlitePool) {
    seed(&pool).await;
    let err = goal_service::reorder_goals(&pool, "p1", &["g1".to_string(), "g2".to_string()])
        .await
        .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));

    let err = goal_service::reorder_goals(
        &pool,
        "p1",
        &["g1".to_string(), "g2".to_string(), "ghost".to_string()],
    )
    .await
    .unwrap_err();
    assert!(matches!(err, AppError::Validation(_)));

    // a rejected reorder must leave the original order intact (transaction rolled back)
    assert_eq!(order_of(&pool, "goals", "project_id", "p1").await, ["g1", "g2", "g3"]);
}

#[sqlx::test]
async fn reorder_goals_ignores_archived_children(pool: SqlitePool) {
    seed(&pool).await;
    goal_service::archive_goal(&pool, "g2").await.unwrap();
    // the visible tree shows g1, g3 only — that full list must be accepted
    goal_service::reorder_goals(&pool, "p1", &["g3".to_string(), "g1".to_string()])
        .await
        .unwrap();
    let visible: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM goals WHERE project_id = 'p1' AND is_archived = 0 ORDER BY sort_order",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(visible, ["g3", "g1"]);
}

#[sqlx::test]
async fn reorder_tasks_and_microtasks_work_the_same_way(pool: SqlitePool) {
    seed(&pool).await;
    task_service::reorder_tasks(&pool, "g1", &["t2".to_string(), "t1".to_string()])
        .await
        .unwrap();
    assert_eq!(order_of(&pool, "tasks", "goal_id", "g1").await, ["t2", "t1"]);

    microtask_service::reorder_microtasks(&pool, "t1", &["m2".to_string(), "m1".to_string()])
        .await
        .unwrap();
    assert_eq!(order_of(&pool, "microtasks", "task_id", "t1").await, ["m2", "m1"]);
}
