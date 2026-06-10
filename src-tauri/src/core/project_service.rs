use crate::core::time::now_iso8601;
use crate::error::AppError;
use crate::models::project::ProjectSummary;
use crate::models::tree::{ProjectTree, TreeGoal, TreeMicrotask, TreeTask};
use sqlx::SqlitePool;

pub async fn list_projects(pool: &SqlitePool, include_archived: bool) -> Result<Vec<ProjectSummary>, AppError> {
    let projects = sqlx::query_as!(
        ProjectSummary,
        r#"SELECT p.id as "id!: String", p.name as "name!: String", p.description,
                  p.status as "status!: String",
                  p.is_archived as "is_archived!: bool",
                  p.completed_at, p.created_at as "created_at!: String",
                  p.updated_at as "updated_at!: String",
                  (SELECT COUNT(*) FROM microtasks m
                     JOIN tasks t ON m.task_id = t.id
                     JOIN goals g ON t.goal_id = g.id
                    WHERE g.project_id = p.id AND m.is_archived = 0
                  ) as "total_microtasks!: i64",
                  (SELECT COUNT(*) FROM microtasks m
                     JOIN tasks t ON m.task_id = t.id
                     JOIN goals g ON t.goal_id = g.id
                    WHERE g.project_id = p.id AND m.is_archived = 0
                      AND m.status = 'completed'
                  ) as "completed_microtasks!: i64"
           FROM projects p
           WHERE p.is_archived = 0 OR ?1 = 1
           ORDER BY p.created_at"#,
        include_archived
    )
    .fetch_all(pool)
    .await?;
    Ok(projects)
}

pub async fn create_project(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<(), AppError> {
    let name = name.trim();
    if name.is_empty() {
        tracing::warn!(id, "validation: project name must not be empty");
        return Err(AppError::Validation("project name must not be empty".into()));
    }
    let now = now_iso8601();
    sqlx::query!(
        "INSERT INTO projects (id, name, description, status, is_archived, created_at, updated_at)
         VALUES (?, ?, ?, 'open', 0, ?, ?)",
        id, name, description, now, now
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_project(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    description: Option<&str>,
) -> Result<(), AppError> {
    let name = name.trim();
    if name.is_empty() {
        tracing::warn!(id, "validation: project name must not be empty");
        return Err(AppError::Validation("project name must not be empty".into()));
    }
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE projects SET name = ?, description = ?, updated_at = ? WHERE id = ?",
        name, description, now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "project", id: id.to_string() });
    }
    Ok(())
}

pub async fn archive_project(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let now = now_iso8601();
    let result = sqlx::query!(
        "UPDATE projects SET is_archived = 1, updated_at = ? WHERE id = ?",
        now, id
    )
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "project", id: id.to_string() });
    }
    Ok(())
}

pub async fn delete_project(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let result = sqlx::query!("DELETE FROM projects WHERE id = ?", id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound { entity: "project", id: id.to_string() });
    }
    Ok(())
}

pub async fn get_project_tree(pool: &SqlitePool, project_id: &str) -> Result<ProjectTree, AppError> {
    let project = sqlx::query!(
        r#"SELECT id as "id!: String", name, description, status
           FROM projects WHERE id = ?1"#,
        project_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound { entity: "project", id: project_id.to_string() })?;

    let goal_rows = sqlx::query!(
        r#"SELECT id as "id!: String", title, description, deadline, priority, status
           FROM goals WHERE project_id = ?1 AND is_archived = 0 ORDER BY sort_order"#,
        project_id
    )
    .fetch_all(pool)
    .await?;

    let task_rows = sqlx::query!(
        r#"SELECT t.id as "id!: String", t.goal_id as "goal_id!: String",
                  t.title, t.description, t.deadline, t.priority, t.status
           FROM tasks t JOIN goals g ON t.goal_id = g.id
           WHERE g.project_id = ?1 AND t.is_archived = 0 AND g.is_archived = 0
           ORDER BY t.sort_order"#,
        project_id
    )
    .fetch_all(pool)
    .await?;

    let micro_rows = sqlx::query!(
        r#"SELECT m.id as "id!: String", m.task_id as "task_id!: String",
                  m.title, m.estimated_minutes, m.pomodoro_count,
                  m.pomodoro_type_id, m.deadline, m.priority, m.status
           FROM microtasks m
           JOIN tasks t ON m.task_id = t.id
           JOIN goals g ON t.goal_id = g.id
           WHERE g.project_id = ?1 AND m.is_archived = 0
             AND t.is_archived = 0 AND g.is_archived = 0
           ORDER BY m.sort_order"#,
        project_id
    )
    .fetch_all(pool)
    .await?;

    let mut tasks_by_goal: std::collections::HashMap<String, Vec<TreeTask>> =
        std::collections::HashMap::new();
    let mut micros_by_task: std::collections::HashMap<String, Vec<TreeMicrotask>> =
        std::collections::HashMap::new();

    for m in micro_rows {
        micros_by_task.entry(m.task_id).or_default().push(TreeMicrotask {
            id: m.id,
            title: m.title,
            estimated_minutes: m.estimated_minutes,
            pomodoro_count: m.pomodoro_count,
            pomodoro_type_id: m.pomodoro_type_id,
            deadline: m.deadline,
            priority: m.priority,
            status: m.status,
        });
    }
    for t in task_rows {
        let microtasks = micros_by_task.remove(&t.id).unwrap_or_default();
        tasks_by_goal.entry(t.goal_id).or_default().push(TreeTask {
            id: t.id,
            title: t.title,
            description: t.description,
            deadline: t.deadline,
            priority: t.priority,
            status: t.status,
            microtasks,
        });
    }
    let goals = goal_rows
        .into_iter()
        .map(|g| TreeGoal {
            tasks: tasks_by_goal.remove(&g.id).unwrap_or_default(),
            id: g.id,
            title: g.title,
            description: g.description,
            deadline: g.deadline,
            priority: g.priority,
            status: g.status,
        })
        .collect();

    Ok(ProjectTree {
        id: project.id,
        name: project.name,
        description: project.description,
        status: project.status,
        goals,
    })
}
