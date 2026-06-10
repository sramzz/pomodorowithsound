pub mod frontend_log;
pub mod goal;
pub mod microtask;
pub mod project;
pub mod task;

use crate::error::AppError;

/// Uniform outcome logging for every IPC command (spec §7):
/// INFO `ok` · WARN validation rejections · ERROR everything else.
/// Runs inside the command's `#[tracing::instrument]` span, so the
/// command name and key params are already on the line.
pub(crate) fn log_outcome<T>(result: &Result<T, AppError>) {
    match result {
        Ok(_) => tracing::info!("ok"),
        Err(AppError::Validation(msg)) => tracing::warn!(reason = %msg, "rejected"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
}
