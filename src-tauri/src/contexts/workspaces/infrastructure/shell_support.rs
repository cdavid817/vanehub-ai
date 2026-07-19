use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::workspaces::application::{
    ShellEvent, ShellLog, WorkspaceLogLevel, WorkspaceShellEventPort, WorkspaceShellIdPort,
    WorkspaceShellLogPort,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct UuidWorkspaceShellId;

impl WorkspaceShellIdPort for UuidWorkspaceShellId {
    fn next_shell_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

#[derive(Clone)]
pub(crate) struct TauriWorkspaceShellEventPublisher {
    app: AppHandle,
}

impl TauriWorkspaceShellEventPublisher {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum TauriShellEvent {
    #[serde(rename_all = "camelCase")]
    Output {
        shell_id: String,
        session_id: String,
        content: String,
    },
    #[serde(rename_all = "camelCase")]
    State {
        shell_id: String,
        session_id: String,
        state: &'static str,
        error: Option<String>,
    },
}

impl From<ShellEvent> for TauriShellEvent {
    fn from(event: ShellEvent) -> Self {
        match event {
            ShellEvent::Output {
                shell_id,
                session_id,
                content,
            } => Self::Output {
                shell_id,
                session_id,
                content,
            },
            ShellEvent::State {
                shell_id,
                session_id,
                state,
                error,
            } => Self::State {
                shell_id,
                session_id,
                state,
                error,
            },
        }
    }
}

impl WorkspaceShellEventPort for TauriWorkspaceShellEventPublisher {
    fn publish(&self, event: ShellEvent) {
        let _ = self.app.emit("shell:event", TauriShellEvent::from(event));
    }
}

#[derive(Clone)]
pub(crate) struct WorkspaceShellLoggingAdapter {
    logging: Arc<dyn DiagnosticLogPort>,
}

impl WorkspaceShellLoggingAdapter {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self { logging }
    }
}

impl WorkspaceShellLogPort for WorkspaceShellLoggingAdapter {
    fn write(&self, log: ShellLog) {
        let mut context = BTreeMap::new();
        context.insert("sessionId".to_string(), log.session_id);
        context.insert("shellId".to_string(), log.shell_id);
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity: match log.level {
                WorkspaceLogLevel::Error => LogSeverity::Error,
                WorkspaceLogLevel::Warn => LogSeverity::Warn,
                WorkspaceLogLevel::Info => LogSeverity::Info,
                WorkspaceLogLevel::Debug => LogSeverity::Debug,
            },
            category: "session.shell".to_string(),
            message: log.message,
            context,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::application::ApplicationError;
    use std::sync::Mutex;

    #[derive(Default)]
    struct CapturingDiagnostics {
        logs: Mutex<Vec<DiagnosticLog>>,
    }

    impl DiagnosticLogPort for CapturingDiagnostics {
        fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), ApplicationError> {
            self.logs.lock().expect("logs").push(log);
            Ok(())
        }
    }

    #[test]
    fn tauri_shell_events_keep_tagged_camel_case_contracts() {
        let output = TauriShellEvent::from(ShellEvent::Output {
            shell_id: "shell-1".to_string(),
            session_id: "session-1".to_string(),
            content: "ready".to_string(),
        });
        let state = TauriShellEvent::from(ShellEvent::State {
            shell_id: "shell-1".to_string(),
            session_id: "session-1".to_string(),
            state: "disconnected",
            error: None,
        });

        assert_eq!(
            serde_json::to_value(output).expect("output"),
            serde_json::json!({
                "type": "output",
                "shellId": "shell-1",
                "sessionId": "session-1",
                "content": "ready"
            })
        );
        assert_eq!(
            serde_json::to_value(state).expect("state"),
            serde_json::json!({
                "type": "state",
                "shellId": "shell-1",
                "sessionId": "session-1",
                "state": "disconnected",
                "error": null
            })
        );
    }

    #[test]
    fn shell_logs_use_unified_diagnostic_category_and_trace_context() {
        let diagnostics = Arc::new(CapturingDiagnostics::default());
        let adapter = WorkspaceShellLoggingAdapter::new(diagnostics.clone());
        adapter.write(ShellLog {
            level: WorkspaceLogLevel::Warn,
            session_id: "session-1".to_string(),
            shell_id: "shell-1".to_string(),
            message: "Shell input failed.".to_string(),
        });

        let logs = diagnostics.logs.lock().expect("logs");
        assert_eq!(logs[0].severity, LogSeverity::Warn);
        assert_eq!(logs[0].category, "session.shell");
        assert_eq!(
            logs[0].context.get("sessionId").map(String::as_str),
            Some("session-1")
        );
        assert_eq!(
            logs[0].context.get("shellId").map(String::as_str),
            Some("shell-1")
        );
    }
}
