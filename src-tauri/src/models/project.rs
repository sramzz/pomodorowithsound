use serde::Serialize;

#[derive(Debug, Serialize, sqlx::FromRow)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub is_archived: bool,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// `list_projects` row: project columns + completion roll-up stats (spec §5).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub is_archived: bool,
    pub completed_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub total_microtasks: i64,
    pub completed_microtasks: i64,
}
