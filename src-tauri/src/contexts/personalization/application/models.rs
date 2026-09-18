use chrono::{DateTime, Utc};

use crate::contexts::personalization::domain::{
    AgentId, LegacySourceFingerprint, LegacySourceId, LegacySourceLocator, MaintenanceFailure,
    MemoryAudience, MemoryId, MemoryProvenance, MemoryReadHandle, MemoryScope, MemorySensitivity,
    MemorySource, MemoryStatus, MemoryType, WorkspaceKey,
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

impl WorkspaceIdentityRequest {
    /// The identity request for a stored session, in the owner's preference order: a worktree is
    /// its own workspace, a remote workspace is a connection identity, then the project root, and
    /// only then the legacy folder string. `None` when the session names no workspace at all.
    ///
    /// One function because two callers -- the generation bridge and the bound-session preview --
    /// must agree on which workspace a session is in, and two rules would eventually disagree.
    pub(crate) fn from_session_workspace(
        worktree_path: Option<&str>,
        remote_uri: Option<&str>,
        project_path: Option<&str>,
        folder: Option<&str>,
    ) -> Option<Self> {
        fn present(value: Option<&str>) -> Option<&str> {
            value.map(str::trim).filter(|value| !value.is_empty())
        }
        if let Some(worktree) = present(worktree_path) {
            return Some(Self {
                worktree_path: Some(worktree.to_string()),
                ..Self::default()
            });
        }
        if let Some(remote) = present(remote_uri) {
            return Some(Self {
                remote_uri: Some(remote.to_string()),
                ..Self::default()
            });
        }
        if let Some(project) = present(project_path) {
            return Some(Self {
                project_path: Some(project.to_string()),
                ..Self::default()
            });
        }
        present(folder).and_then(super::migrate_legacy_memories::legacy_workspace_request)
    }
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

/// How much work one complete-eligibility enumeration may do before it is declared incomplete.
///
/// Versioned rather than implied: a relation that stopped early because of a limit is a partial
/// authorization set, and searching over it would silently drop the eligible records past the
/// cut. The caller sees `complete: false` and fails closed instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EligibilityEnumerationBudget {
    pub(crate) max_entries: usize,
    pub(crate) page_size: usize,
}

impl EligibilityEnumerationBudget {
    /// Enough for any realistic personal memory pool while still bounding the memory a single
    /// recall may hold; a pool past this is reported as unavailable, never truncated.
    pub(crate) const DEFAULT: Self = Self {
        max_entries: 50_000,
        page_size: 1_000,
    };
}

/// The complete authorized metadata domain for one read context, without a single body.
///
/// `entries` is every eligible record at the moment of the consistent read, each pinned by the
/// version the authorization was decided on. Retrieval materializes this as a query-local relation
/// and filters both paths through it before ranking; delivery re-validates each pinned handle
/// against the authoritative file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AuthorizedMemoryRelation {
    pub(crate) entries: Vec<MemoryReadHandle>,
    /// How many projected rows the enumeration classified, eligible or not. Diagnostics only.
    pub(crate) considered: usize,
    /// False when the enumeration stopped at its budget. A partial relation must not be searched.
    pub(crate) complete: bool,
}

/// One record as the background index maintenance sees it: every valid active record regardless of
/// scope or audience, because the local keyword index is host-wide and each session's read
/// authority is applied at query time, not at index time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexMaintenanceRecord {
    pub(crate) handle: MemoryReadHandle,
    pub(crate) content: String,
    pub(crate) created_at: chrono::DateTime<chrono::Utc>,
    /// Whether the body may leave the machine for a remote embedder. At this baseline nothing
    /// grants that for a workspace-scoped or audience-restricted record, so those stay
    /// keyword-only; the flag is decided here, once, from the authoritative record.
    pub(crate) egress_restricted: bool,
}

/// One body read through the governed path, at exactly the version the handle pinned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PinnedMemoryBody {
    pub(crate) handle: MemoryReadHandle,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) memory_type: MemoryType,
    pub(crate) content: String,
    pub(crate) scope_hint: String,
    pub(crate) updated_at: chrono::DateTime<chrono::Utc>,
    pub(crate) created_at: chrono::DateTime<chrono::Utc>,
}

/// Why a governed read refused. Typed, bounded, and free of record identity: the model-facing
/// caller turns these into the existing "unavailable" tool result and nothing more.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MemoryReadRefusal {
    /// The context was not minted by this process for this epoch, or was altered.
    Inauthentic,
    /// The frozen context permits no read at all (temporary, disabled, unresolved workspace).
    ReadDenied,
    /// Memory is not in a `Ready` generation, or the generation moved since the context froze.
    Unhealthy,
    /// The complete eligibility relation could not be enumerated within budget.
    Incomplete,
    Storage(String),
}

impl MemoryReadRefusal {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Inauthentic => "inauthentic_context",
            Self::ReadDenied => "read_denied",
            Self::Unhealthy => "unhealthy",
            Self::Incomplete => "incomplete_authority",
            Self::Storage(_) => "storage",
        }
    }
}

/// Which embedding dispatches the authoritative store still permits, per handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EmbeddingEgressDecision {
    pub(crate) id: MemoryId,
    pub(crate) permitted: bool,
}
