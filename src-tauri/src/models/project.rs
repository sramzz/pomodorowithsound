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
