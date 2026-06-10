use serde::Serialize;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PomodoroType {
    pub id: String,
    pub name: String,
    pub work_minutes: i64,
    pub rest_minutes: i64,
    pub long_break_minutes: Option<i64>,
    pub long_break_every: Option<i64>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}
