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

#[sqlx::test]
async fn get_project_tree_nests_all_levels_in_sort_order(pool: SqlitePool) {
    seed_tree(&pool).await;
    goal_service::create_goal(&pool, "g2", "p1", "G2", None, None, 0).await.unwrap();
    goal_service::reorder_goals(&pool, "p1", &["g2".to_string(), "g1".to_string()])
        .await
        .unwrap();

    let tree = project_service::get_project_tree(&pool, "p1").await.unwrap();
    assert_eq!(tree.id, "p1");
    assert_eq!(tree.goals.len(), 2);
    assert_eq!(tree.goals[0].id, "g2", "goals come back in sort order");
    assert_eq!(tree.goals[1].tasks.len(), 1);
    assert_eq!(tree.goals[1].tasks[0].microtasks.len(), 3);
    assert_eq!(tree.goals[1].tasks[0].microtasks[0].id, "m1");
}

#[sqlx::test]
async fn get_project_tree_excludes_archived_rows(pool: SqlitePool) {
    seed_tree(&pool).await;
    microtask_service::archive_microtask(&pool, "m2").await.unwrap();

    let tree = project_service::get_project_tree(&pool, "p1").await.unwrap();
    let micro_ids: Vec<&str> = tree.goals[0].tasks[0]
        .microtasks
        .iter()
        .map(|m| m.id.as_str())
        .collect();
    assert_eq!(micro_ids, ["m1", "m3"]);
}

#[sqlx::test]
async fn get_project_tree_unknown_project_is_not_found(pool: SqlitePool) {
    let err = project_service::get_project_tree(&pool, "ghost").await.unwrap_err();
    assert!(matches!(
        err,
        focus_planner_lib::error::AppError::NotFound { entity: "project", .. }
    ));
}
