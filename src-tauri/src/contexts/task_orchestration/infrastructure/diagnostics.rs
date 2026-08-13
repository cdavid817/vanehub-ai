use crate::contexts::operations::application::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::contexts::task_orchestration::application::{
    PlanDiagnostic, PlanDiagnosticLevel, PlanDiagnosticsPort,
};
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct UnifiedPlanDiagnosticsAdapter {
    logging: Arc<dyn DiagnosticLogPort>,
}

impl UnifiedPlanDiagnosticsAdapter {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self { logging }
    }
}

impl PlanDiagnosticsPort for UnifiedPlanDiagnosticsAdapter {
    fn record(&self, diagnostic: PlanDiagnostic) {
        let mut context = BTreeMap::new();
        insert(&mut context, "planRunId", diagnostic.plan_run_id);
        insert(&mut context, "subtaskRunId", diagnostic.subtask_run_id);
        insert(&mut context, "attemptId", diagnostic.attempt_id);
        insert(&mut context, "sessionId", diagnostic.session_id);
        insert(&mut context, "operationId", diagnostic.operation_id);
        insert(&mut context, "executionRunId", diagnostic.execution_run_id);
        insert(&mut context, "state", diagnostic.state.map(str::to_string));
        insert(
            &mut context,
            "errorClass",
            diagnostic.error_class.map(str::to_string),
        );
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity: severity(diagnostic.level),
            category: "task_orchestration.lifecycle".to_string(),
            message: diagnostic.event.to_string(),
            context,
        });
    }
}

fn insert(context: &mut BTreeMap<String, String>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        context.insert(key.to_string(), value);
    }
}

fn severity(level: PlanDiagnosticLevel) -> LogSeverity {
    match level {
        PlanDiagnosticLevel::Error => LogSeverity::Error,
        PlanDiagnosticLevel::Warn => LogSeverity::Warn,
        PlanDiagnosticLevel::Info => LogSeverity::Info,
        PlanDiagnosticLevel::Debug => LogSeverity::Debug,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::application::ApplicationError;
    use std::sync::Mutex;

    #[derive(Default)]
    struct CapturingLogs(Mutex<Vec<DiagnosticLog>>);

    impl DiagnosticLogPort for CapturingLogs {
        fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), ApplicationError> {
            self.0.lock().expect("logs").push(log);
            Ok(())
        }
    }

    #[test]
    fn lifecycle_logs_preserve_levels_and_only_publish_safe_correlation_fields() {
        let logs = Arc::new(CapturingLogs::default());
        let adapter = UnifiedPlanDiagnosticsAdapter::new(logs.clone());
        for level in [
            PlanDiagnosticLevel::Error,
            PlanDiagnosticLevel::Warn,
            PlanDiagnosticLevel::Info,
            PlanDiagnosticLevel::Debug,
        ] {
            adapter.record(PlanDiagnostic {
                level,
                event: "plan.attempt.correlated",
                plan_run_id: Some("run-1".to_string()),
                subtask_run_id: Some("subtask-run-1".to_string()),
                attempt_id: Some("attempt-1".to_string()),
                session_id: Some("session-1".to_string()),
                operation_id: Some("operation-1".to_string()),
                execution_run_id: Some("execution-1".to_string()),
                state: Some("running"),
                error_class: None,
            });
        }

        let recorded = logs.0.lock().expect("logs");
        assert_eq!(
            recorded.iter().map(|log| log.severity).collect::<Vec<_>>(),
            vec![
                LogSeverity::Error,
                LogSeverity::Warn,
                LogSeverity::Info,
                LogSeverity::Debug,
            ]
        );
        let allowed = [
            "planRunId",
            "subtaskRunId",
            "attemptId",
            "sessionId",
            "operationId",
            "executionRunId",
            "state",
        ];
        assert!(recorded
            .iter()
            .flat_map(|log| log.context.keys())
            .all(|key| allowed.contains(&key.as_str())));
        let rendered = format!("{recorded:?}");
        for prohibited in [
            "goal",
            "description",
            "prompt",
            "credential",
            "toolPayload",
            "commandOutput",
        ] {
            assert!(!rendered.contains(prohibited));
        }
    }
}
