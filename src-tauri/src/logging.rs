use tauri::Manager;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub struct LogGuard(pub WorkerGuard);

fn logs_dir(app: &tauri::AppHandle) -> std::path::PathBuf {
    if cfg!(debug_assertions) {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../logs")
    } else {
        app.path().app_log_dir().expect("no app log dir")
    }
}

/// Console layer (pretty, for the dev terminal) + daily-rolling plain-text file
/// layer in the exposed Logs folder. Level via RUST_LOG; defaults: debug (dev),
/// info (release).
pub fn init(app: &tauri::AppHandle) -> LogGuard {
    let dir = logs_dir(app);
    std::fs::create_dir_all(&dir).ok();
    let file_appender = tracing_appender::rolling::daily(&dir, "focus-planner.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let default_level = if cfg!(debug_assertions) { "debug" } else { "info" };
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_writer(file_writer).with_ansi(false))
        .init();

    tracing::info!(logs_dir = %dir.display(), "logging initialized");
    LogGuard(guard)
}
