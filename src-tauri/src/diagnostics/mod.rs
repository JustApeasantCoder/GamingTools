use std::path::Path;

use dee_bugee_rust::{non_blocking_layer, LoggerConfig, LoggerGuard};
use tracing_subscriber::{filter::filter_fn, prelude::*};

pub fn initialize(directory: &Path) -> Result<LoggerGuard, Box<dyn std::error::Error>> {
    let mut config = LoggerConfig::new(directory.join("GamingToolkit.jsonl"), "backend");
    config.archive_count = 1;
    let (layer, guard) = non_blocking_layer(config)?;
    // Leave the `log` facade owned by tauri-plugin-log in development builds.
    let subscriber =
        tracing_subscriber::registry().with(layer.with_filter(filter_fn(|metadata| {
            metadata.target() == "gaming_toolkit.diagnostics"
        })));
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(guard)
}

pub fn runtime_event(kind: &str, message: &str) {
    if kind == "error" {
        tracing::error!(target: "gaming_toolkit.diagnostics", subsystem = "runtime",
            event = "runtime.error", status = "failed", error_kind = "runtime_error",
            kind, "[Runtime] {message}");
    } else {
        tracing::info!(target: "gaming_toolkit.diagnostics", subsystem = "runtime",
            event = "runtime.activity", kind, "[Runtime] {message}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialization_preserves_the_existing_log_facade_and_filters_dependencies() {
        struct ExistingLogger;
        impl log::Log for ExistingLogger {
            fn enabled(&self, _: &log::Metadata<'_>) -> bool {
                true
            }
            fn log(&self, _: &log::Record<'_>) {}
            fn flush(&self) {}
        }
        static EXISTING_LOGGER: ExistingLogger = ExistingLogger;
        log::set_logger(&EXISTING_LOGGER).unwrap();
        let directory =
            std::env::temp_dir().join(format!("gaming-toolkit-init-{}", uuid::Uuid::new_v4()));
        let guard = initialize(&directory).unwrap();
        runtime_event("runtime", "Initialization verified");
        tracing::info!(target: "dependency", "Should not enter the application log");
        drop(guard);
        let text = std::fs::read_to_string(directory.join("GamingToolkit.jsonl")).unwrap();
        assert_eq!(text.lines().count(), 1);
        assert!(text.contains("Initialization verified"));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn writes_valid_events_with_shared_session_and_failure_metadata() {
        let directory =
            std::env::temp_dir().join(format!("gaming-toolkit-logs-{}", uuid::Uuid::new_v4()));
        let mut config = LoggerConfig::new(directory.join("test.jsonl"), "backend");
        config.archive_count = 1;
        let (layer, guard) = non_blocking_layer(config).unwrap();
        let subscriber = tracing_subscriber::registry().with(layer);
        tracing::subscriber::with_default(subscriber, || {
            runtime_event("runtime", "Automation started");
            runtime_event("error", "Test action failed");
        });
        drop(guard);
        let text = std::fs::read_to_string(directory.join("test.jsonl")).unwrap();
        assert!(text.ends_with('\n'));
        let events: Vec<serde_json::Value> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["schema_version"], 1);
        assert_eq!(events[0]["app_session_id"], events[1]["app_session_id"]);
        assert_eq!(events[1]["status"], "failed");
        assert_eq!(events[1]["error_kind"], "runtime_error");
        assert_eq!(events[1]["level"], "error");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
