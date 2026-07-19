use crate::contexts::tooling::skills::domain::{
    SkillDriftIssue, SkillId, SkillKey, SkillLocation, SkillMetadata, SkillMountPath, SkillSource,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillScopeQuery {
    pub(crate) location: SkillLocation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillDocument {
    pub(crate) metadata: SkillMetadata,
    pub(crate) body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManagedSkillSource {
    pub(crate) skill_dir: String,
    pub(crate) skill_md_path: String,
    pub(crate) content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillImportedSource {
    pub(crate) metadata: SkillMetadata,
    pub(crate) source: ManagedSkillSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillAgentBinding {
    pub(crate) agent_id: String,
    pub(crate) mount_path: SkillMountPath,
    pub(crate) mounted_path: String,
    pub(crate) mounted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillRecord {
    pub(crate) key: SkillKey,
    pub(crate) source: SkillSource,
    pub(crate) enabled: bool,
    pub(crate) managed_source: ManagedSkillSource,
    pub(crate) metadata: SkillMetadata,
    pub(crate) bindings: Vec<SkillAgentBinding>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl SkillRecord {
    pub(crate) fn bound_agent_ids(&self) -> Vec<String> {
        self.bindings
            .iter()
            .map(|binding| binding.agent_id.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SkillStats {
    pub(crate) total: usize,
    pub(crate) enabled: usize,
    pub(crate) mounted: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillListResult {
    pub(crate) skills: Vec<SkillRecord>,
    pub(crate) stats: SkillStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentMountConfiguration {
    pub(crate) agent_id: String,
    pub(crate) configured_path: Option<SkillMountPath>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillAgentMountPath {
    pub(crate) agent_id: String,
    pub(crate) mount_path: SkillMountPath,
    pub(crate) is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillCreateRequest {
    pub(crate) id: SkillId,
    pub(crate) location: SkillLocation,
    pub(crate) metadata: SkillMetadata,
    pub(crate) body: String,
    pub(crate) enabled: bool,
    pub(crate) bound_agent_ids: Vec<String>,
    pub(crate) source: Option<SkillSource>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillUpdateRequest {
    pub(crate) key: SkillKey,
    pub(crate) metadata: SkillMetadata,
    pub(crate) body: String,
    pub(crate) enabled: bool,
    pub(crate) bound_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillImportRequest {
    pub(crate) location: SkillLocation,
    pub(crate) source_path: String,
    pub(crate) enabled: bool,
    pub(crate) bound_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillPreview {
    pub(crate) key: SkillKey,
    pub(crate) content: String,
    pub(crate) path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillDriftReport {
    pub(crate) location: SkillLocation,
    pub(crate) issues: Vec<SkillDriftIssue>,
    pub(crate) drift_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillBackupEntry {
    pub(crate) original_path: String,
    pub(crate) backup_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillFailure {
    pub(crate) skill_id: String,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillSyncResult {
    pub(crate) mounted: Vec<String>,
    pub(crate) unmounted: Vec<String>,
    pub(crate) overwritten: Vec<String>,
    pub(crate) backed_up: Vec<SkillBackupEntry>,
    pub(crate) restored: Vec<String>,
    pub(crate) failed: Vec<SkillFailure>,
    pub(crate) resolved_from: SkillDriftReport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillMountMigrationReport {
    pub(crate) agent_id: String,
    pub(crate) old_mount_path: SkillMountPath,
    pub(crate) new_mount_path: SkillMountPath,
    pub(crate) migrated: Vec<String>,
    pub(crate) removed: Vec<String>,
    pub(crate) overwritten: Vec<String>,
    pub(crate) backed_up: Vec<SkillBackupEntry>,
    pub(crate) failed: Vec<SkillFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillFilesystemTransaction {
    pub(crate) id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillMountRepair {
    pub(crate) binding: SkillAgentBinding,
    pub(crate) removed_path: Option<String>,
    pub(crate) overwritten: Vec<String>,
    pub(crate) backed_up: Vec<SkillBackupEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillSourceRefresh {
    pub(crate) metadata: SkillMetadata,
    pub(crate) content_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillLogAction {
    SeedBuiltins,
    UpdateMountPath,
    Create,
    Update,
    Delete,
    Restore,
    SetEnabled,
    SetBindings,
    Import,
    DetectDrift,
    SyncDrift,
}

impl SkillLogAction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::SeedBuiltins => "seed-builtins",
            Self::UpdateMountPath => "update-mount-path",
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::Restore => "restore",
            Self::SetEnabled => "set-enabled",
            Self::SetBindings => "set-bindings",
            Self::Import => "import",
            Self::DetectDrift => "detect-drift",
            Self::SyncDrift => "sync-drift",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillLogLevel {
    Error,
    Warn,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillLogEvent {
    pub(crate) action: SkillLogAction,
    pub(crate) level: SkillLogLevel,
    pub(crate) skill_id: Option<String>,
    pub(crate) message: String,
    pub(crate) timestamp: String,
    pub(crate) context: BTreeMap<String, String>,
}
