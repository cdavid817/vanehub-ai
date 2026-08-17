use crate::contexts::tooling::skill_tools::domain::{
    SkillToolAvailability, SkillToolDeclaration, SkillToolDiagnostic, SkillToolDiagnosticSummary,
    SkillToolIntegrity, SkillToolKey, SkillToolLifecycle, SkillToolOwnerId, SkillToolRevision,
    SkillToolSourceScope,
};
use std::collections::BTreeMap;

/// Largest inventory a Skill's overview will report. A Skill that declares more tools than this
/// is truncated rather than paged: the overview is a status surface, and the Tools tab is where a
/// complete list belongs.
pub(crate) const MAX_INVENTORY_ENTRIES: usize = 16;

/// One resolved Skill package, as the caller of discovery resolved it.
///
/// `root_path` is the only filesystem knowledge this context has, and it comes from effective
/// Skill resolution rather than being recomputed here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolPackageRef {
    pub(crate) owner: SkillToolOwnerId,
    pub(crate) source: SkillToolSourceScope,
    pub(crate) base_revision: String,
    pub(crate) root_path: String,
}

/// The winning revision plus the revisions it shadows.
///
/// Shadowed packages are carried so discovery can *report* that they were ignored; nothing in the
/// pipeline is allowed to read their content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectiveSkillToolPackage {
    pub(crate) effective: SkillToolPackageRef,
    pub(crate) shadowed: Vec<SkillToolPackageRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolFileEntry {
    pub(crate) relative_path: String,
    pub(crate) size_bytes: u64,
}

/// One tool that discovery accepted, with the immutable witness it is keyed by.
///
/// Deliberately carries no bytes: the module contents stay on disk and are re-read under the
/// verified hash when the runtime needs them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiscoveredSkillTool {
    pub(crate) key: SkillToolKey,
    pub(crate) canonical_name: String,
    pub(crate) declaration: SkillToolDeclaration,
    pub(crate) integrity: SkillToolIntegrity,
    pub(crate) requires_module_runtime: bool,
}

/// One tool that discovery refused, kept so the surface can explain the gap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RejectedSkillTool {
    pub(crate) tool_id: Option<String>,
    pub(crate) diagnostic: SkillToolDiagnostic,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SkillToolDiscoveryOutcome {
    pub(crate) discovered: Vec<DiscoveredSkillTool>,
    pub(crate) rejected: Vec<RejectedSkillTool>,
    /// Files found under the reserved tool directory that no manifest entry binds.
    pub(crate) undeclared_files: Vec<String>,
    /// Revisions that were resolved as shadowed and therefore never read.
    pub(crate) ignored_shadowed_revisions: Vec<String>,
    pub(crate) manifest_present: bool,
}

impl SkillToolDiscoveryOutcome {
    pub(crate) fn is_empty(&self) -> bool {
        self.discovered.is_empty() && self.rejected.is_empty()
    }
}

/// Persisted governance state for one tool revision, keyed by its immutable witness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolRevisionState {
    pub(crate) key: SkillToolKey,
    pub(crate) integrity: SkillToolIntegrity,
    pub(crate) implementation_kind: String,
    pub(crate) lifecycle: SkillToolLifecycle,
    pub(crate) validation_code: Option<String>,
    pub(crate) diagnostics: SkillToolDiagnosticSummary,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

/// A bounded, byte-free row for the Skill overview and the Tools tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolInventoryEntry {
    pub(crate) tool_id: String,
    pub(crate) canonical_name: String,
    pub(crate) implementation_kind: String,
    pub(crate) revision: String,
    pub(crate) capabilities: Vec<String>,
    pub(crate) manifest_hash: String,
    pub(crate) implementation_hash: String,
    pub(crate) capability_digest: String,
    pub(crate) trusted: bool,
    pub(crate) enabled: bool,
    pub(crate) validation: String,
    pub(crate) quarantine_reason: Option<String>,
    pub(crate) availability: String,
}

/// The bounded per-Skill tool summary attached to effective Skill metadata.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SkillToolInventorySummary {
    pub(crate) entries: Vec<SkillToolInventoryEntry>,
    pub(crate) truncated: bool,
    pub(crate) declared_count: usize,
    pub(crate) available_count: usize,
    pub(crate) rejected_count: usize,
    pub(crate) quarantined_count: usize,
    pub(crate) undeclared_file_count: usize,
}

impl SkillToolInventorySummary {
    pub(crate) fn is_empty(&self) -> bool {
        self.declared_count == 0 && self.rejected_count == 0
    }
}

/// Projects discovery output plus persisted state into the bounded overview summary.
///
/// Takes state by key rather than reading it itself so the projection stays pure and a Skill with
/// no persisted rows yet projects to "declared but not yet trusted" instead of failing.
pub(crate) fn project_inventory_summary(
    outcome: &SkillToolDiscoveryOutcome,
    state: &BTreeMap<SkillToolRevision, SkillToolRevisionState>,
    module_runtime_available: bool,
) -> SkillToolInventorySummary {
    let mut summary = SkillToolInventorySummary {
        declared_count: outcome.discovered.len(),
        rejected_count: outcome.rejected.len(),
        undeclared_file_count: outcome.undeclared_files.len(),
        ..SkillToolInventorySummary::default()
    };
    for tool in &outcome.discovered {
        let lifecycle = state
            .get(&tool.key.revision)
            .map(|state| state.lifecycle.clone())
            .unwrap_or_default();
        let availability = lifecycle.availability(
            false,
            module_runtime_available,
            tool.requires_module_runtime,
        );
        if availability.is_available() {
            summary.available_count += 1;
        }
        if lifecycle.quarantine.is_quarantined() {
            summary.quarantined_count += 1;
        }
        if summary.entries.len() < MAX_INVENTORY_ENTRIES {
            summary
                .entries
                .push(inventory_entry(tool, &lifecycle, availability));
        } else {
            summary.truncated = true;
        }
    }
    summary
}

fn inventory_entry(
    tool: &DiscoveredSkillTool,
    lifecycle: &SkillToolLifecycle,
    availability: SkillToolAvailability,
) -> SkillToolInventoryEntry {
    SkillToolInventoryEntry {
        tool_id: tool.key.tool.as_str().to_string(),
        canonical_name: tool.canonical_name.clone(),
        implementation_kind: tool.declaration.implementation.kind().to_string(),
        revision: tool.key.revision.as_str().to_string(),
        capabilities: tool
            .declaration
            .capabilities
            .iter()
            .map(|capability| capability.as_declaration())
            .collect(),
        manifest_hash: tool.integrity.manifest_hash.as_str().to_string(),
        implementation_hash: tool.integrity.implementation_hash.as_str().to_string(),
        capability_digest: tool.integrity.capability_digest.clone(),
        trusted: lifecycle.trusted,
        enabled: lifecycle.enabled,
        validation: lifecycle.validation.as_str().to_string(),
        quarantine_reason: lifecycle.quarantine.reason().map(str::to_string),
        availability: availability.as_str().to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillToolLogLevel {
    Error,
    Warn,
    Info,
    Debug,
}

impl SkillToolLogLevel {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillToolLogAction {
    Discover,
    Validate,
    Trust,
    Revoke,
    SetEnabled,
    Quarantine,
    Recover,
    RegistryRefresh,
    Invocation,
    HostCall,
    Permission,
    Approval,
    Limit,
    Cancellation,
}

impl SkillToolLogAction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Discover => "skill-tool-discover",
            Self::Validate => "skill-tool-validate",
            Self::Trust => "skill-tool-trust",
            Self::Revoke => "skill-tool-revoke",
            Self::SetEnabled => "skill-tool-set-enabled",
            Self::Quarantine => "skill-tool-quarantine",
            Self::Recover => "skill-tool-recover",
            Self::RegistryRefresh => "skill-tool-registry-refresh",
            Self::Invocation => "skill-tool-invocation",
            Self::HostCall => "skill-tool-host-call",
            Self::Permission => "skill-tool-permission",
            Self::Approval => "skill-tool-approval",
            Self::Limit => "skill-tool-limit",
            Self::Cancellation => "skill-tool-cancellation",
        }
    }
}

/// A redacted structured event for unified log management.
///
/// `context` holds identities, hashes, counters, and outcome codes only; no payloads, module
/// bytes, or unbounded paths reach it, so the record is safe to persist without a second pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolLogEvent {
    pub(crate) action: SkillToolLogAction,
    pub(crate) level: SkillToolLogLevel,
    pub(crate) skill_id: Option<String>,
    pub(crate) tool_id: Option<String>,
    pub(crate) revision: Option<String>,
    pub(crate) message: String,
    pub(crate) context: BTreeMap<String, String>,
}
