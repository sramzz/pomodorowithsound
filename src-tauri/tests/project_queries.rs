use focus_planner_lib::core::{goal_service, microtask_service, project_service, task_service};
use sqlx::SqlitePool;

async fn seed_tree(pool: &SqlitePool) {
    project_service::create_project(pool, "p1", "P", None).await.unwrap();
    goal_service::create_goal(pool, "g1", "p1", "G", None, None, 0).await.unwrap();
    task_service::create_task(pool, "t1", "g1", "T", None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m1", "t1", "M1", 20, 1, None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m2", "t1", "M2", 20, 1, None, None, 0).await.unwrap();
    microtask_service::create_microtask(pool, "m3", "t1", "M3", 20, 1, None, None, 0).await.unwrap();
}

#[sqlx::test]
async fn list_projects_counts_completed_and_total_microtasks(pool: SqlitePool) {
    seed_tree(&pool).await;
    microtask_service::complete_microtask(&pool, "m1").await.unwrap();

    let projects = project_service::list_projects(&pool, false).await.unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].total_microtasks, 3);
    assert_eq!(projects[0].completed_microtasks, 1);
}

#[sqlx::test]
async fn archived_microtasks_are_excluded_from_the_stats(pool: SqlitePool) {
    seed_tree(&pool).await;
    microtask_service::archive_microtask(&pool, "m3").await.unwrap();

    let projects = project_service::list_projects(&pool, false).await.unwrap();
    assert_eq!(projects[0].total_microtasks, 2);
}

#[sqlx::test]
async fn empty_project_has_zero_stats(pool: SqlitePool) {
    project_service::create_project(&pool, "p1", "Empty", None).await.unwrap();
    let projects = project_service::list_projects(&pool, false).await.unwrap();
    assert_eq!(projects[0].total_microtasks, 0);
    assert_eq!(projects[0].completed_microtasks, 0);
}
