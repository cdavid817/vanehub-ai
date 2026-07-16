use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SkillScope {
    Global,
    Workspace,
}

impl SkillScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Workspace => "workspace",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "global" => Some(Self::Global),
            "workspace" => Some(Self::Workspace),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SkillSource {
    Builtin,
    User,
    Imported,
}

impl SkillSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::User => "user",
            Self::Imported => "imported",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "builtin" => Some(Self::Builtin),
            "user" => Some(Self::User),
            "imported" => Some(Self::Imported),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillScopeInput {
    pub scope: SkillScope,
    pub workspace_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub version: String,
    pub triggers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SkillAgentBinding {
    pub agent_id: String,
    pub mount_path: String,
    pub mounted_path: String,
    pub mounted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub id: String,
    pub scope: SkillScope,
    pub workspace_path: Option<String>,
    pub source: SkillSource,
    pub enabled: bool,
    pub skill_dir: String,
    pub skill_md_path: String,
    pub content_hash: String,
    pub metadata: SkillMetadata,
    pub bound_agent_ids: Vec<String>,
    pub bindings: Vec<SkillAgentBinding>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillStats {
    pub total: usize,
    pub enabled: usize,
    pub mounted: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillListResult {
    pub skills: Vec<Skill>,
    pub stats: SkillStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillAgentMountPath {
    pub agent_id: String,
    pub mount_path: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillMutationInput {
    pub id: String,
    pub scope: SkillScope,
    pub workspace_path: Option<String>,
    pub metadata: SkillMetadata,
    pub body: String,
    pub enabled: bool,
    pub bound_agent_ids: Vec<String>,
    pub source: Option<SkillSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillUpdateInput {
    pub scope: SkillScope,
    pub workspace_path: Option<String>,
    pub metadata: SkillMetadata,
    pub body: String,
    pub enabled: bool,
    pub bound_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillImportInput {
    pub scope: SkillScope,
    pub workspace_path: Option<String>,
    pub source_path: String,
    pub enabled: bool,
    pub bound_agent_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillPreview {
    pub id: String,
    pub scope: SkillScope,
    pub workspace_path: Option<String>,
    pub content: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SkillDriftIssueType {
    MissingSource,
    MetadataChanged,
    UnregisteredSource,
    MissingMount,
    Conflict,
    DeletedBuiltin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDriftIssue {
    pub skill_id: String,
    pub r#type: SkillDriftIssueType,
    pub agent_id: Option<String>,
    pub path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDriftReport {
    pub scope: SkillScope,
    pub workspace_path: Option<String>,
    pub issues: Vec<SkillDriftIssue>,
    pub drift_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillBackupEntry {
    pub original_path: String,
    pub backup_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSyncResult {
    pub mounted: Vec<String>,
    pub unmounted: Vec<String>,
    pub overwritten: Vec<String>,
    pub backed_up: Vec<SkillBackupEntry>,
    pub restored: Vec<String>,
    pub failed: Vec<SkillFailure>,
    pub resolved_from: SkillDriftReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillFailure {
    pub skill_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillMountMigrationReport {
    pub agent_id: String,
    pub old_mount_path: String,
    pub new_mount_path: String,
    pub migrated: Vec<String>,
    pub removed: Vec<String>,
    pub overwritten: Vec<String>,
    pub backed_up: Vec<SkillBackupEntry>,
    pub failed: Vec<SkillFailure>,
}
