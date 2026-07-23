use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort, AgentOperation,
    AgentRuntimeApplicationError, AgentTaskPort, LoopLog, LoopLoggingPort, LoopOperationContext,
};
use crate::contexts::operations::api::{
    DiagnosticLog, DiagnosticLogPort, LogSeverity, OperationKind, OperationLog, OperationLogPort,
    OperationsApi,
};
use crate::platform::clock::SystemClock;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemAgentRuntimeClock;

impl AgentClockPort for SystemAgentRuntimeClock {
    fn now(&self) -> String {
        SystemClock.rfc3339()
    }
}

#[derive(Clone)]
pub(crate) struct AgentRuntimeOperationAdapter {
    operations: OperationsApi,
}

impl AgentRuntimeOperationAdapter {
    pub(crate) fn new(operations: OperationsApi) -> Self {
        Self { operations }
    }
}

impl AgentTaskPort for AgentRuntimeOperationAdapter {
    fn start_agent_launch(
        &self,
        agent_id: &str,
        message: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        self.start(agent_id, message.to_string())
    }

    fn start_agent_generation(
        &self,
        agent_id: &str,
        session_id: &str,
        message_id: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        self.start(
            agent_id,
            format!("Generating response for session {session_id} message {message_id}"),
        )
    }

    fn start_loop_operation(
        &self,
        context: &LoopOperationContext,
        message: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        self.operations
            .start(
                OperationKind::Agent,
                Some(context.run_id.clone()),
                Some(format!("Loop {}: {message}", context.kind.as_str())),
            )
            .map(|operation| AgentOperation {
                id: operation.id,
                related_agent_id: operation.related_entity_id,
                message: operation.message,
            })
            .map_err(operation_error)
    }

    fn append_log(
        &self,
        operation_id: &str,
        line: String,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.operations
            .append_log(operation_id, line)
            .map(|_| ())
            .map_err(operation_error)
    }

    fn complete(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.operations
            .complete(operation_id, None)
            .map(|_| ())
            .map_err(operation_error)
    }

    fn fail(&self, operation_id: &str, error: String) -> Result<(), AgentRuntimeApplicationError> {
        self.operations
            .fail(operation_id, error)
            .map(|_| ())
            .map_err(operation_error)
    }

    fn cancel(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.operations
            .cancel(operation_id)
            .map(|_| ())
            .map_err(operation_error)
    }
}

impl AgentRuntimeOperationAdapter {
    fn start(
        &self,
        agent_id: &str,
        message: String,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        self.operations
            .start(
                OperationKind::Agent,
                Some(agent_id.to_string()),
                Some(message),
            )
            .map(|operation| AgentOperation {
                id: operation.id,
                related_agent_id: operation.related_entity_id,
                message: operation.message,
            })
            .map_err(operation_error)
    }
}

#[derive(Clone)]
pub(crate) struct AgentRuntimeLoggingAdapter {
    diagnostics: Arc<dyn DiagnosticLogPort>,
    operations: Arc<dyn OperationLogPort>,
}

impl AgentRuntimeLoggingAdapter {
    pub(crate) fn new(
        diagnostics: Arc<dyn DiagnosticLogPort>,
        operations: Arc<dyn OperationLogPort>,
    ) -> Self {
        Self {
            diagnostics,
            operations,
        }
    }
}

impl AgentLoggingPort for AgentRuntimeLoggingAdapter {
    fn record(&self, log: AgentLog) -> Result<(), AgentRuntimeApplicationError> {
        let severity = log_severity(log.level);
        let mut context = BTreeMap::new();
        context.insert("occurredAt".to_string(), log.occurred_at);
        if let Some(agent_id) = log.agent_id {
            context.insert("agentId".to_string(), agent_id);
        }
        if let Some(session_id) = log.session_id {
            context.insert("sessionId".to_string(), session_id);
        }
        match log.operation_id {
            Some(operation_id) => self
                .operations
                .write_operation(OperationLog {
                    operation_id,
                    severity,
                    category: log.category,
                    message: log.message,
                    context,
                })
                .map_err(logging_error),
            None => self
                .diagnostics
                .write_diagnostic(DiagnosticLog {
                    severity,
                    category: log.category,
                    message: log.message,
                    context,
                })
                .map_err(logging_error),
        }
    }
}

impl LoopLoggingPort for AgentRuntimeLoggingAdapter {
    fn record_loop(&self, log: LoopLog) -> Result<(), AgentRuntimeApplicationError> {
        let severity = log_severity(log.level);
        let mut context = BTreeMap::from([
            ("occurredAt".to_string(), log.occurred_at),
            ("runId".to_string(), log.context.run_id),
            (
                "loopOperation".to_string(),
                log.context.kind.as_str().to_string(),
            ),
        ]);
        if let Some(iteration_id) = log.context.iteration_id {
            context.insert("iterationId".to_string(), iteration_id);
        }
        match log.operation_id {
            Some(operation_id) => self
                .operations
                .write_operation(OperationLog {
                    operation_id,
                    severity,
                    category: log.category,
                    message: log.message,
                    context,
                })
                .map_err(logging_error),
            None => self
                .diagnostics
                .write_diagnostic(DiagnosticLog {
                    severity,
                    category: log.category,
                    message: log.message,
                    context,
                })
                .map_err(logging_error),
        }
    }
}

fn log_severity(level: AgentLogLevel) -> LogSeverity {
    match level {
        AgentLogLevel::Error => LogSeverity::Error,
        AgentLogLevel::Warn => LogSeverity::Warn,
        AgentLogLevel::Info => LogSeverity::Info,
        AgentLogLevel::Debug => LogSeverity::Debug,
    }
}

fn operation_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Operation(error.to_string())
}

fn logging_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Logging(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::api::OperationsError;
    use std::sync::Mutex;

    #[derive(Default)]
    struct CapturedLogs {
        diagnostics: Mutex<Vec<DiagnosticLog>>,
        operations: Mutex<Vec<OperationLog>>,
    }

    impl DiagnosticLogPort for CapturedLogs {
        fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), OperationsError> {
            self.diagnostics.lock().expect("diagnostics").push(log);
            Ok(())
        }
    }

    impl OperationLogPort for CapturedLogs {
        fn write_operation(&self, log: OperationLog) -> Result<(), OperationsError> {
            self.operations.lock().expect("operations").push(log);
            Ok(())
        }
    }

    #[test]
    fn logging_routes_operation_association_without_losing_runtime_context() {
        let captured = Arc::new(CapturedLogs::default());
        let adapter = AgentRuntimeLoggingAdapter::new(captured.clone(), captured.clone());

        adapter
            .record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime".to_string(),
                message: "provider warning".to_string(),
                agent_id: Some("codex-cli".to_string()),
                session_id: Some("session-1".to_string()),
                operation_id: Some("operation-1".to_string()),
                occurred_at: "2026-07-18T10:00:00Z".to_string(),
            })
            .expect("operation log");

        assert!(captured.diagnostics.lock().expect("diagnostics").is_empty());
        let logs = captured.operations.lock().expect("operations");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].operation_id, "operation-1");
        assert_eq!(logs[0].severity, LogSeverity::Warn);
        assert_eq!(
            logs[0].context.get("agentId").map(String::as_str),
            Some("codex-cli")
        );
        assert_eq!(
            logs[0].context.get("sessionId").map(String::as_str),
            Some("session-1")
        );
    }

    #[test]
    fn loop_logging_keeps_stable_run_iteration_and_operation_context() {
        let captured = Arc::new(CapturedLogs::default());
        let adapter = AgentRuntimeLoggingAdapter::new(captured.clone(), captured.clone());

        adapter
            .record_loop(LoopLog {
                level: AgentLogLevel::Info,
                category: "loop.verification".to_string(),
                message: "check completed token=secret-value".to_string(),
                context: LoopOperationContext {
                    run_id: "run-1".to_string(),
                    iteration_id: Some("iteration-2".to_string()),
                    kind: crate::contexts::agent_runtime::application::LoopOperationKind::Verification,
                },
                operation_id: Some("operation-3".to_string()),
                occurred_at: "2026-07-18T10:00:00Z".to_string(),
            })
            .expect("Loop operation log");

        assert!(captured.diagnostics.lock().expect("diagnostics").is_empty());
        let logs = captured.operations.lock().expect("operations");
        assert_eq!(logs[0].operation_id, "operation-3");
        assert_eq!(
            logs[0].context.get("runId").map(String::as_str),
            Some("run-1")
        );
        assert_eq!(
            logs[0].context.get("iterationId").map(String::as_str),
            Some("iteration-2")
        );
        assert_eq!(
            logs[0].context.get("loopOperation").map(String::as_str),
            Some("verification")
        );
    }
}
