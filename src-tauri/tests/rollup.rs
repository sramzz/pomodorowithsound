use focus_planner_lib::core::{goal_service, microtask_service, project_service, task_service};
use focus_planner_lib::error::AppError;
use sqlx::SqlitePool;

/// p1 -> g1 -> t1 (m1, m2) + t2 (m3)
async fn seed_tree(pool: &SqlitePool) {
    project_service::create_project(pool, "p1", "P", None).await.unwrap();
    goal_service::create_goal(pool, "g1", "p1", "G", None, None, 0).await.unwrap();
    task_service::create_task(pool, "t1", "g1", "T1", None, None, 0).await.unwrap();
    task_service::create_task(pool, "t2", "g1", "T2", None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m1", "t1", "M1", 20, 1, None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m2", "t1", "M2", 20, 1, None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m3", "t2", "M3", 20, 1, None, None, 0).await.unwrap();
}

async fn status_of(pool: &SqlitePool, table: &str, id: &str) -> (String, Option<String>) {
    let sql = format!("SELECT status, completed_at FROM {table} WHERE id = ?");
    let row: (String, Option<String>) = sqlx::query_as(&sql).bind(id).fetch_one(pool).await.unwrap();
    row
}

#[sqlx::test]
async fn completing_a_non_last_microtask_does_not_touch_the_task(pool: SqlitePool) {
    seed_tree(&pool).await;
    microtask_service::complete_microtask(&pool, "m1").await.unwrap();

    let (m1_status, m1_completed) = status_of(&pool, "microtasks", "m1").await;
    assert_eq!(m1_status, "completed");
    assert!(m1_completed.is_some());
    assert_eq!(status_of(&pool, "tasks", "t1").await.0, "open");
    assert_eq!(status_of(&pool, "goals", "g1").await.0, "open");
}

#[sqlx::test]
async fn completing_the_last_microtask_completes_the_task_but_not_the_goal_with_open_siblings(
    pool: SqlitePool,
) {
    seed_tree(&pool).await;
    microtask_service::complete_microtask(&pool, "m1").await.unwrap();
    microtask_service::complete_microtask(&pool, "m2").await.unwrap();

    assert_eq!(status_of(&pool, "tasks", "t1").await.0, "completed");
    assert!(status_of(&pool, "tasks", "t1").await.1.is_some());
    assert_eq!(status_of(&pool, "goals", "g1").await.0, "open", "t2 is still open");
}

#[sqlx::test]
async fn completing_the_last_task_completes_the_goal_but_never_the_project(pool: SqlitePool) {
    seed_tree(&pool).await;
    microtask_service::complete_microtask(&pool, "m1").await.unwrap();
    microtask_service::complete_microtask(&pool, "m2").await.unwrap();
    microtask_service::complete_microtask(&pool, "m3").await.unwrap();

    assert_eq!(status_of(&pool, "tasks", "t2").await.0, "completed");
    assert_eq!(status_of(&pool, "goals", "g1").await.0, "completed");
    assert_eq!(status_of(&pool, "projects", "p1").await.0, "open", "roll-up stops at the goal");
}

#[sqlx::test]
async fn archived_open_microtasks_do_not_block_task_completion(pool: SqlitePool) {
    seed_tree(&pool).await;
    microtask_service::archive_microtask(&pool, "m2").await.unwrap();
    microtask_service::complete_microtask(&pool, "m1").await.unwrap();

    assert_eq!(status_of(&pool, "tasks", "t1").await.0, "completed");
}

#[sqlx::test]
async fn uncomplete_reverses_the_full_chain(pool: SqlitePool) {
    seed_tree(&pool).await;
    for m in ["m1", "m2", "m3"] {
        microtask_service::complete_microtask(&pool, m).await.unwrap();
    }
    assert_eq!(status_of(&pool, "goals", "g1").await.0, "completed");

    microtask_service::uncomplete_microtask(&pool, "m3").await.unwrap();

    let (m3_status, m3_completed) = status_of(&pool, "microtasks", "m3").await;
    assert_eq!(m3_status, "open");
    assert!(m3_completed.is_none());
    assert_eq!(status_of(&pool, "tasks", "t2").await.0, "open");
    assert!(status_of(&pool, "tasks", "t2").await.1.is_none());
    assert_eq!(status_of(&pool, "goals", "g1").await.0, "open");
    // t1 keeps its own completion — only the ancestors of m3 reopen
    assert_eq!(status_of(&pool, "tasks", "t1").await.0, "completed");
}

#[sqlx::test]
async fn complete_is_idempotent_and_unknown_id_is_not_found(pool: SqlitePool) {
    seed_tree(&pool).await;
    microtask_service::complete_microtask(&pool, "m1").await.unwrap();
    microtask_service::complete_microtask(&pool, "m1").await.unwrap(); // no-op, still Ok

    let err = microtask_service::complete_microtask(&pool, "ghost").await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { entity: "microtask", .. }));

    microtask_service::uncomplete_microtask(&pool, "m2").await.unwrap(); // already open: no-op Ok
    let err = microtask_service::uncomplete_microtask(&pool, "ghost").await.unwrap_err();
    assert!(matches!(err, AppError::NotFound { entity: "microtask", .. }));
}
