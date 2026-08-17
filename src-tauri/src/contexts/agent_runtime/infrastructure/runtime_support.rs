use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort, AgentOperation,
    AgentRuntimeApplicationError, AgentTaskPort, CanonicalLoopSignal, CanonicalRunLinks,
    CanonicalRunOutcome, CanonicalRunSignal, LoopLog, LoopLoggingPort, LoopOperationContext,
};
use crate::contexts::operations::api::{
    AgentRunsApi, CreateAgentRun, DiagnosticLog, DiagnosticLogPort, LogSeverity, OperationKind,
    OperationLog, OperationLogPort, OperationsApi, RunLink, RunOwner, RunRecoveryPolicy, RunState,
    RunTrigger,
};
use crate::platform::clock::SystemClock;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemAgentRuntimeClock;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemExpertRoleClock;

impl crate::contexts::agent_runtime::application::ExpertRoleClockPort for SystemExpertRoleClock {
    fn now(&self) -> String {
        crate::platform::clock::SystemClock.rfc3339()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct UuidExpertRoleIds;

impl crate::contexts::agent_runtime::application::ExpertRoleIdPort for UuidExpertRoleIds {
    fn next_id(&self) -> String {
        format!("expert-role-{}", uuid::Uuid::new_v4())
    }
}

impl AgentClockPort for SystemAgentRuntimeClock {
    fn now(&self) -> String {
        SystemClock.rfc3339()
    }
}

#[derive(Clone)]
pub(crate) struct AgentRuntimeOperationAdapter {
    operations: OperationsApi,
    runs: AgentRunsApi,
}

impl AgentRuntimeOperationAdapter {
    pub(crate) fn new(operations: OperationsApi, runs: AgentRunsApi) -> Self {
        Self { operations, runs }
    }
}

impl AgentTaskPort for AgentRuntimeOperationAdapter {
    fn start_canonical_loop(
        &self,
        loop_run_id: &str,
        definition_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let run_id = canonical_owner_run_id(loop_run_id)?;
        self.runs
            .create(CreateAgentRun {
                id: Some(run_id.clone()),
                owner: RunOwner {
                    owner_type: "loop_run".into(),
                    owner_id: loop_run_id.into(),
                },
                links: vec![RunLink {
                    link_type: "loop_definition".into(),
                    link_id: definition_id.into(),
                }],
                parent_run_id: None,
                recovery_policy: RunRecoveryPolicy::OwnerReconciles,
                max_retries: 3,
                witness: format!("loop-accepted:{run_id}"),
            })
            .and_then(|_| self.runs.transition(&run_id, RunTrigger::Prepare, None))
            .map(|_| ())
            .map_err(operation_error)
    }

    fn signal_canonical_loop(
        &self,
        loop_run_id: &str,
        signal: CanonicalLoopSignal,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let run_id = canonical_owner_run_id(loop_run_id)?;
        let run = self.runs.get(&run_id).map_err(operation_error)?;
        if run.state.is_terminal() {
            return Err(operation_error("late Loop lifecycle signal rejected"));
        }
        let trigger = match (run.state, signal) {
            (RunState::Preparing, CanonicalLoopSignal::Running) => RunTrigger::Start,
            (RunState::Verifying, CanonicalLoopSignal::Running) => RunTrigger::Continue,
            (RunState::Retrying, CanonicalLoopSignal::Running) => RunTrigger::RetryReady,
            (_, CanonicalLoopSignal::Paused) => RunTrigger::Pause,
            (_, CanonicalLoopSignal::Resumed) => RunTrigger::Resume,
            (_, CanonicalLoopSignal::Retrying) => RunTrigger::Retry,
            (_, CanonicalLoopSignal::Verifying) => RunTrigger::Verify,
            (_, CanonicalLoopSignal::Stuck) => RunTrigger::MarkStuck,
            (_, CanonicalLoopSignal::Completed) => RunTrigger::Complete,
            (_, CanonicalLoopSignal::Failed) => RunTrigger::Fail,
            (_, CanonicalLoopSignal::Cancelled) => RunTrigger::CancelUser,
            _ => return Ok(()),
        };
        self.runs
            .transition(
                &run_id,
                trigger,
                Some(format!("loop_{signal:?}").to_lowercase()),
            )
            .map(|_| ())
            .map_err(operation_error)
    }

    fn start_canonical_run(
        &self,
        run_id: &str,
        owner_id: &str,
        parent_run_id: Option<&str>,
        links: CanonicalRunLinks<'_>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.runs
            .create(CreateAgentRun {
                id: Some(run_id.to_string()),
                owner: RunOwner {
                    owner_type: "session_generation".into(),
                    owner_id: owner_id.into(),
                },
                links: [
                    ("session", Some(links.session_id)),
                    ("user_message", links.user_message_id),
                    ("assistant_message", Some(links.assistant_message_id)),
                    ("operation", Some(links.operation_id)),
                ]
                .into_iter()
                .filter_map(|(link_type, link_id)| {
                    link_id.map(|link_id| RunLink {
                        link_type: link_type.into(),
                        link_id: link_id.into(),
                    })
                })
                .collect(),
                parent_run_id: parent_run_id.map(str::to_string),
                recovery_policy: RunRecoveryPolicy::NotRecoverable,
                max_retries: 2,
                witness: format!("accepted:{run_id}"),
            })
            .map_err(operation_error)?;
        self.runs
            .transition(run_id, RunTrigger::Prepare, None)
            .map_err(operation_error)?;
        self.runs
            .transition(run_id, RunTrigger::Start, None)
            .map(|_| ())
            .map_err(operation_error)
    }

    fn finish_canonical_run(
        &self,
        run_id: &str,
        outcome: CanonicalRunOutcome,
        reason: Option<&str>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let trigger = match outcome {
            CanonicalRunOutcome::Completed => RunTrigger::Complete,
            CanonicalRunOutcome::Failed => RunTrigger::Fail,
            CanonicalRunOutcome::Cancelled => RunTrigger::CancelUser,
        };
        if outcome == CanonicalRunOutcome::Completed {
            self.runs
                .transition(run_id, RunTrigger::Verify, None)
                .map_err(operation_error)?;
        }
        self.runs
            .transition(run_id, trigger, reason.map(str::to_string))
            .map(|_| ())
            .map_err(operation_error)
    }

    fn signal_canonical_run(
        &self,
        run_id: &str,
        signal: CanonicalRunSignal,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let run = self.runs.get(run_id).map_err(operation_error)?;
        if run.state.is_terminal() {
            return Err(operation_error("late Agent lifecycle signal rejected"));
        }
        let trigger = match (run.state, signal) {
            (RunState::Running, CanonicalRunSignal::WaitingApproval) => {
                Some(RunTrigger::RequestApproval)
            }
            (RunState::Running, CanonicalRunSignal::WaitingUser) => Some(RunTrigger::AskUser),
            (RunState::WaitingApproval, CanonicalRunSignal::Active) => {
                Some(RunTrigger::ApprovalGranted)
            }
            (RunState::WaitingUser, CanonicalRunSignal::Active) => Some(RunTrigger::UserAnswered),
            _ => None,
        };
        match trigger {
            Some(trigger) => self
                .runs
                .transition(run_id, trigger, None)
                .map(|_| ())
                .map_err(operation_error),
            None => Ok(()),
        }
    }

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

    fn correlate_execution(
        &self,
        operation_id: &str,
        run_id: &str,
        trace_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.operations
            .correlate_execution(operation_id, run_id.to_string(), trace_id.to_string())
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
        if let Some(run_id) = log.run_id {
            context.insert("runId".to_string(), run_id);
        }
        if let Some(trace_id) = log.trace_id {
            context.insert("traceId".to_string(), trace_id);
        }
        if let Some(span_id) = log.span_id {
            context.insert("spanId".to_string(), span_id);
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

fn canonical_owner_run_id(owner_id: &str) -> Result<String, AgentRuntimeApplicationError> {
    let candidate = owner_id
        .get(owner_id.len().saturating_sub(36)..)
        .ok_or_else(|| operation_error("owner run id is invalid"))?;
    uuid::Uuid::parse_str(candidate)
        .map(|value| value.to_string())
        .map_err(|_| operation_error("owner run id is invalid"))
}

fn logging_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Logging(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::api::OperationsError;
    use crate::contexts::operations::infrastructure::persistent_operation_service;
    use crate::platform::database::NativeDatabase;
    use crate::test_support::TempDirectory;
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
                run_id: Some("run-1".to_string()),
                trace_id: Some("trace-1".to_string()),
                span_id: Some("span-1".to_string()),
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
        assert_eq!(
            logs[0].context.get("traceId").map(String::as_str),
            Some("trace-1")
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

    #[test]
    fn canonical_generation_projects_normal_waiting_and_terminal_paths() {
        let directory = TempDirectory::new("agent-canonical-run");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let runs = crate::bootstrap::assemble_agent_runs_api(database.clone());
        let adapter = AgentRuntimeOperationAdapter::new(
            OperationsApi::new(persistent_operation_service(database)),
            runs.clone(),
        );
        let run_id = "018f0f17-4d6a-7e20-b41d-66c5271a28d0";
        adapter
            .start_canonical_run(
                run_id,
                "assistant-1",
                None,
                CanonicalRunLinks {
                    session_id: "session-1",
                    user_message_id: Some("message-1"),
                    assistant_message_id: "assistant-1",
                    operation_id: "operation-1",
                },
            )
            .expect("start");
        for signal in [
            CanonicalRunSignal::WaitingApproval,
            CanonicalRunSignal::Active,
            CanonicalRunSignal::WaitingUser,
            CanonicalRunSignal::Active,
        ] {
            adapter
                .signal_canonical_run(run_id, signal)
                .expect("signal");
        }
        adapter
            .finish_canonical_run(run_id, CanonicalRunOutcome::Completed, None)
            .expect("complete");
        assert_eq!(runs.get(run_id).expect("run").state, RunState::Completed);
        let states = runs
            .events(run_id, 0, 20)
            .expect("events")
            .into_iter()
            .map(|event| event.state)
            .collect::<Vec<_>>();
        assert!(states.contains(&RunState::WaitingApproval));
        assert!(states.contains(&RunState::WaitingUser));
        assert!(adapter
            .signal_canonical_run(run_id, CanonicalRunSignal::Active)
            .is_err());
    }
}
