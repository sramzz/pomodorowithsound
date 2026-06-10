#[tauri::command]
pub fn log_frontend(level: String, message: String, context: Option<String>) {
    let ctx = context.as_deref().unwrap_or("");
    match level.as_str() {
        "error" => tracing::error!(target: "frontend", context = ctx, "{message}"),
        "warn" => tracing::warn!(target: "frontend", context = ctx, "{message}"),
        _ => tracing::info!(target: "frontend", context = ctx, "{message}"),
    }
}
