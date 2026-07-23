use crate::contexts::execution_observability::api::ExecutionObservabilityApi;
use crate::contexts::execution_observability::infrastructure::OsObservabilityCredentialAdapter;
use crate::contexts::execution_observability::infrastructure::SqliteExecutionTimelineRepository;
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::platform::database::NativeDatabase;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub(crate) fn assemble_execution_observability_api(
    database: NativeDatabase,
) -> ExecutionObservabilityApi {
    ExecutionObservabilityApi::new(
        Arc::new(SqliteExecutionTimelineRepository::new(database)),
        Arc::new(OsObservabilityCredentialAdapter::new()),
    )
}

pub(crate) fn start_execution_retention_job(
    database: NativeDatabase,
    fallback_log_directory: PathBuf,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            run_retention_cycle(&database, &fallback_log_directory);
            sleep(Duration::from_secs(6 * 60 * 60)).await;
        }
    });
}

fn run_retention_cycle(database: &NativeDatabase, fallback_log_directory: &std::path::Path) {
    let repository = SqliteExecutionTimelineRepository::new(database.clone());
    let result = repository.load_settings().and_then(|settings| {
        repository.maintain_retention(&chrono::Utc::now().to_rfc3339(), settings.retention_days)
    });
    match result {
        Ok(outcome) if outcome.ran && outcome.deleted_runs > 0 => {
            write_retention_log(
                fallback_log_directory,
                LogSeverity::Info,
                "Execution timeline retention removed expired runs",
                Some(outcome.deleted_runs),
            );
        }
        Ok(_) => {}
        Err(_) => write_retention_log(
            fallback_log_directory,
            LogSeverity::Warn,
            "Execution timeline retention was deferred after a storage error",
            None,
        ),
    }
}

fn write_retention_log(
    fallback_log_directory: &std::path::Path,
    severity: LogSeverity,
    message: &str,
    deleted_runs: Option<usize>,
) {
    let logging = UnifiedLoggingAdapter::active(fallback_log_directory.to_path_buf());
    let mut context = BTreeMap::from([("source".to_string(), "scheduled-maintenance".to_string())]);
    if let Some(deleted_runs) = deleted_runs {
        context.insert("deletedRuns".to_string(), deleted_runs.to_string());
    }
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity,
        category: "execution_observability.retention".to_string(),
        message: message.to_string(),
        context,
    });
}
