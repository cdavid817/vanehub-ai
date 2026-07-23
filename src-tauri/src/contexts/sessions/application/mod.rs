mod error;
mod models;
mod ports;
mod service;

pub(crate) use error::SessionsApplicationError;
pub(crate) use models::{
    ArchivalPolicy, CategoryRecord, ChatConfigurationValues, CompleteMessageRequest,
    CreateMessageRequest, CreatedSessionWorktree, EstimatedCharacterTotals, FailMessageRequest,
    FileReferenceInput, MessagePageQuery, MessageRecord, MessageTokenUsage, MessageUsageRecord,
    NewRemoteWorkspace, NewSessionRequest, NewSessionWorkspace, NewWorktree,
    PreparedNewSessionCreation, ReportedTokenTotals, RuntimeMessageSnapshot,
    RuntimeSessionSnapshot, SessionApplicationLog, SessionApplicationLogLevel,
    SessionChatConfiguration, SessionCreationOperation, SessionExportFormat, SessionExportRequest,
    SessionExportResult, SessionListScope, SessionMaintenanceResult, SessionProject, SessionRecord,
    SessionRemoteWorkspace, SessionSearchMatch, SessionSearchMatchKind, SessionSearchQuery,
    SessionSearchResult, SessionUsageAccountingKind, SessionUsageAgentBreakdown,
    SessionUsageCoverage, SessionUsagePoint, SessionUsageStatistics, SessionUsageSummary,
    SessionUsageUnit, SessionWorkspace, UsageStatisticsRange,
};
pub(crate) use ports::{
    SessionCategoryRepository, SessionChatProfilePort, SessionClockPort,
    SessionConfigurationRepository, SessionCreationContextPort, SessionFileContentPort,
    SessionIdentityPort, SessionLoggingPort, SessionMessageRepository, SessionOperationPort,
    SessionRepository, SessionRuntimePort, SessionTransactionPort, SessionUsageRepository,
};
pub(crate) use service::{SessionApplicationPorts, SessionsApplicationService};

#[cfg(test)]
mod tests;
