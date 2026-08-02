mod error;
mod models;
mod ports;
mod service;

#[cfg(test)]
mod tests;

pub(crate) use error::SkillApplicationError;
pub(crate) use models::{
    AgentMountConfiguration, ManagedSkillSource, SkillAgentBinding, SkillAgentKind,
    SkillAgentMountPath, SkillBackupEntry, SkillCompatibleAgent, SkillCreateRequest, SkillDocument,
    SkillDriftReport, SkillFailure, SkillFilesystemTransaction, SkillImportRequest,
    SkillImportedSource, SkillListResult, SkillLogAction, SkillLogEvent, SkillLogLevel,
    SkillMountMigrationReport, SkillMountRepair, SkillOverview, SkillPreview, SkillPromptForAgent,
    SkillRecord, SkillScopeQuery, SkillSourceRefresh, SkillStats, SkillSyncResult,
    SkillUpdateRequest,
};
pub(crate) use ports::{
    SkillApiBindingRepository, SkillClockPort, SkillFilesystemPort, SkillLoggingPort,
    SkillRepository, SkillWorkspaceSelectionPort,
};
pub(crate) use service::SkillApplicationService;
