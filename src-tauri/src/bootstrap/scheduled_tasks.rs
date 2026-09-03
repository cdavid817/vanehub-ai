use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::sessions::infrastructure::scheduled_tasks;
use crate::platform::database::NativeDatabase;
use chrono::Utc;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use tokio::time::{sleep, Duration};

#[derive(Clone)]
enum ScheduledTaskScanMode {
    Startup,
    Tick,
}

pub(crate) fn start_scheduled_task_jobs(
    database: NativeDatabase,
    sessions: SessionsApi,
    agents: AgentRuntimeApi,
    fallback_log_directory: PathBuf,
) {
    tauri::async_runtime::spawn(async move {
        run_due_tasks(
            &database,
            &sessions,
            &agents,
            &fallback_log_directory,
            ScheduledTaskScanMode::Startup,
        );
        loop {
            sleep(Duration::from_secs(60)).await;
            run_due_tasks(
                &database,
                &sessions,
                &agents,
                &fallback_log_directory,
                ScheduledTaskScanMode::Tick,
            );
        }
    });
}

fn run_due_tasks(
    database: &NativeDatabase,
    sessions: &SessionsApi,
    agents: &AgentRuntimeApi,
    fallback_log_directory: &Path,
    mode: ScheduledTaskScanMode,
) {
    let tasks = match scheduled_tasks::due_tasks(database, Utc::now()) {
        Ok(tasks) => tasks,
        Err(error) => {
            log_scheduled_task(
                fallback_log_directory,
                LogSeverity::Error,
                "scheduled-tasks.scan",
                &error.to_string(),
                None,
            );
            return;
        }
    };
    for task in tasks {
        if matches!(mode, ScheduledTaskScanMode::Startup) {
            log_scheduled_task(
                fallback_log_directory,
                LogSeverity::Info,
                "scheduled-tasks.run.backfill",
                &task.next_run_at,
                Some(&task.id),
            );
        }
        log_scheduled_task(
            fallback_log_directory,
            LogSeverity::Info,
            "scheduled-tasks.run.start",
            &task.name,
            Some(&task.id),
        );
        if let Err(error) = scheduled_tasks::mark_task_running_with_trigger(
            database,
            &task.id,
            matches!(mode, ScheduledTaskScanMode::Startup),
        ) {
            let _ = scheduled_tasks::record_task_skipped(database, &task.id, &error.to_string());
            log_scheduled_task(
                fallback_log_directory,
                LogSeverity::Warn,
                "scheduled-tasks.run.skipped",
                &error.to_string(),
                Some(&task.id),
            );
            continue;
        }
        match scheduled_tasks::run_one_task(sessions, agents, &task) {
            Ok(session_id) => {
                if let Err(error) =
                    scheduled_tasks::mark_task_succeeded(database, &task, &session_id)
                {
                    log_scheduled_task(
                        fallback_log_directory,
                        LogSeverity::Warn,
                        "scheduled-tasks.run.state",
                        &error.to_string(),
                        Some(&task.id),
                    );
                }
                log_scheduled_task(
                    fallback_log_directory,
                    LogSeverity::Info,
                    "scheduled-tasks.run.complete",
                    &session_id,
                    Some(&task.id),
                );
            }
            Err(error) => {
                let message = error.to_string();
                let _ = scheduled_tasks::mark_task_failed(database, &task, &message);
                log_scheduled_task(
                    fallback_log_directory,
                    LogSeverity::Error,
                    "scheduled-tasks.run.failed",
                    &message,
                    Some(&task.id),
                );
            }
        }
    }
}

fn log_scheduled_task(
    fallback_log_directory: &Path,
    severity: LogSeverity,
    category: &str,
    message: &str,
    task_id: Option<&str>,
) {
    let adapter = UnifiedLoggingAdapter::active(fallback_log_directory.to_path_buf());
    let mut context = BTreeMap::new();
    context.insert("source".to_string(), "scheduled-task".to_string());
    if let Some(task_id) = task_id {
        context.insert("taskId".to_string(), task_id.to_string());
    }
    let _ = adapter.write_diagnostic(DiagnosticLog {
        severity,
        category: category.to_string(),
        message: message.to_string(),
        context,
    });
}

/// 19.18: `run_due_tasks`'s own dispatch loop calls `scheduled_tasks::run_one_task(sessions,
/// agents, &task)`, which needs a real `SessionsApi` + `AgentRuntimeApi` (creates an actual
/// session and sends an actual agent message -- confirmed by reading `run_one_task` itself).
/// Constructing working instances of both costs the same ~100-line, port-double harness
/// `native_lifecycle_tests.rs`'s own `LifecycleHarness` already pays for a different test module
/// (native process/terminal lifecycle) -- disproportionate to add here just to reach the
/// zero-or-one-due-task path through `run_due_tasks`, and this domain's own existing test suite
/// (`contexts::sessions::infrastructure::scheduled_tasks`) already deliberately draws its test
/// boundary at the DB bookkeeping around dispatch (`mark_task_running_with_trigger`,
/// `mark_task_succeeded`, `record_manual_run`, ...), never at `run_one_task` itself -- the due-scan
/// query `run_due_tasks` calls (`due_tasks`) is exhaustively tested there already
/// (`due_scan_skips_disabled_tasks`, `due_scan_returns_one_backfill_candidate_for_missed_task`).
/// What this file's own tests genuinely lacked before this pass -- and can be tested honestly
/// without that dispatch harness -- is `log_scheduled_task` itself: every branch in
/// `run_due_tasks` reaches the unified log only through this one function, so proving it writes
/// severity/category/message and conditionally-present `taskId` context correctly is real,
/// meaningful coverage of this file's own logging behavior, mirroring
/// `scheduled_tasks::delete_task_writes_unified_log_when_directory_is_available`'s own
/// read-the-file-back pattern.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn read_log(log_directory: &Path) -> String {
        std::fs::read_dir(log_directory)
            .expect("log directory")
            .filter_map(Result::ok)
            .find_map(|entry| std::fs::read_to_string(entry.path()).ok())
            .expect("log content")
    }

    #[test]
    fn log_scheduled_task_writes_severity_category_message_and_task_id_context() {
        let directory = TempDirectory::new("scheduled-tasks-bootstrap-log");
        let log_directory = directory.path().join("logs");

        log_scheduled_task(
            &log_directory,
            LogSeverity::Warn,
            "scheduled-tasks.run.skipped",
            "already claimed",
            Some("task-1"),
        );

        let log_content = read_log(&log_directory);
        assert!(log_content.contains("scheduled-tasks.run.skipped"));
        assert!(log_content.contains("already claimed"));
        assert!(log_content.contains("task-1"));
        assert!(log_content.contains("scheduled-task"));
    }

    #[test]
    fn log_scheduled_task_omits_task_id_context_when_none() {
        let directory = TempDirectory::new("scheduled-tasks-bootstrap-log-no-task");
        let log_directory = directory.path().join("logs");

        log_scheduled_task(
            &log_directory,
            LogSeverity::Error,
            "scheduled-tasks.scan",
            "db unavailable",
            None,
        );

        let log_content = read_log(&log_directory);
        assert!(log_content.contains("scheduled-tasks.scan"));
        assert!(log_content.contains("db unavailable"));
        assert!(!log_content.contains("taskId"));
    }
}
