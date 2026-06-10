use focus_planner_lib::core::project_service;
use sqlx::SqlitePool;

#[sqlx::test]
async fn list_projects_on_empty_db_returns_empty_vec(pool: SqlitePool) {
    let projects = project_service::list_projects(&pool, false).await.unwrap();
    assert!(projects.is_empty());
}

#[sqlx::test]
async fn list_projects_excludes_archived_unless_asked(pool: SqlitePool) {
    sqlx::query("INSERT INTO projects (id, name, status, is_archived, created_at, updated_at) VALUES ('p1', 'Active', 'open', 0, '2026-06-09T08:00:00Z', '2026-06-09T08:00:00Z'), ('p2', 'Archived', 'open', 1, '2026-06-09T08:00:00Z', '2026-06-09T08:00:00Z')")
        .execute(&pool).await.unwrap();

    let visible = project_service::list_projects(&pool, false).await.unwrap();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].name, "Active");

    let all = project_service::list_projects(&pool, true).await.unwrap();
    assert_eq!(all.len(), 2);
}
