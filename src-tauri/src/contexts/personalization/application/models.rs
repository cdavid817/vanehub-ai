use crate::contexts::personalization::domain::{
    MaintenanceFailure, MemoryAudience, MemoryProvenance, MemoryScope, MemorySensitivity,
    MemorySource, MemoryStatus, MemoryType,
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
