use crate::contexts::agent_runtime::api::{
    AgentChatConfiguration, AgentRuntimeApi, InteractionMode, SendMessageRequest,
};
use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::sessions::api::{
    NewSessionRequest, NewSessionWorkspace, SessionActivation, SessionOwner, SessionsApi,
};
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
            log_scheduled_task(fallback_log_directory, LogSeverity::Error, "scheduled-tasks.scan", &error.to_string(), None);
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
        log_scheduled_task(fallback_log_directory, LogSeverity::Info, "scheduled-tasks.run.start", &task.name, Some(&task.id));
        if let Err(error) = scheduled_tasks::mark_task_running(database, &task.id) {
            log_scheduled_task(fallback_log_directory, LogSeverity::Warn, "scheduled-tasks.run.skipped", &error.to_string(), Some(&task.id));
            continue;
        }
        match run_one_task(sessions, agents, &task) {
            Ok(session_id) => {
                if let Err(error) = scheduled_tasks::mark_task_succeeded(database, &task, &session_id) {
                    log_scheduled_task(fallback_log_directory, LogSeverity::Warn, "scheduled-tasks.run.state", &error.to_string(), Some(&task.id));
                }
                log_scheduled_task(fallback_log_directory, LogSeverity::Info, "scheduled-tasks.run.complete", &session_id, Some(&task.id));
            }
            Err(error) => {
                let message = error.to_string();
                let _ = scheduled_tasks::mark_task_failed(database, &task, &message);
                log_scheduled_task(fallback_log_directory, LogSeverity::Error, "scheduled-tasks.run.failed", &message, Some(&task.id));
            }
        }
    }
}

fn run_one_task(
    sessions: &SessionsApi,
    agents: &AgentRuntimeApi,
    task: &scheduled_tasks::ScheduledTask,
) -> Result<String, Box<dyn std::error::Error>> {
    let prepared = sessions.prepare_creation(NewSessionRequest {
        agent_id: task.agent_id.clone(),
        interaction_mode: "cli".to_string(),
        title: Some(task.name.clone()),
        workspace: NewSessionWorkspace::default(),
        owner: SessionOwner::desktop(),
        activation: SessionActivation::PreserveActive,
    })?;
    let session = sessions.execute_creation(prepared)?;
    agents.send_message(SendMessageRequest {
        session_id: session.id().to_string(),
        content: task.content.clone(),
        configuration: AgentChatConfiguration {
            agent_id: task.agent_id.clone(),
            interaction_mode: InteractionMode::Cli,
            permission_mode: "default".to_string(),
            provider_id: None,
            model_id: None,
            reasoning_depth: None,
            streaming: true,
            thinking: false,
            long_context: false,
        },
        file_references: Vec::new(),
    })?;
    Ok(session.id().to_string())
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
