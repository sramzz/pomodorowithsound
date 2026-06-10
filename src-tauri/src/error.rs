use serde::Serialize;

// NotFound/Validation variants are used from Phase 2 onward; allow until then.
#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("{entity} not found: {id}")]
    NotFound { entity: &'static str, id: String },
    #[error("validation failed: {0}")]
    Validation(String),
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Wire<'a> {
            code: &'a str,
            message: String,
        }
        let code = match self {
            AppError::Db(_) => "db",
            AppError::NotFound { .. } => "not_found",
            AppError::Validation(_) => "validation",
        };
        Wire { code, message: self.to_string() }.serialize(serializer)
    }
}
