use super::config_overview::SkillConfigurationOverview;
use crate::contexts::tooling::skill_tools::api::SkillToolInventorySummary;
use crate::contexts::tooling::skills::domain::{
    SkillAvailability, SkillCompatibilityDefaults, SkillDelivery, SkillDriftIssue, SkillId,
    SkillKey, SkillLayer, SkillLocation, SkillMetadata, SkillMountPath, SkillOrigin, SkillSource,
    SkillTrust, SkillType,
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
pub(crate) struct SkillPackageDescriptor {
    pub(crate) package_key: String,
    pub(crate) workspace_path: Option<String>,
    pub(crate) metadata: SkillMetadata,
    pub(crate) layer: SkillLayer,
    pub(crate) origin: SkillOrigin,
    pub(crate) trust: SkillTrust,
    pub(crate) availability: SkillAvailability,
    pub(crate) revision: String,
    pub(crate) source_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectiveSkill {
    pub(crate) effective: SkillPackageDescriptor,
    pub(crate) shadowed: Vec<SkillPackageDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillPackagePreview {
    pub(crate) id: SkillId,
    pub(crate) document: SkillDocument,
    pub(crate) layer: SkillLayer,
    pub(crate) revision: String,
    pub(crate) immutable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillPackageResource {
    pub(crate) relative_path: String,
    pub(crate) media_type: String,
    pub(crate) size_bytes: u64,
    pub(crate) content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillResourceDocument {
    pub(crate) content: String,
    pub(crate) size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SkillDiscoveryRequest {
    pub(crate) workspace_path: Option<String>,
    pub(crate) query: Option<String>,
    pub(crate) skill_type: Option<SkillType>,
    pub(crate) delivery: Option<SkillDelivery>,
    pub(crate) availability: Option<SkillAvailability>,
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillDiscoveryEntry {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) aliases: Vec<String>,
    pub(crate) skill_type: SkillType,
    pub(crate) delivery: SkillDelivery,
    pub(crate) layer: SkillLayer,
    pub(crate) availability: SkillAvailability,
    pub(crate) version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SkillDiscoveryResult {
    pub(crate) skills: Vec<SkillDiscoveryEntry>,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillLoadRequest {
    pub(crate) id_or_alias: String,
    pub(crate) workspace_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillResourceReadRequest {
    pub(crate) uri: String,
    pub(crate) revision: String,
    pub(crate) workspace_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillResourceEntry {
    pub(crate) uri: String,
    pub(crate) relative_path: String,
    pub(crate) size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SkillResourceIndex {
    pub(crate) scripts: Vec<SkillResourceEntry>,
    pub(crate) references: Vec<SkillResourceEntry>,
    pub(crate) templates: Vec<SkillResourceEntry>,
    pub(crate) assets: Vec<SkillResourceEntry>,
    pub(crate) truncated: bool,
}

impl SkillResourceIndex {
    pub(crate) fn entry(&self, relative_path: &str) -> Option<&SkillResourceEntry> {
        self.scripts
            .iter()
            .chain(&self.references)
            .chain(&self.templates)
            .chain(&self.assets)
            .find(|entry| entry.relative_path == relative_path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillLoadResult {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) content: String,
    pub(crate) truncated: bool,
    pub(crate) revision: String,
    pub(crate) base_uri: String,
    pub(crate) resources: SkillResourceIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillAccessRefusalReason {
    NotFound,
    AmbiguousAlias,
    Disabled,
    Invalid,
    Conflicting,
    Unsupported,
    UtilityNotLoadable,
    InvalidUri,
    UnindexedResource,
    StaleRevision,
    EscapingResource,
    BinaryResource,
    OversizedResource,
    Unreadable,
}

impl SkillAccessRefusalReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NotFound => "not-found",
            Self::AmbiguousAlias => "ambiguous-alias",
            Self::Disabled => "disabled",
            Self::Invalid => "invalid",
            Self::Conflicting => "conflicting",
            Self::Unsupported => "unsupported",
            Self::UtilityNotLoadable => "utility-not-loadable",
            Self::InvalidUri => "invalid-uri",
            Self::UnindexedResource => "unindexed-resource",
            Self::StaleRevision => "stale-revision",
            Self::EscapingResource => "escaping-resource",
            Self::BinaryResource => "binary-resource",
            Self::OversizedResource => "oversized-resource",
            Self::Unreadable => "unreadable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillAccessRefusal {
    pub(crate) requested: String,
    pub(crate) canonical_id: Option<String>,
    pub(crate) reason: SkillAccessRefusalReason,
    pub(crate) conflicting_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillLoadOutcome {
    Loaded(SkillLoadResult),
    Refused(SkillAccessRefusal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillResourceReadResult {
    pub(crate) id: String,
    pub(crate) uri: String,
    pub(crate) revision: String,
    pub(crate) content: String,
    pub(crate) size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillResourceReadOutcome {
    Read(SkillResourceReadResult),
    Refused(SkillAccessRefusal),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct EffectiveCatalogCacheKey {
    pub(crate) workspace_path: Option<String>,
    pub(crate) inventory_fingerprint: String,
    pub(crate) state_revision: u64,
}

pub(crate) const BUILTIN_RECONCILIATION_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BuiltinReconciliationOutcome {
    System,
    MigratedOverride,
    Deleted,
    Invalid,
}

impl BuiltinReconciliationOutcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::MigratedOverride => "migrated-override",
            Self::Deleted => "deleted",
            Self::Invalid => "invalid",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "system" => Some(Self::System),
            "migrated-override" => Some(Self::MigratedOverride),
            "deleted" => Some(Self::Deleted),
            "invalid" => Some(Self::Invalid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BuiltinCleanupStatus {
    NotRequired,
    Pending,
    Complete,
}

impl BuiltinCleanupStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NotRequired => "not-required",
            Self::Pending => "pending",
            Self::Complete => "complete",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "not-required" => Some(Self::NotRequired),
            "pending" => Some(Self::Pending),
            "complete" => Some(Self::Complete),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuiltinReconciliationState {
    pub(crate) skill_id: SkillId,
    pub(crate) reconciliation_version: u32,
    pub(crate) outcome: BuiltinReconciliationOutcome,
    pub(crate) system_revision: String,
    pub(crate) legacy_revision: Option<String>,
    pub(crate) cleanup_status: BuiltinCleanupStatus,
    pub(crate) backup_path: Option<String>,
    pub(crate) error_code: Option<String>,
    pub(crate) enabled: bool,
    pub(crate) deletion_intent: bool,
    pub(crate) effective_layer: SkillLayer,
    pub(crate) origin: SkillOrigin,
    pub(crate) availability: SkillAvailability,
    pub(crate) updated_at: String,
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
    pub(crate) resolved_metadata: Option<SkillEffectiveMetadata>,
}

impl SkillRecord {
    pub(crate) fn bound_agent_ids(&self) -> Vec<String> {
        self.bindings
            .iter()
            .map(|binding| binding.agent_id.clone())
            .collect()
    }

    pub(crate) fn effective_metadata(&self) -> SkillEffectiveMetadata {
        if let Some(metadata) = &self.resolved_metadata {
            return metadata.clone();
        }
        let layer = match self.key.location.scope {
            crate::contexts::tooling::skills::domain::SkillScope::Workspace => SkillLayer::Project,
            crate::contexts::tooling::skills::domain::SkillScope::Global => match self.source {
                SkillSource::Builtin => SkillLayer::System,
                SkillSource::User | SkillSource::Imported => SkillLayer::User,
            },
        };
        let origin = match self.source {
            SkillSource::Builtin => SkillOrigin::Shipped,
            SkillSource::User => SkillOrigin::Created,
            SkillSource::Imported => SkillOrigin::Imported,
        };
        let trust = if self.source == SkillSource::Imported {
            SkillTrust::Untrusted
        } else {
            SkillTrust::Trusted
        };
        let availability = if !self.enabled {
            SkillAvailability::Disabled
        } else if self.metadata.skill_type == SkillType::Utility {
            SkillAvailability::Unsupported
        } else {
            SkillAvailability::Available
        };
        SkillEffectiveMetadata {
            layer,
            origin,
            trust,
            availability,
            skill_type: self.metadata.skill_type,
            delivery: self.metadata.delivery,
            compatibility_defaults: self.metadata.compatibility_defaults,
            immutable: layer == SkillLayer::System,
            shadowed: Vec::new(),
            usage: SkillUsageSummary::default(),
            // This path runs on a record rebuilt from the registry, whose metadata carries no
            // frontmatter block. Reporting not-configurable here would hide a configurable
            // Skill's settings, so the question stays unanswered until the effective package
            // resolves it.
            configuration: SkillConfigurationOverview::not_evaluated(),
            tools: SkillToolInventorySummary::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillShadowSummary {
    pub(crate) layer: SkillLayer,
    pub(crate) origin: SkillOrigin,
    pub(crate) version: String,
    pub(crate) availability: SkillAvailability,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillEffectiveMetadata {
    pub(crate) layer: SkillLayer,
    pub(crate) origin: SkillOrigin,
    pub(crate) trust: SkillTrust,
    pub(crate) availability: SkillAvailability,
    pub(crate) skill_type: SkillType,
    pub(crate) delivery: SkillDelivery,
    pub(crate) compatibility_defaults: SkillCompatibilityDefaults,
    pub(crate) immutable: bool,
    pub(crate) shadowed: Vec<SkillShadowSummary>,
    pub(crate) usage: SkillUsageSummary,
    pub(crate) configuration: SkillConfigurationOverview,
    /// Bounded Skill tool inventory for this effective revision.
    ///
    /// Empty for every Skill that ships no tool manifest, which is what keeps existing Skills
    /// behaving exactly as before. Carries integrity hashes and governance status but no
    /// executable bytes: `skill_tools` owns reading and running the implementations, and this
    /// projection exists so the Skill overview can report them without that ability.
    pub(crate) tools: SkillToolInventorySummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SkillUsageSummary {
    pub(crate) view_count: u64,
    pub(crate) use_count: u64,
    pub(crate) last_viewed_at: Option<String>,
    pub(crate) last_used_at: Option<String>,
    pub(crate) revision_witness: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SkillUsageIdentity {
    pub(crate) id: SkillId,
    pub(crate) layer: SkillLayer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillUsageActivity {
    View,
    Use,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SkillUsageRead {
    pub(crate) summaries: BTreeMap<SkillUsageIdentity, SkillUsageSummary>,
    pub(crate) recovered_corrupt_state: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillUsageMutation {
    pub(crate) summary: SkillUsageSummary,
    pub(crate) recovered_corrupt_state: bool,
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
    pub(crate) expected_content_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillAgentKind {
    Cli,
    Api,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillCompatibleAgent {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) kind: SkillAgentKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillOverview {
    pub(crate) skills: Vec<SkillRecord>,
    pub(crate) stats: SkillStats,
    pub(crate) mount_paths: Vec<SkillAgentMountPath>,
    pub(crate) agents: Vec<SkillCompatibleAgent>,
    pub(crate) api_agent_bindings: BTreeMap<String, Vec<String>>,
    pub(crate) drift: SkillDriftReport,
    pub(crate) restore_candidates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillImportRequest {
    pub(crate) location: SkillLocation,
    pub(crate) source_path: String,
    pub(crate) enabled: bool,
    pub(crate) bound_agent_ids: Vec<String>,
}

/// A bound Skill's content, resolved and ready to inject as an API Agent's system prompt
/// (`add-agent-skill-support`) — `name` from metadata, `body` from the Skill's source file with
/// its frontmatter stripped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillPromptForAgent {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) body: String,
    pub(crate) revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UtilitySkillExecutionSnapshot {
    pub(crate) id: String,
    pub(crate) revision: String,
    pub(crate) instructions: String,
    pub(crate) workspace_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum UtilitySkillResolutionOutcome {
    Resolved(UtilitySkillExecutionSnapshot),
    Refused(SkillAccessRefusal),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillPreview {
    pub(crate) key: SkillKey,
    pub(crate) content: String,
    pub(crate) path: String,
    pub(crate) effective: SkillEffectiveMetadata,
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
    BindCliAgent,
    UnbindCliAgent,
    SetApiAgentBinding,
    ResolveApiPrompt,
    ListAgentSkills,
    LoadSkill,
    ReadSkillResource,
    TrackView,
    TrackUse,
    Import,
    DetectDrift,
    SyncDrift,
    OverlayValidation,
    ConfigurationValidate,
    ConfigurationSave,
    ConfigurationReset,
    ConfigurationSecretMutation,
    ConfigurationDrift,
    ConfigurationReconcile,
    ConfigurationDelete,
    ConfigurationRetention,
    ConfigurationCleanup,
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
            Self::BindCliAgent => "bind-cli-agent",
            Self::UnbindCliAgent => "unbind-cli-agent",
            Self::SetApiAgentBinding => "set-api-agent-binding",
            Self::ResolveApiPrompt => "resolve-api-prompt",
            Self::ListAgentSkills => "list-agent-skills",
            Self::LoadSkill => "load-skill",
            Self::ReadSkillResource => "read-skill-resource",
            Self::TrackView => "track-view",
            Self::TrackUse => "track-use",
            Self::Import => "import",
            Self::DetectDrift => "detect-drift",
            Self::SyncDrift => "sync-drift",
            Self::OverlayValidation => "overlay-validation",
            Self::ConfigurationValidate => "configuration-validate",
            Self::ConfigurationSave => "configuration-save",
            Self::ConfigurationReset => "configuration-reset",
            Self::ConfigurationSecretMutation => "configuration-secret-mutation",
            Self::ConfigurationDrift => "configuration-drift",
            Self::ConfigurationReconcile => "configuration-reconcile",
            Self::ConfigurationDelete => "configuration-delete",
            Self::ConfigurationRetention => "configuration-retention",
            Self::ConfigurationCleanup => "configuration-cleanup",
        }
    }

    pub(crate) fn invalidates_effective_catalog(self) -> bool {
        !matches!(
            self,
            Self::ResolveApiPrompt
                | Self::ListAgentSkills
                | Self::LoadSkill
                | Self::ReadSkillResource
                | Self::TrackView
                | Self::TrackUse
                | Self::DetectDrift
                | Self::SeedBuiltins
                | Self::OverlayValidation
                // Configuration values do not change which Skill revision wins or what schema it
                // declares, so they cannot invalidate the package catalog. Revisit if the
                // effective overview ever derives its scoped summaries from stored records.
                | Self::ConfigurationValidate
                | Self::ConfigurationSave
                | Self::ConfigurationReset
                | Self::ConfigurationSecretMutation
                | Self::ConfigurationDrift
                | Self::ConfigurationReconcile
                | Self::ConfigurationDelete
                | Self::ConfigurationRetention
                | Self::ConfigurationCleanup
        )
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
