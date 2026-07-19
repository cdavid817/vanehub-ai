use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SkillScope {
    Global,
    Workspace,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SkillSource {
    Builtin,
    User,
    Imported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillScopeInput {
    pub(crate) scope: SkillScope,
    pub(crate) workspace_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillMetadata {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) category: String,
    pub(crate) version: String,
    pub(crate) triggers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillAgentBinding {
    pub(crate) agent_id: String,
    pub(crate) mount_path: String,
    pub(crate) mounted_path: String,
    pub(crate) mounted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Skill {
    pub(crate) id: String,
    pub(crate) scope: SkillScope,
    pub(crate) workspace_path: Option<String>,
    pub(crate) source: SkillSource,
    pub(crate) enabled: bool,
    pub(crate) skill_dir: String,
    pub(crate) skill_md_path: String,
    pub(crate) content_hash: String,
    pub(crate) metadata: SkillMetadata,
    pub(crate) bound_agent_ids: Vec<String>,
    pub(crate) bindings: Vec<SkillAgentBinding>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillStats {
    pub(crate) total: usize,
    pub(crate) enabled: usize,
    pub(crate) mounted: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillListResult {
    pub(crate) skills: Vec<Skill>,
    pub(crate) stats: SkillStats,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillAgentMountPath {
    pub(crate) agent_id: String,
    pub(crate) mount_path: String,
    pub(crate) is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillMutationInput {
    pub(crate) id: String,
    pub(crate) scope: SkillScope,
    pub(crate) workspace_path: Option<String>,
    pub(crate) metadata: SkillMetadata,
    pub(crate) body: String,
    pub(crate) enabled: bool,
    pub(crate) bound_agent_ids: Vec<String>,
    pub(crate) source: Option<SkillSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillUpdateInput {
    pub(crate) scope: SkillScope,
    pub(crate) workspace_path: Option<String>,
    pub(crate) metadata: SkillMetadata,
    pub(crate) body: String,
    pub(crate) enabled: bool,
    pub(crate) bound_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillImportInput {
    pub(crate) scope: SkillScope,
    pub(crate) workspace_path: Option<String>,
    pub(crate) source_path: String,
    pub(crate) enabled: bool,
    pub(crate) bound_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillPreview {
    pub(crate) id: String,
    pub(crate) scope: SkillScope,
    pub(crate) workspace_path: Option<String>,
    pub(crate) content: String,
    pub(crate) path: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillDriftIssueType {
    MissingSource,
    MetadataChanged,
    UnregisteredSource,
    MissingMount,
    Conflict,
    DeletedBuiltin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillDriftIssue {
    pub(crate) skill_id: String,
    pub(crate) r#type: SkillDriftIssueType,
    pub(crate) agent_id: Option<String>,
    pub(crate) path: Option<String>,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillDriftReport {
    pub(crate) scope: SkillScope,
    pub(crate) workspace_path: Option<String>,
    pub(crate) issues: Vec<SkillDriftIssue>,
    pub(crate) drift_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillBackupEntry {
    pub(crate) original_path: String,
    pub(crate) backup_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillFailure {
    pub(crate) skill_id: String,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillSyncResult {
    pub(crate) mounted: Vec<String>,
    pub(crate) unmounted: Vec<String>,
    pub(crate) overwritten: Vec<String>,
    pub(crate) backed_up: Vec<SkillBackupEntry>,
    pub(crate) restored: Vec<String>,
    pub(crate) failed: Vec<SkillFailure>,
    pub(crate) resolved_from: SkillDriftReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillMountMigrationReport {
    pub(crate) agent_id: String,
    pub(crate) old_mount_path: String,
    pub(crate) new_mount_path: String,
    pub(crate) migrated: Vec<String>,
    pub(crate) removed: Vec<String>,
    pub(crate) overwritten: Vec<String>,
    pub(crate) backed_up: Vec<SkillBackupEntry>,
    pub(crate) failed: Vec<SkillFailure>,
}
