mod error;
mod evidence;
mod models;
mod ports;
mod recovery_coordinator;
mod report;
mod review;
mod service;
mod usage_accounting;
mod usage_accounting_ports;

pub(crate) use error::SessionsApplicationError;
#[cfg(test)]
pub(crate) use evidence::NoSessionEvidence;
pub(crate) use evidence::{
    SessionEvidencePort, SessionEvidenceSignal, SessionReviewDecision, SessionUsageEvidenceQuality,
    SessionVerificationOutcome,
};
pub(crate) use models::{
    AcknowledgeRecoveryRequest, AcknowledgeRecoveryResult, ArchivalPolicy, CategoryRecord,
    ChatConfigurationValues, ClaimRecoveryCandidateRequest, CompleteMessageRequest,
    CreateMessageRequest, CreatedSessionWorktree, DurableGenerationStartRequest,
    DurableGenerationTerminalRequest, EstimatedCharacterTotals, FailMessageRequest,
    FileReferenceInput, GenerationStartRequest, GenerationStartResult, GenerationTerminalRequest,
    GenerationTerminalResult, GenerationTerminalStatus, LoopRoleSessionRequest,
    LoopSessionOwnership, MessagePageQuery, MessageRecord, MessageTokenUsage, MessageUsageRecord,
    NewRemoteWorkspace, NewSessionRequest, NewSessionWorkspace, NewWorktree,
    PreparedNewSessionCreation, PublishRecoveryRequest, RecoveryBatchResult,
    RecoveryCandidateClaim, ReportedTokenTotals, RuntimeMessageSnapshot, RuntimeSessionSnapshot,
    SessionApplicationLog, SessionApplicationLogLevel, SessionChatConfiguration,
    SessionCreationOperation, SessionExportFormat, SessionExportRequest, SessionExportResult,
    SessionListScope, SessionMaintenanceResult, SessionProject, SessionRecord,
    SessionRecoveryEvent, SessionRecoveryEventKind, SessionRecoveryProjection,
    SessionRecoverySummary, SessionRemoteWorkspace, SessionRunnerTarget, SessionSearchMatch,
    SessionSearchMatchKind, SessionSearchQuery, SessionSearchResult, SessionSshBinding,
    SessionSshProfile, SessionUsageAccountingKind, SessionUsageAgentBreakdown,
    SessionUsageCoverage, SessionUsagePoint, SessionUsageStatistics, SessionUsageSummary,
    SessionUsageUnit, SessionWorkspace, UpdateSessionSeatsRequest, UsageStatisticsRange,
};
pub(crate) use ports::{
    SessionAgentEligibilityPort, SessionCategoryRepository, SessionChatProfilePort,
    SessionClockPort, SessionConfigurationRepository, SessionCreationContextPort,
    SessionFileContentPort, SessionIdentityPort, SessionLoggingPort, SessionMessageRepository,
    SessionOperationPort, SessionRecoveryEventPort, SessionRecoveryReportRepository,
    SessionRepository, SessionRuntimePort, SessionTerminalEvidencePort, SessionTransactionPort,
    SessionUsageRepository, StreamTextField,
};
pub(crate) use recovery_coordinator::SessionRecoveryCoordinator;
pub(crate) use report::{
    AgentReportRow, ChangeSummary, ChangeSummaryPort, CommandReport, ExecutionEvidencePort,
    ExecutionEvidenceSummary, FailureReportRow, LogFailurePort, LogFailureSummary,
    ObservabilityTimingPort, ReportClock, ReportCoverage, ReportCoverageState, ReportEvidenceLink,
    ReportExportPort, ReportScope, ReportScopeRequest, ReportSectionCoverage, ReportSourceError,
    ReportSourceResult, ReportUsagePort, ReportUsageSummary, RunOutcomePort, RunOutcomeSummary,
    SessionRunReport, SessionRunReportService, TimingSummary, ToolReportRow, VerificationReport,
};
/// Exported for the boundary test that proves all three witnesses cross as one reason code. The
/// command layer matches the error, not the witness; 13.10 renders the distinction from state the
/// Review Center already holds, so nothing in production needs to name this yet.
#[cfg(test)]
pub(crate) use review::StaleReviewWitness;
pub(crate) use review::{
    AddReviewCommentRequest, CreateReviewRequest, PreparedReviewFeedback, ReviewAction,
    ReviewActionFindingInput, ReviewApplicationError, ReviewApplicationService, ReviewClockPort,
    ReviewDecisionRepository, ReviewFeedbackPort, ReviewHunkWitnessPort, ReviewIdPort,
    ReviewLogEvent, ReviewLoggingPort, ReviewOperationPort, ReviewRepository, ReviewSnapshotPort,
    ReviewSummary, ReviewView, SetFileViewedRequest, SetHunkDecisionRequest,
};
pub(crate) use service::{SessionApplicationPorts, SessionsApplicationService};
pub(crate) use usage_accounting::{
    CompletedInvocationAccounting, InvocationDetailQuery, ModelInvocationRecord,
    NewModelInvocation, NewUsageObservation, TokenUsageObservation, UsageAccountingSummary,
    UsageBreakdown, UsageBreakdownDimension, UsageBreakdownEntry, UsageCursor, UsageCursorAdvance,
    UsageDailyAggregate, UsageDetailPage, UsageEntityCounts, UsageMeasureAggregate,
    UsageQualityAggregate, UsageSummaryQuery,
};
pub(crate) use usage_accounting_ports::{
    TokenAccountingPort, TokenAccountingQueryPort, TokenAccountingRepository,
};

#[cfg(test)]
mod tests;
