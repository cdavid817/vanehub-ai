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
    pub(crate) supported: bool,
    pub(crate) reason: String,
}

impl Default for SkillDelegationCapability {
    fn default() -> Self {
        Self {
            supported: false,
            reason: "not-utility".to_string(),
        }
    }
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SkillConfigurationScope {
    User,
    Project,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillConfigurationProvenance {
    Project,
    User,
    Default,
    Missing,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillConfigurationSecretState {
    Configured,
    Missing,
    Error,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillConfigurationDrift {
    Compatible,
    MigrationRequired,
    Invalid,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillConfigurationReadiness {
    Ready,
    MissingRequired,
    MigrationRequired,
    Invalid,
    NotConfigurable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillConfigurableState {
    NotEvaluated,
    NotConfigurable,
    Configurable,
    SchemaUnsupported,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillConfigurationCleanupState {
    None,
    Pending,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillConfigurationConsumption {
    Native,
    UnsupportedExternalCli,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SkillConfigurationScalarType {
    String,
    Integer,
    Number,
    Boolean,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SkillConfigurationFieldKind {
    Scalar,
    List,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationFieldType {
    pub(crate) kind: SkillConfigurationFieldKind,
    pub(crate) scalar: SkillConfigurationScalarType,
}

/// Tagged rather than a bare JSON value: `3` and `3.0` are the same JSON number, and a stored
/// integer that a later schema retypes to a number has to stay distinguishable from one that was
/// always a number, or drift classification silently accepts it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "lowercase")]
pub(crate) enum SkillConfigurationValue {
    String(String),
    Integer(i64),
    Number(f64),
    Boolean(bool),
    List(Vec<SkillConfigurationValue>),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationValueEntry {
    pub(crate) key: String,
    pub(crate) value: SkillConfigurationValue,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationConstraints {
    pub(crate) minimum: Option<f64>,
    pub(crate) maximum: Option<f64>,
    pub(crate) min_length: Option<usize>,
    pub(crate) max_length: Option<usize>,
    pub(crate) min_items: Option<usize>,
    pub(crate) max_items: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationPresentation {
    pub(crate) label: Option<String>,
    pub(crate) label_key: Option<String>,
    pub(crate) help: Option<String>,
    pub(crate) order: Option<u32>,
    pub(crate) advanced: bool,
    pub(crate) multiline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationGroupDescriptor {
    pub(crate) key: String,
    pub(crate) declared_key: String,
    pub(crate) presentation: SkillConfigurationPresentation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationFieldDescriptor {
    pub(crate) key: String,
    pub(crate) declared_key: String,
    pub(crate) group: Option<String>,
    pub(crate) field_type: SkillConfigurationFieldType,
    pub(crate) required: bool,
    pub(crate) secret: bool,
    pub(crate) default_value: Option<SkillConfigurationValue>,
    pub(crate) choices: Vec<SkillConfigurationValue>,
    pub(crate) constraints: SkillConfigurationConstraints,
    pub(crate) presentation: SkillConfigurationPresentation,
    pub(crate) description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationDescriptor {
    pub(crate) schema_hash: String,
    pub(crate) groups: Vec<SkillConfigurationGroupDescriptor>,
    pub(crate) fields: Vec<SkillConfigurationFieldDescriptor>,
}

/// A secret property reports presence and nothing else; `value` stays null for it even when the
/// credential exists, so no response path can carry the secret or its credential alias.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationProperty {
    pub(crate) key: String,
    pub(crate) value: Option<SkillConfigurationValue>,
    pub(crate) provenance: SkillConfigurationProvenance,
    pub(crate) secret: bool,
    pub(crate) secret_state: Option<SkillConfigurationSecretState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationScopeState {
    pub(crate) scope: SkillConfigurationScope,
    pub(crate) stored_revision: u64,
    pub(crate) schema_hash: String,
    pub(crate) drift: SkillConfigurationDrift,
    pub(crate) configured_property_count: usize,
    pub(crate) configured_secret_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationResolution {
    pub(crate) properties: Vec<SkillConfigurationProperty>,
    pub(crate) drift: SkillConfigurationDrift,
    pub(crate) readiness: SkillConfigurationReadiness,
    pub(crate) scopes: Vec<SkillConfigurationScopeState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationRecordState {
    pub(crate) scope: SkillConfigurationScope,
    pub(crate) workspace_identity: Option<String>,
    pub(crate) schema_hash: String,
    pub(crate) base_revision: String,
    pub(crate) stored_revision: u64,
    pub(crate) validation_state: SkillConfigurationDrift,
    pub(crate) values: Vec<SkillConfigurationValueEntry>,
    pub(crate) secret_keys: Vec<String>,
    pub(crate) orphaned_at: Option<String>,
    pub(crate) cleanup_state: SkillConfigurationCleanupState,
}

/// Stated per binding kind because there is no bridge: the UI has to be able to say that an
/// external CLI receives the package without values instead of letting a user assume otherwise.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationRuntimeSupport {
    pub(crate) native_api: SkillConfigurationConsumption,
    pub(crate) external_cli: SkillConfigurationConsumption,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationView {
    pub(crate) skill_id: String,
    pub(crate) state: SkillConfigurableState,
    pub(crate) unsupported_reason: Option<String>,
    pub(crate) schema_hash: Option<String>,
    pub(crate) base_revision: Option<String>,
    pub(crate) workspace_identity: Option<String>,
    pub(crate) available_scopes: Vec<SkillConfigurationScope>,
    pub(crate) descriptor: Option<SkillConfigurationDescriptor>,
    pub(crate) resolution: Option<SkillConfigurationResolution>,
    pub(crate) readiness: SkillConfigurationReadiness,
    pub(crate) drift: Option<SkillConfigurationDrift>,
    pub(crate) runtime: SkillConfigurationRuntimeSupport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum SkillConfigurationRecovery {
    Clean,
    /// Property names only. They are already public in the Skill package; values and credential
    /// aliases are not, and a cleanup report is exactly where one would otherwise leak.
    Incomplete {
        properties: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillConfigurationRejectionCode {
    UnknownProperty,
    NotConfigurable,
    InvalidValue,
    PayloadTooLarge,
    SchemaChanged,
    BaseRevisionChanged,
    Stale,
    InvalidWorkspace,
    CredentialFailure,
    RepositoryFailure,
    RecoveryRequired,
    ReconciliationIncomplete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationRejection {
    pub(crate) code: SkillConfigurationRejectionCode,
    pub(crate) message: String,
    pub(crate) property_key: Option<String>,
    pub(crate) properties: Vec<String>,
    pub(crate) expected: Option<String>,
    pub(crate) actual: Option<String>,
    pub(crate) bytes: Option<usize>,
    /// Present when the write lost a compare-and-swap, so an editor can refresh from the record
    /// that won instead of issuing another read that could race again.
    pub(crate) current: Option<SkillConfigurationRecordState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationSaveResult {
    pub(crate) record: SkillConfigurationRecordState,
    pub(crate) preview: SkillConfigurationResolution,
    pub(crate) recovery: SkillConfigurationRecovery,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum SkillConfigurationSaveOutcome {
    Saved {
        result: Box<SkillConfigurationSaveResult>,
    },
    Rejected {
        rejection: Box<SkillConfigurationRejection>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum SkillConfigurationResolutionOutcome {
    Resolved {
        resolution: SkillConfigurationResolution,
    },
    Rejected {
        rejection: Box<SkillConfigurationRejection>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "camelCase")]
pub(crate) enum SkillConfigurationRetentionOutcome {
    Applied {
        recovery: SkillConfigurationRecovery,
    },
    Rejected {
        rejection: Box<SkillConfigurationRejection>,
    },
}

/// Inbound only, and deliberately not `Serialize`: a replacement value must be able to enter but
/// must have no way back out through a response.
#[derive(Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "camelCase")]
pub(crate) enum SkillConfigurationSecretIntent {
    Preserve,
    Replace { value: String },
    Clear,
}

impl std::fmt::Debug for SkillConfigurationSecretIntent {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Preserve => "Preserve",
            Self::Replace { .. } => "Replace(<redacted>)",
            Self::Clear => "Clear",
        })
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationSecretIntentInput {
    pub(crate) key: String,
    pub(crate) intent: SkillConfigurationSecretIntent,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationWriteInput {
    pub(crate) scope: SkillScope,
    pub(crate) workspace_path: Option<String>,
    pub(crate) config_scope: SkillConfigurationScope,
    pub(crate) schema_hash: String,
    pub(crate) base_revision: String,
    /// `null` asserts that the scope has no stored record yet; any mismatch is stale.
    pub(crate) expected_revision: Option<u64>,
    #[serde(default)]
    pub(crate) values: Vec<SkillConfigurationValueEntry>,
    #[serde(default)]
    pub(crate) secret_intents: Vec<SkillConfigurationSecretIntentInput>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationScopeInput {
    pub(crate) scope: SkillScope,
    pub(crate) workspace_path: Option<String>,
    pub(crate) config_scope: SkillConfigurationScope,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SkillConfigurationRetention {
    Retain,
    Delete,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "camelCase")]
pub(crate) enum SkillConfigurationObsoleteAction {
    MapTo { target_key: String },
    Discard,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationObsoleteFieldChoice {
    pub(crate) key: String,
    pub(crate) choice: SkillConfigurationObsoleteAction,
}

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationReconciliationPlan {
    #[serde(default)]
    pub(crate) fields: Vec<SkillConfigurationObsoleteFieldChoice>,
    #[serde(default)]
    pub(crate) clear_secrets: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillConfigurationReconciliationInput {
    pub(crate) request: SkillConfigurationWriteInput,
    #[serde(default)]
    pub(crate) plan: SkillConfigurationReconciliationPlan,
}
