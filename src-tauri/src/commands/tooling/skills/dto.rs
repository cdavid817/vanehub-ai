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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillType {
    Role,
    Utility,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillDelivery {
    Eager,
    OnDemand,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillLayer {
    Project,
    User,
    Registry,
    System,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillOrigin {
    Created,
    Imported,
    Installed,
    Shipped,
    Migrated,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillTrust {
    Trusted,
    Untrusted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillAvailability {
    Available,
    Disabled,
    Invalid,
    Conflicting,
    Unsupported,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillCompatibilityDefaults {
    pub(crate) skill_type: bool,
    pub(crate) delivery: bool,
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
    #[serde(default)]
    pub(crate) aliases: Vec<String>,
    #[serde(rename = "type", default)]
    pub(crate) skill_type: Option<SkillType>,
    #[serde(default)]
    pub(crate) delivery: Option<SkillDelivery>,
    #[serde(default)]
    pub(crate) compatibility_defaults: SkillCompatibilityDefaults,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillShadowSummary {
    pub(crate) layer: SkillLayer,
    pub(crate) origin: SkillOrigin,
    pub(crate) version: String,
    pub(crate) availability: SkillAvailability,
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
    pub(crate) layer: SkillLayer,
    pub(crate) origin: SkillOrigin,
    pub(crate) trust: SkillTrust,
    pub(crate) availability: SkillAvailability,
    #[serde(default)]
    pub(crate) delegation_capability: SkillDelegationCapability,
    pub(crate) immutable: bool,
    pub(crate) shadowed_definitions: Vec<SkillShadowSummary>,
    pub(crate) usage: SkillUsageSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillDelegationCapability {
    /// Coarse legacy discriminant kept inside its existing frontend union
    /// (`available` / `not-utility` / `skill-unavailable`). `unavailable_reason` carries the
    /// specific, repairable cause.
    pub(crate) supported: bool,
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) unavailable_reason: Option<String>,
    #[serde(default)]
    pub(crate) declared_capabilities: Vec<String>,
    #[serde(default)]
    pub(crate) effective_capabilities: Vec<String>,
    #[serde(default)]
    pub(crate) requested_limits: SkillDelegationRequestedLimits,
    #[serde(default)]
    pub(crate) effective_limits: Option<SkillDelegationLimits>,
    #[serde(default)]
    pub(crate) capped_limits: Vec<String>,
    #[serde(default)]
    pub(crate) uses_platform_default: bool,
    #[serde(default)]
    pub(crate) read_only: bool,
    #[serde(default)]
    pub(crate) history: SkillDelegationHistorySummary,
}

impl Default for SkillDelegationCapability {
    fn default() -> Self {
        Self {
            supported: false,
            reason: "not-utility".to_string(),
            unavailable_reason: None,
            declared_capabilities: Vec::new(),
            effective_capabilities: Vec::new(),
            requested_limits: SkillDelegationRequestedLimits::default(),
            effective_limits: None,
            capped_limits: Vec::new(),
            uses_platform_default: false,
            read_only: false,
            history: SkillDelegationHistorySummary::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillDelegationRequestedLimits {
    pub(crate) max_rounds: Option<u16>,
    pub(crate) timeout_seconds: Option<u32>,
    pub(crate) max_context_chars: Option<u32>,
    pub(crate) max_output_chars: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillDelegationLimits {
    pub(crate) max_rounds: u16,
    pub(crate) timeout_seconds: u32,
    pub(crate) max_context_chars: u32,
    pub(crate) max_output_chars: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillDelegationHistorySummary {
    pub(crate) attempt_count: u64,
    pub(crate) last_attempt_at: Option<String>,
    pub(crate) last_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillUsageSummary {
    pub(crate) view_count: u64,
    pub(crate) use_count: u64,
    pub(crate) last_viewed_at: Option<String>,
    pub(crate) last_used_at: Option<String>,
    pub(crate) revision_witness: Option<String>,
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
    pub(crate) expected_content_hash: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SkillAgentKind {
    Cli,
    Api,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillCompatibleAgent {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) kind: SkillAgentKind,
    /// Whether this Agent can hold a delegated Utility assignment. CLI Agents and API runtimes
    /// without the native delegation tool stay listed so existing associations remain repairable.
    #[serde(default)]
    pub(crate) supports_utility_delegation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillOverview {
    pub(crate) skills: Vec<Skill>,
    pub(crate) stats: SkillStats,
    pub(crate) mount_paths: Vec<SkillAgentMountPath>,
    pub(crate) agents: Vec<SkillCompatibleAgent>,
    pub(crate) api_agent_bindings: std::collections::BTreeMap<String, Vec<String>>,
    pub(crate) drift: SkillDriftReport,
    pub(crate) restore_candidates: Vec<String>,
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
    pub(crate) layer: SkillLayer,
    pub(crate) origin: SkillOrigin,
    pub(crate) availability: SkillAvailability,
    pub(crate) immutable: bool,
    pub(crate) shadowed_definitions: Vec<SkillShadowSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillLoadInput {
    pub(crate) id_or_alias: String,
    pub(crate) workspace_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillResourceReadInput {
    pub(crate) uri: String,
    pub(crate) revision: String,
    pub(crate) workspace_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillResourceEntry {
    pub(crate) uri: String,
    pub(crate) relative_path: String,
    pub(crate) size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillResourceIndex {
    pub(crate) scripts: Vec<SkillResourceEntry>,
    pub(crate) references: Vec<SkillResourceEntry>,
    pub(crate) templates: Vec<SkillResourceEntry>,
    pub(crate) assets: Vec<SkillResourceEntry>,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillLoadResult {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) content: String,
    pub(crate) truncated: bool,
    pub(crate) revision: String,
    pub(crate) base_uri: String,
    pub(crate) resources: SkillResourceIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillAccessRefusal {
    pub(crate) requested: String,
    pub(crate) canonical_id: Option<String>,
    pub(crate) reason: String,
    pub(crate) conflicting_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub(crate) enum SkillLoadOutcome {
    Loaded { result: SkillLoadResult },
    Refused { refusal: SkillAccessRefusal },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillResourceReadResult {
    pub(crate) id: String,
    pub(crate) uri: String,
    pub(crate) revision: String,
    pub(crate) content: String,
    pub(crate) size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub(crate) enum SkillResourceReadOutcome {
    Read { result: SkillResourceReadResult },
    Refused { refusal: SkillAccessRefusal },
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
