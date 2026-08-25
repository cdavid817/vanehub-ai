use chrono::{DateTime, Utc};

use crate::contexts::personalization::domain::{
    AgentId, LegacySourceFingerprint, LegacySourceId, LegacySourceLocator, MaintenanceFailure,
    MemoryAudience, MemoryProvenance, MemoryScope, MemorySensitivity, MemorySource, MemoryStatus,
    MemoryType, WorkspaceKey,
};

/// Everything needed to create one memory. Deliberately has no id field: allocating the immutable
/// id is the store's job, so no caller can propose one and no caller can reuse one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateMemoryInput {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) memory_type: MemoryType,
    pub(crate) content: String,
    pub(crate) scope: MemoryScope,
    pub(crate) audience: MemoryAudience,
    pub(crate) status: MemoryStatus,
    pub(crate) source: MemorySource,
    pub(crate) provenance: MemoryProvenance,
    pub(crate) sensitivity: MemorySensitivity,
}

/// A partial edit. Absent fields are left alone, which is what lets a rename be a rename rather
/// than a full rewrite that could clobber a concurrent content edit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct UpdateMemoryPatch {
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) memory_type: Option<MemoryType>,
    pub(crate) content: Option<String>,
    pub(crate) scope: Option<MemoryScope>,
    pub(crate) audience: Option<MemoryAudience>,
    pub(crate) status: Option<MemoryStatus>,
    pub(crate) sensitivity: Option<MemorySensitivity>,
}

/// What a delete actually managed to remove.
///
/// Reported per surface rather than as one boolean because the authoritative file, the projection
/// row, the derived index line, and the retrieval entry can fail independently, and a partial
/// delete must set repair-required rather than report success.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DeleteMemoryOutcome {
    pub(crate) deleted_file: bool,
    pub(crate) deleted_projection_row: bool,
    pub(crate) removed_index_line: bool,
    pub(crate) revoked_retrieval_entry: bool,
    pub(crate) failures: Vec<MaintenanceFailure>,
}

impl DeleteMemoryOutcome {
    pub(crate) fn requires_repair(&self) -> bool {
        !self.failures.is_empty()
    }
}

/// Exact counts behind a reset preview, split so the confirmation dialog can state what will be
/// removed instead of a single total the user has to trust.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ResetCounts {
    pub(crate) matched: usize,
    pub(crate) global: usize,
    pub(crate) workspace: usize,
    pub(crate) candidates: usize,
    pub(crate) malformed: usize,
}

/// The fields a readable legacy memory carries.
///
/// `memory_type` is `Option` because v1 permitted an absent or unrecognized type and degraded to
/// untyped. Migration preserves that rather than guessing: a wrong type is worse than a missing
/// one, and the management UI can ask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LegacyMemoryFields {
    pub(crate) name: String,
    pub(crate) description: String,
    /// The raw v1 `type` value. Mapped onto the v2 taxonomy in one place, so an unrecognized value
    /// becomes explicitly untyped rather than being guessed at differently by two callers.
    pub(crate) memory_type: Option<String>,
    pub(crate) content: String,
    pub(crate) source_agent_id: Option<String>,
    /// The raw workspace path v1 recorded, not a workspace key. v1 stored the display path, and two
    /// remote workspaces can share one; deriving a stable key from it is the identity resolver's
    /// job, not this struct's.
    pub(crate) folder: Option<String>,
    /// The raw v1 `source` value. Mapped to a typed value in one place, and an unrecognized one
    /// becomes absent rather than being folded into whichever variant happens to be first.
    pub(crate) save_source: Option<String>,
    /// Where the file was, relative to the memory directory. Carried on the fields rather than
    /// re-derived from the locator downstream so the value that reaches provenance is the value
    /// enumeration actually found.
    pub(crate) source_relative_path: Option<String>,
    /// Both timestamps are carried because both are real. `created_at` is what the file declared;
    /// `modified_at` is what the filesystem knows, and it is what recency ordering used under v1 —
    /// dropping it would put a memory the model had just corrected behind every stale one.
    pub(crate) created_at: Option<DateTime<Utc>>,
    pub(crate) modified_at: Option<DateTime<Utc>>,
}

/// One source as enumeration found it.
///
/// `fields` is `None` for a source that will not parse. That is a first-class outcome rather than a
/// skip: the source still has a locator and a fingerprint, so it can be journalled, quarantined,
/// and reported instead of silently vanishing the way the previous parse-dependent scan let it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiscoveredLegacySource {
    pub(crate) locator: LegacySourceLocator,
    /// `None` for a source whose bytes were never read — one that is unreadable, or one that
    /// resolves outside the directory and must not be opened at all. Absent rather than a
    /// placeholder value, so nothing downstream can compare against a fingerprint of nothing and
    /// conclude the source is unchanged.
    pub(crate) fingerprint: Option<LegacySourceFingerprint>,
    pub(crate) fields: Option<LegacyMemoryFields>,
}

impl DiscoveredLegacySource {
    pub(crate) fn source_id(&self) -> LegacySourceId {
        self.locator.source_id()
    }
}

/// What one migration run did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct MigrationRunOutcome {
    pub(crate) discovered: usize,
    /// Reached `Completed` in this run.
    pub(crate) migrated: usize,
    /// Already terminal before this run started.
    pub(crate) already_done: usize,
    pub(crate) quarantined: usize,
    /// Changed between discovery and a checkpoint. Nothing was overwritten or deleted.
    pub(crate) source_changed: usize,
    /// Left for the next run because the directory was held by someone else. Not a failure: the
    /// journal still describes exactly where each source stopped, and nothing was half-applied.
    pub(crate) deferred: usize,
    pub(crate) failed: usize,
    /// Codes only — never a path or a memory body, because this is reported and logged.
    pub(crate) failure_codes: Vec<String>,
}

impl MigrationRunOutcome {
    pub(crate) fn requires_repair(&self) -> bool {
        self.failed > 0 || self.source_changed > 0
    }
}

/// What the caller knows about a workspace before an identity is derived from it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WorkspaceIdentityRequest {
    /// A stable id the workspace subsystem already assigns. Preferred over anything derived here:
    /// two subsystems deriving their own answer is how "the same workspace" ends up meaning two
    /// different things.
    pub(crate) stable_id: Option<String>,
    pub(crate) project_path: Option<String>,
    pub(crate) worktree_path: Option<String>,
    /// Present for a remote workspace. Carries connection identity, not just a path, because two
    /// hosts can expose the same path and must not share a scope.
    pub(crate) remote_uri: Option<String>,
}

/// What eligibility is being asked about, after every policy and session restriction is decided.
///
/// Deliberately not a policy snapshot: by the time this is built, every "may I" question has been
/// answered, and what remains is a query over records. A criteria object that still carried policy
/// could be built inconsistently with the snapshot it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MemoryEligibilityCriteria {
    /// Whose audience must include this memory.
    pub(crate) agent_id: AgentId,
    /// Whether global-scoped memories may be read at all.
    pub(crate) allow_global: bool,
    /// The single workspace whose memories may be read. `None` excludes every workspace scope.
    pub(crate) workspace: Option<WorkspaceKey>,
    /// Set for a project-only session, so a global memory is reported as excluded by the session
    /// rather than by the global toggle — the same outcome with a very different fix.
    pub(crate) project_only: bool,
    /// How many refs to return. The count is always exact regardless.
    pub(crate) limit: usize,
}
