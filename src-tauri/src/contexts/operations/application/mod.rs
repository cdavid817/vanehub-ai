mod error;
mod evidence;
mod log_cursor;
mod log_index;
mod log_index_ports;
mod log_query_service;
#[cfg(test)]
mod log_query_service_tests;
mod logging;
mod mission_control;
mod operation_service;
mod run_service;

pub(crate) use error::ApplicationError;
#[cfg(test)]
pub(crate) use evidence::NoOperationsEvidence;
pub(crate) use evidence::{OperationsEvidencePort, OperationsEvidenceSignal};
pub(crate) use log_cursor::{filter_fingerprint, LogPageCursor, LogSortDirection};
pub(crate) use log_index::{
    IndexedLogLevel, IndexedSessionLogDetail, IndexedSessionLogPage, IndexedSessionLogQuery,
    IndexedSessionLogRecord, LogCorrelation, OperationsLogError, SafeLogExportPreparation,
    SessionLogBackfillState, SessionLogBackfillStatus, SessionLogCoverage, SessionLogCoverageState,
    SessionLogFilters, SessionLogNotice, SessionLogNoticeKind, SessionLogQueryScope,
    SessionLogSubscriptionBootstrap, SessionLogSummary, DEFAULT_LOG_PAGE_SIZE, MAX_LOG_PAGE_SIZE,
    MAX_LOG_SEARCH_CANDIDATES,
};
pub(crate) use log_index_ports::{
    BackfillOperationPublisher, LogIndexClock, LogIndexDiagnostics, LogIndexIdGenerator,
    LogIndexInsertOutcome, LogSourceIdentity, PostCommitLogNoticePublisher, RedactedLogBatch,
    RedactedLogRecord, RedactedLogSourceReader, SessionLogIndexRepository,
};
pub(crate) use log_query_service::SessionLogQueryService;
pub(crate) use logging::{
    DiagnosticLog, DiagnosticLogPort, ExternalLogExportPort, LogSeverity, OperationLog,
    OperationLogPort,
};
pub(crate) use mission_control::{
    project as project_mission_control_run, MissionControlOverview, MissionControlQuery,
    MissionControlRepository, MissionControlRunDetail, MissionControlRunSummary,
    MissionControlService,
};
pub(crate) use operation_service::{
    OperationClock, OperationIdGenerator, OperationRepository, OperationService,
};
pub(crate) use run_service::{
    AgentRunRepository, AgentRunService, CreateAgentRun, RunClockPort, RunListFilter,
    RunOwnerRecoveryPort, RunPage, RunRecoveryDecision,
};
