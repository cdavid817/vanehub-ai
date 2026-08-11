mod error;
mod models;
mod ports;
mod recovery_coordinator;
mod service;

pub(crate) use error::SessionsApplicationError;
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
    SessionRecoverySummary, SessionRemoteWorkspace, SessionSearchMatch, SessionSearchMatchKind,
    SessionSearchQuery, SessionSearchResult, SessionSshBinding, SessionSshProfile,
    SessionUsageAccountingKind, SessionUsageAgentBreakdown, SessionUsageCoverage,
    SessionUsagePoint, SessionUsageStatistics, SessionUsageSummary, SessionUsageUnit,
    SessionWorkspace, UpdateSessionSeatsRequest, UsageStatisticsRange,
};
pub(crate) use ports::{
    SessionAgentEligibilityPort, SessionCategoryRepository, SessionChatProfilePort,
    SessionClockPort, SessionConfigurationRepository, SessionCreationContextPort,
    SessionFileContentPort, SessionIdentityPort, SessionLoggingPort, SessionMessageRepository,
    SessionOperationPort, SessionRecoveryEventPort, SessionRecoveryReportRepository,
    SessionRepository, SessionRuntimePort, SessionTerminalEvidencePort, SessionTransactionPort,
    SessionUsageRepository,
};
pub(crate) use recovery_coordinator::SessionRecoveryCoordinator;
pub(crate) use service::{SessionApplicationPorts, SessionsApplicationService};

#[cfg(test)]
mod tests;
