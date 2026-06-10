use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectTree {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub goals: Vec<TreeGoal>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeGoal {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub deadline: Option<String>,
    pub priority: i64,
    pub status: String,
    pub tasks: Vec<TreeTask>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeTask {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub deadline: Option<String>,
    pub priority: i64,
    pub status: String,
    pub microtasks: Vec<TreeMicrotask>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeMicrotask {
    pub id: String,
    pub title: String,
    pub estimated_minutes: i64,
    pub pomodoro_count: i64,
    pub pomodoro_type_id: Option<String>,
    pub deadline: Option<String>,
    pub priority: i64,
    pub status: String,
}
