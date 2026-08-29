use crate::contexts::agent_runtime::api::{AgentRuntimeApi, AgentRuntimeApplicationError};
use crate::contexts::operations::api::{
    DiagnosticLog, DiagnosticLogPort, LogSeverity, OperationsApi,
};
use crate::contexts::operations::domain::OperationStatus;
use crate::contexts::sessions::application::{
    ReportExportPort, ReportSourceError, ReportSourceResult, SessionApplicationLog,
    SessionApplicationLogLevel, SessionClockPort, SessionFileContentPort, SessionIdentityPort,
    SessionLoggingPort, SessionRuntimePort, SessionTerminalEvidencePort, SessionsApplicationError,
    UsageStatisticsRange,
};
use crate::contexts::sessions::domain::evidence::{
    LiveHandleEvidence, OperationTerminalEvidence, OperationTerminalStatus,
    SessionTerminalEvidence, MAX_RECOVERY_EVIDENCE_OPERATIONS,
};
use crate::contexts::sessions::domain::SessionId;
use crate::contexts::workspaces::api::{WorkspaceApi, WorkspaceError};
use crate::platform::clock::SystemClock;
use crate::platform::filesystem::BoundedFilesystem;
use chrono::{DateTime, Days, Duration, Local, TimeZone, Utc};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemSessionClock;

impl SessionClockPort for SystemSessionClock {
    fn now(&self) -> String {
        SystemClock.rfc3339()
    }

    fn inactivity_cutoff(&self, inactive_days: i64) -> Result<String, SessionsApplicationError> {
        let duration = Duration::try_days(inactive_days).ok_or_else(|| {
            SessionsApplicationError::Validation(
                "Automatic archival inactivity range is out of bounds.".to_string(),
            )
        })?;
        Utc::now()
            .checked_sub_signed(duration)
            .map(|value| value.to_rfc3339())
            .ok_or_else(|| {
                SessionsApplicationError::Validation(
                    "Automatic archival inactivity range is out of bounds.".to_string(),
                )
            })
    }

    fn usage_range_start(
        &self,
        range: UsageStatisticsRange,
    ) -> Result<Option<String>, SessionsApplicationError> {
        usage_range_start_at(Local::now(), range)
    }
}

fn usage_range_start_at(
    now: DateTime<Local>,
    range: UsageStatisticsRange,
) -> Result<Option<String>, SessionsApplicationError> {
    let days_back = match range {
        UsageStatisticsRange::All => return Ok(None),
        UsageStatisticsRange::Today => 0,
        UsageStatisticsRange::Last7Days => 6,
        UsageStatisticsRange::Last30Days => 29,
    };
    let date = now.checked_sub_days(Days::new(days_back)).ok_or_else(|| {
        SessionsApplicationError::Validation("usage range is out of bounds".to_string())
    })?;
    let midnight = date.date_naive().and_hms_opt(0, 0, 0).ok_or_else(|| {
        SessionsApplicationError::Validation("invalid local date boundary".to_string())
    })?;
    let local = Local
        .from_local_datetime(&midnight)
        .earliest()
        .ok_or_else(|| {
            SessionsApplicationError::Validation("unresolvable local date boundary".to_string())
        })?;
    Ok(Some(local.with_timezone(&Utc).to_rfc3339()))
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct UuidSessionIdentities;

impl SessionIdentityPort for UuidSessionIdentities {
    fn next_session_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn next_seat_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn next_message_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn next_category_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

#[derive(Clone)]
pub(crate) struct SessionFileAdapter {
    workspaces: WorkspaceApi,
    logging: Arc<dyn DiagnosticLogPort>,
}

impl SessionFileAdapter {
    pub(crate) fn new(workspaces: WorkspaceApi, logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self {
            workspaces,
            logging,
        }
    }
}

impl SessionFileContentPort for SessionFileAdapter {
    fn read_reference_text(
        &self,
        session_id: &str,
        path: &str,
    ) -> Result<Option<String>, SessionsApplicationError> {
        // Passing the absent content through instead of flattening it to "" is what lets
        // prompt assembly tell an unreadable file from an empty one.
        Ok(self
            .workspaces
            .read_session_text_file(session_id, path)
            .map_err(workspace_error)?
            .content)
    }

    fn write_export(
        &self,
        destination_directory: &str,
        filename: &str,
        content: &str,
    ) -> Result<String, SessionsApplicationError> {
        let destination = Path::new(destination_directory);
        let filesystem = BoundedFilesystem::new(destination).map_err(|_| {
            SessionsApplicationError::Validation(
                "Export destination directory is unavailable.".to_string(),
            )
        })?;
        let (path, _) = filesystem
            .resolve_with_existing_parent(filename)
            .map_err(|error| export_error(self.logging.as_ref(), error))?;
        std::fs::write(&path, content)
            .map_err(|error| export_error(self.logging.as_ref(), error))?;
        Ok(path.to_string_lossy().to_string())
    }
}

/// The report export, over the same bounded filesystem the session export uses.
///
/// Delegating rather than reimplementing is the whole point: `write_export` already refuses a
/// filename that escapes the picked directory, and a second write path would be a second place
/// that rule has to be remembered.
impl ReportExportPort for SessionFileAdapter {
    fn write_export(
        &self,
        destination_directory: &str,
        filename: &str,
        content: &str,
    ) -> ReportSourceResult<String> {
        SessionFileContentPort::write_export(self, destination_directory, filename, content)
            // The report's own vocabulary. A sessions error carries a message, and a message
            // crossing into a report is untranslated text in a document nobody redacted.
            .map_err(|_| ReportSourceError::Unavailable("report_export_failed"))
    }
}

fn export_error(
    logging: &dyn DiagnosticLogPort,
    error: impl std::fmt::Display,
) -> SessionsApplicationError {
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity: LogSeverity::Error,
        category: "session.export".to_string(),
        message: format!("Session export write failed: {error}"),
        context: BTreeMap::new(),
    });
    SessionsApplicationError::FileContent("Session export failed".to_string())
}

#[derive(Clone)]
pub(crate) struct UnifiedSessionLoggingAdapter {
    logging: Arc<dyn DiagnosticLogPort>,
}

impl UnifiedSessionLoggingAdapter {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self { logging }
    }
}

impl SessionLoggingPort for UnifiedSessionLoggingAdapter {
    fn write(&self, log: SessionApplicationLog) -> Result<(), SessionsApplicationError> {
        let mut context = BTreeMap::new();
        if let Some(session_id) = log.session_id {
            context.insert("sessionId".to_string(), session_id);
        }
        if let Some(operation_id) = log.operation_id {
            context.insert("operationId".to_string(), operation_id);
        }
        if let Some(execution_run_id) = log.execution_run_id {
            context.insert("executionRunId".to_string(), execution_run_id);
        }
        if let Some(recovery_report_id) = log.recovery_report_id {
            context.insert("recoveryReportId".to_string(), recovery_report_id);
        }
        self.logging
            .write_diagnostic(DiagnosticLog {
                severity: match log.level {
                    SessionApplicationLogLevel::Error => LogSeverity::Error,
                    SessionApplicationLogLevel::Warn => LogSeverity::Warn,
                    SessionApplicationLogLevel::Info => LogSeverity::Info,
                    SessionApplicationLogLevel::Debug => LogSeverity::Debug,
                },
                category: log.category,
                message: log.message,
                context,
            })
            .map_err(|error| SessionsApplicationError::Logging(error.to_string()))
    }
}

#[derive(Clone)]
pub(crate) struct AgentSessionRuntimeAdapter {
    workspaces: WorkspaceApi,
    agent_runtime: Arc<RwLock<Option<AgentRuntimeApi>>>,
    evidence: Arc<dyn SessionTerminalEvidencePort>,
    operations: Option<OperationsApi>,
}

impl AgentSessionRuntimeAdapter {
    pub(crate) fn new(
        workspaces: WorkspaceApi,
        evidence: Arc<dyn SessionTerminalEvidencePort>,
    ) -> Self {
        Self {
            workspaces,
            agent_runtime: Arc::new(RwLock::new(None)),
            evidence,
            operations: None,
        }
    }

    pub(crate) fn with_operations(mut self, operations: OperationsApi) -> Self {
        self.operations = Some(operations);
        self
    }

    pub(crate) fn attach_agent_runtime(
        &self,
        api: AgentRuntimeApi,
    ) -> Result<(), SessionsApplicationError> {
        *self
            .agent_runtime
            .write()
            .map_err(|error| SessionsApplicationError::Runtime(error.to_string()))? = Some(api);
        Ok(())
    }

    fn published_agent_runtime(&self) -> Result<AgentRuntimeApi, SessionsApplicationError> {
        self.agent_runtime
            .read()
            .map_err(|error| SessionsApplicationError::Runtime(error.to_string()))
            .and_then(|api| {
                api.clone().ok_or_else(|| {
                    SessionsApplicationError::Runtime(
                        "Agent runtime is not attached to the sessions context.".to_string(),
                    )
                })
            })
    }
}

impl SessionTerminalEvidencePort for AgentSessionRuntimeAdapter {
    fn read_terminal_evidence(
        &self,
        session_id: &SessionId,
        execution_run_id: Option<&str>,
    ) -> Result<SessionTerminalEvidence, SessionsApplicationError> {
        let mut evidence = self
            .evidence
            .read_terminal_evidence(session_id, execution_run_id)?;
        if let (Some(operations), Some(execution_run_id)) = (&self.operations, execution_run_id) {
            let correlated = operations
                .list_recovery_evidence(execution_run_id, MAX_RECOVERY_EVIDENCE_OPERATIONS + 1)
                .map_err(|error| {
                    match error {
                    crate::contexts::operations::application::ApplicationError::Infrastructure {
                        ..
                    } => SessionsApplicationError::RetryableStorage(error.to_string()),
                    _ => SessionsApplicationError::StructuralRecoveryEvidence(error.to_string()),
                }
                })?
                .into_iter()
                .map(|operation| OperationTerminalEvidence {
                    operation_id: operation.operation_id,
                    execution_run_id: Some(operation.execution_run_id),
                    status: operation_terminal_status(operation.status),
                })
                .collect();
            evidence.replace_operations(correlated).map_err(|error| {
                SessionsApplicationError::StructuralRecoveryEvidence(format!(
                    "operation evidence exceeded its bounded read: {error:?}"
                ))
            })?;
        }
        let runtime = match self.published_agent_runtime() {
            Ok(runtime) => runtime,
            Err(_) => {
                evidence.live_handle = LiveHandleEvidence::Unavailable;
                return Ok(evidence);
            }
        };
        let correlation = runtime
            .active_generation_correlation(session_id.as_str())
            .map_err(agent_runtime_error)?;
        evidence.live_handle = if correlation.is_some_and(|correlation| {
            execution_run_id.is_none()
                || correlation.execution_run_id.as_deref() == execution_run_id
        }) {
            LiveHandleEvidence::Present
        } else {
            LiveHandleEvidence::Absent
        };
        Ok(evidence)
    }
}

fn operation_terminal_status(status: OperationStatus) -> OperationTerminalStatus {
    match status {
        OperationStatus::Queued | OperationStatus::Running => OperationTerminalStatus::Running,
        OperationStatus::Succeeded => OperationTerminalStatus::Succeeded,
        OperationStatus::Failed => OperationTerminalStatus::Failed,
        OperationStatus::Cancelled => OperationTerminalStatus::Cancelled,
    }
}

impl SessionRuntimePort for AgentSessionRuntimeAdapter {
    fn stop_session_activity(&self, session_id: &str) -> Result<(), SessionsApplicationError> {
        let runtime = self.published_agent_runtime()?;
        runtime
            .stop_generation(session_id)
            .map_err(agent_runtime_error)?;
        // Background commands are reaped on the same "this session is done" edge as workspace
        // shells below, and for the same reason: neither has an owner left to attend to it.
        runtime.reap_background_commands(session_id);
        self.workspaces
            .kill_shells_for_session(session_id)
            .map_err(workspace_error)
    }
}

fn agent_runtime_error(error: AgentRuntimeApplicationError) -> SessionsApplicationError {
    match error {
        AgentRuntimeApplicationError::Domain(error) => {
            SessionsApplicationError::Validation(error.to_string())
        }
        AgentRuntimeApplicationError::Provider(error) => {
            SessionsApplicationError::RuntimeLaunch(error.to_string())
        }
        AgentRuntimeApplicationError::Validation(message) => {
            SessionsApplicationError::Validation(message)
        }
        AgentRuntimeApplicationError::GenerationConflict(session_id) => {
            SessionsApplicationError::Validation(format!(
                "A generation is already active for session {session_id}."
            ))
        }
        AgentRuntimeApplicationError::PolicyDenied { session_id, action } => {
            SessionsApplicationError::Validation(format!(
                "Verifier session {session_id} cannot perform Agent action: {action}"
            ))
        }
        AgentRuntimeApplicationError::AgentNotFound(agent_id) => {
            SessionsApplicationError::AgentNotFound(agent_id)
        }
        AgentRuntimeApplicationError::InvalidSeatMention { .. } => {
            SessionsApplicationError::Validation("Mentioned seat is unavailable.".to_string())
        }
        AgentRuntimeApplicationError::SessionNotFound(session_id) => {
            SessionsApplicationError::SessionNotFound(session_id)
        }
        AgentRuntimeApplicationError::MessageNotFound(message_id) => {
            SessionsApplicationError::MessageNotFound(message_id)
        }
        AgentRuntimeApplicationError::UnsupportedInteractionMode(mode) => {
            SessionsApplicationError::UnsupportedInteractionMode(mode)
        }
        AgentRuntimeApplicationError::Process(message) => {
            SessionsApplicationError::RuntimeLaunch(message)
        }
        AgentRuntimeApplicationError::VerificationPolicy(message) => {
            SessionsApplicationError::Validation(message)
        }
        AgentRuntimeApplicationError::NoActiveAgent => {
            SessionsApplicationError::Runtime("No active agent selected.".to_string())
        }
        AgentRuntimeApplicationError::AgentUnavailable(message)
        | AgentRuntimeApplicationError::Registry(message)
        | AgentRuntimeApplicationError::Workflow(message)
        | AgentRuntimeApplicationError::Session(message)
        | AgentRuntimeApplicationError::CliProfile(message)
        | AgentRuntimeApplicationError::Prompt(message)
        | AgentRuntimeApplicationError::Operation(message)
        | AgentRuntimeApplicationError::Loop(message)
        | AgentRuntimeApplicationError::VerificationProcess(message)
        | AgentRuntimeApplicationError::Logging(message)
        | AgentRuntimeApplicationError::Event(message)
        | AgentRuntimeApplicationError::Generation(message)
        | AgentRuntimeApplicationError::Credential(message)
        | AgentRuntimeApplicationError::Skill(message)
        | AgentRuntimeApplicationError::Memory(message)
        | AgentRuntimeApplicationError::Mcp(message)
        | AgentRuntimeApplicationError::Permission(message)
        | AgentRuntimeApplicationError::ContextQuality(message) => {
            SessionsApplicationError::Runtime(message)
        }
    }
}

fn workspace_error(error: WorkspaceError) -> SessionsApplicationError {
    match error {
        WorkspaceError::Domain(error) => SessionsApplicationError::Validation(error.to_string()),
        WorkspaceError::Validation(message) => SessionsApplicationError::Validation(message),
        // The code, unwrapped. Sessions has no richer conflict of its own to map onto, and losing
        // it would turn a matchable refusal into an unexplained validation failure.
        WorkspaceError::Conflict(code) => SessionsApplicationError::Validation(code.to_string()),
        WorkspaceError::LaunchFailed(message) => SessionsApplicationError::WorkspaceLaunch(message),
        WorkspaceError::SessionNotFound(session_id) => {
            SessionsApplicationError::SessionNotFound(session_id)
        }
        WorkspaceError::PolicyDenied { session_id, action } => {
            SessionsApplicationError::Validation(format!(
                "Verifier session {session_id} cannot perform workspace action: {action}"
            ))
        }
        WorkspaceError::Repository(message)
        | WorkspaceError::Selection(message)
        | WorkspaceError::Filesystem(message)
        | WorkspaceError::Storage(message) => SessionsApplicationError::Workspace(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::application::ApplicationError;
    use chrono::{Datelike, Timelike};
    use std::sync::Mutex;

    #[derive(Default)]
    struct CapturingDiagnosticLog {
        entries: Mutex<Vec<DiagnosticLog>>,
    }

    impl DiagnosticLogPort for CapturingDiagnosticLog {
        fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), ApplicationError> {
            self.entries.lock().expect("diagnostics").push(log);
            Ok(())
        }
    }

    #[test]
    fn usage_ranges_start_at_local_calendar_midnight() {
        let now = Local
            .with_ymd_and_hms(2026, 7, 18, 15, 45, 30)
            .single()
            .expect("local fixture time");

        for (range, expected_date) in [
            (UsageStatisticsRange::Today, (2026, 7, 18)),
            (UsageStatisticsRange::Last7Days, (2026, 7, 12)),
            (UsageStatisticsRange::Last30Days, (2026, 6, 19)),
        ] {
            let start = usage_range_start_at(now, range)
                .expect("range start")
                .expect("bounded range");
            let local_start = DateTime::parse_from_rfc3339(&start)
                .expect("RFC3339 start")
                .with_timezone(&Local);

            assert_eq!(
                (local_start.year(), local_start.month(), local_start.day()),
                expected_date
            );
            assert_eq!(local_start.hour(), 0);
            assert_eq!(local_start.minute(), 0);
            assert_eq!(local_start.second(), 0);
        }
        assert_eq!(
            usage_range_start_at(now, UsageStatisticsRange::All).expect("all time"),
            None
        );
    }

    #[test]
    fn session_diagnostics_preserve_levels_and_context_through_the_unified_port() {
        let capture = Arc::new(CapturingDiagnosticLog::default());
        let adapter = UnifiedSessionLoggingAdapter::new(capture.clone());

        for (level, expected) in [
            (SessionApplicationLogLevel::Error, LogSeverity::Error),
            (SessionApplicationLogLevel::Warn, LogSeverity::Warn),
            (SessionApplicationLogLevel::Info, LogSeverity::Info),
            (SessionApplicationLogLevel::Debug, LogSeverity::Debug),
        ] {
            adapter
                .write(SessionApplicationLog {
                    level,
                    category: "session.fixture".to_string(),
                    message: "fixture".to_string(),
                    session_id: Some("session-1".to_string()),
                    operation_id: Some("operation-1".to_string()),
                    execution_run_id: Some("run-1".to_string()),
                    recovery_report_id: Some("report-1".to_string()),
                })
                .expect("session diagnostic");
            assert_eq!(
                capture
                    .entries
                    .lock()
                    .expect("diagnostics")
                    .last()
                    .unwrap()
                    .severity,
                expected
            );
        }

        let entries = capture.entries.lock().expect("diagnostics");
        assert_eq!(entries.len(), 4);
        assert!(entries.iter().all(|entry| {
            entry.category == "session.fixture"
                && entry.context.get("sessionId").map(String::as_str) == Some("session-1")
                && entry.context.get("operationId").map(String::as_str) == Some("operation-1")
        }));
    }

    #[test]
    fn export_failures_keep_diagnostics_detailed_and_command_errors_safe() {
        let capture = CapturingDiagnosticLog::default();
        let error = export_error(&capture, "private destination could not be written");

        assert!(matches!(
            error,
            SessionsApplicationError::FileContent(ref message) if message == "Session export failed"
        ));
        let entries = capture.entries.lock().expect("diagnostics");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].severity, LogSeverity::Error);
        assert_eq!(entries[0].category, "session.export");
        assert!(entries[0].message.contains("private destination"));
    }
}
