use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Microtask {
    pub id: String,
    pub task_id: String,
    pub title: String,
    pub estimated_minutes: i64,
    pub pomodoro_count: i64,
    pub pomodoro_type_id: Option<String>,
    pub deadline: Option<String>,
    pub priority: i64,
    pub sort_order: i64,
    pub status: String,
    pub is_archived: bool,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
