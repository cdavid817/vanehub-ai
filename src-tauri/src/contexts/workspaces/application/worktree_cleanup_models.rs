//! What the cleanup use case reads, decides on, and asks its adapters for.
//!
//! Everything here is the application's own vocabulary. Git output, SQLite rows and Tauri DTOs
//! are all mapped into these shapes at their own boundaries, so a change in any one of them
//! cannot quietly change what "clean" or "registered" means in the policy.

use super::WorkspaceApplicationError;
use crate::contexts::workspaces::domain::{ManagedWorktree, WorktreeIdentity, WorktreeOrigin};
use std::path::Path;
use std::time::Duration;

/// Whether a check saw everything it needed to. `Incomplete` is never treated as "nothing found".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckCompleteness {
    Complete,
    Incomplete,
}

impl CheckCompleteness {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Incomplete => "incomplete",
        }
    }
}

/// Upper bounds a probe runs under. Constants rather than tuning knobs: exceeding one produces an
/// `Incomplete` answer, never a longer wait.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProbeBudget {
    pub(crate) git_timeout: Duration,
    pub(crate) max_status_entries: usize,
    pub(crate) max_ignored_entries: usize,
    pub(crate) max_ignored_bytes: u64,
    pub(crate) max_ignored_samples: usize,
}

impl ProbeBudget {
    pub(crate) const DEFAULT: Self = Self {
        git_timeout: Duration::from_secs(10),
        max_status_entries: 10_000,
        max_ignored_entries: 10_000,
        max_ignored_bytes: 2 * 1024 * 1024,
        max_ignored_samples: 100,
    };
}

/// How long the single destructive command may run.
pub(crate) const REMOVAL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct WorktreeChangeSummary {
    pub(crate) tracked_modified: usize,
    pub(crate) staged: usize,
    pub(crate) conflicted: usize,
    pub(crate) untracked: usize,
    pub(crate) ignored_paths: usize,
    pub(crate) completeness: Option<CheckCompleteness>,
}

impl WorktreeChangeSummary {
    pub(crate) fn has_non_ignored_changes(&self) -> bool {
        self.tracked_modified > 0 || self.staged > 0 || self.conflicted > 0 || self.untracked > 0
    }
}

/// One ignored path, described by metadata only. Contents are never read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IgnoredEntry {
    /// Lossy, display-only. Identity is carried by the fingerprint, which is computed from the
    /// raw bytes before any conversion.
    pub(crate) path: String,
    pub(crate) kind: &'static str,
    pub(crate) size: u64,
    pub(crate) modified_unix: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IgnoredInventory {
    pub(crate) total_entries: usize,
    pub(crate) samples: Vec<IgnoredEntry>,
    pub(crate) samples_truncated: bool,
    pub(crate) completeness: CheckCompleteness,
    /// SHA-256 over every entry's raw path, kind, size and mtime, in walk order. Acknowledging
    /// the inventory means acknowledging exactly this value.
    pub(crate) fingerprint: String,
}

/// Read-only facts gathered about one directory.
///
/// A probe never fails as a `Result`: a Git that could not be run is a fact about the directory
/// (`failure`), and every consumer must treat that fact as a blocker rather than as an absence
/// of problems.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct WorktreeProbe {
    pub(crate) identity: Option<WorktreeIdentity>,
    pub(crate) root_exists: bool,
    pub(crate) is_linked: bool,
    pub(crate) registered: bool,
    pub(crate) locked: bool,
    pub(crate) prunable: bool,
    pub(crate) detached: bool,
    pub(crate) branch_resolves_to_head: bool,
    pub(crate) in_progress_operation: bool,
    pub(crate) nested_layout: bool,
    pub(crate) unsupported_layout: Option<&'static str>,
    /// The main working tree Git reported for the repository — the directory the removal command
    /// runs from. Never the target itself.
    pub(crate) anchor: Option<String>,
    pub(crate) changes: Option<WorktreeChangeSummary>,
    pub(crate) ignored: Option<IgnoredInventory>,
    pub(crate) failure: Option<&'static str>,
}

impl WorktreeProbe {
    pub(crate) fn failed(reason: &'static str) -> Self {
        Self {
            failure: Some(reason),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Presence {
    Present,
    Absent,
    Unknown,
}

/// What the world looks like after (or instead of) a removal. Three-valued on purpose: an
/// offline volume or a permission error is `Unknown`, and unknown is never "gone".
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorktreeObservation {
    pub(crate) root_present: Presence,
    pub(crate) registered: Presence,
    /// `Some(false)` when something is at the root but it is not the object that was recorded.
    pub(crate) identity_matches: Option<bool>,
    pub(crate) anchor_available: bool,
}

impl WorktreeObservation {
    pub(crate) fn confirmed_removed(&self) -> bool {
        self.anchor_available
            && self.root_present == Presence::Absent
            && self.registered == Presence::Absent
    }

    pub(crate) fn confirmed_intact(&self) -> bool {
        self.anchor_available
            && self.root_present == Presence::Present
            && self.registered == Presence::Present
            && self.identity_matches == Some(true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorktreeRemovalOutcome {
    Succeeded,
    Refused {
        exit_code: Option<i32>,
        diagnostic: String,
    },
    /// The command was killed after its deadline. `exit_confirmed` says whether the adapter saw
    /// the child exit; the effect on disk is unknown either way.
    TimedOut {
        exit_confirmed: bool,
    },
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorktreeRemovalReport {
    pub(crate) outcome: WorktreeRemovalOutcome,
    pub(crate) observation: WorktreeObservation,
}

/// Who holds a gate. The instance id maps to an OS-locked file held for the process lifetime,
/// which is how another instance proves this one is gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GateOwner {
    pub(crate) instance_id: String,
    pub(crate) epoch: u64,
    pub(crate) operation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GateClaim {
    pub(crate) worktree_id: String,
    pub(crate) canonical_root: String,
    pub(crate) owner: GateOwner,
    pub(crate) claimed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GateHolder {
    pub(crate) worktree_id: String,
    pub(crate) owner: GateOwner,
    pub(crate) claimed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GateRejection {
    Held(GateHolder),
    Storage(String),
}

/// What sessions tells workspaces about a session so its worktree can be resolved. Only the
/// facts the resolution needs; no message content, no titles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorktreeSessionView {
    pub(crate) session_id: String,
    pub(crate) worktree_path: Option<String>,
    pub(crate) worktree_branch: Option<String>,
    pub(crate) project_path: Option<String>,
    pub(crate) remote: bool,
    pub(crate) loop_owned: bool,
    /// Whether a completed creation operation for this session survives. Supplied by the caller,
    /// which owns that history; workspaces does not inspect operations.
    pub(crate) creation_evidence: bool,
}

/// The worktree a session is bound to, if any, and what the application knows about its origin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorktreeResolution {
    /// Present when a managed record exists (verified now or verified earlier). Absent means the
    /// directory is known only by path and may only ever be kept.
    pub(crate) record: Option<ManagedWorktree>,
    pub(crate) canonical_root: Option<String>,
    pub(crate) display_root: String,
    pub(crate) branch: Option<String>,
    pub(crate) origin: WorktreeOrigin,
    pub(crate) provenance_reason: &'static str,
}

impl WorktreeResolution {
    /// The key two sessions are grouped under. A verified record's id when there is one, else
    /// the canonical root, else the raw path — so two sessions naming the same directory in two
    /// spellings still meet, and two directories never do.
    pub(crate) fn group_key(&self) -> String {
        if let Some(record) = &self.record {
            return record.id.clone();
        }
        format!(
            "unverified:{}",
            self.canonical_root
                .clone()
                .unwrap_or_else(|| self.display_root.clone())
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorktreeInspection {
    pub(crate) record: ManagedWorktree,
    pub(crate) probe: WorktreeProbe,
    /// Whether the probe's identity equals the one recorded when the worktree became ours.
    pub(crate) identity_matches: bool,
}

pub(crate) trait ManagedWorktreeRepository: Send + Sync {
    fn insert(&self, record: &ManagedWorktree) -> Result<(), WorkspaceApplicationError>;
    fn find(&self, id: &str) -> Result<Option<ManagedWorktree>, WorkspaceApplicationError>;
    fn find_by_root(
        &self,
        canonical_or_requested_root: &str,
    ) -> Result<Option<ManagedWorktree>, WorkspaceApplicationError>;
    /// Compare-and-set on `revision`; `Ok(false)` means someone else moved it first.
    fn save(
        &self,
        record: &ManagedWorktree,
        expected_revision: u64,
    ) -> Result<bool, WorkspaceApplicationError>;
    fn bind_session(
        &self,
        worktree_id: &str,
        session_id: &str,
        binding_kind: &str,
    ) -> Result<(), WorkspaceApplicationError>;
    fn unbind_sessions(
        &self,
        worktree_id: &str,
        session_ids: &[String],
    ) -> Result<(), WorkspaceApplicationError>;
    fn bound_sessions(&self, worktree_id: &str) -> Result<Vec<String>, WorkspaceApplicationError>;
    fn find_by_session(
        &self,
        session_id: &str,
    ) -> Result<Option<ManagedWorktree>, WorkspaceApplicationError>;
}

pub(crate) trait WorktreeProbePort: Send + Sync {
    fn probe(&self, root: &Path, budget: &ProbeBudget) -> WorktreeProbe;
    /// Identity, registration and anchor only: no status walk, no ignored inventory, no index
    /// scan. For the steps that ask "is this still the same object", where a full probe would
    /// re-read a large tree for facts they do not consume. Defaults to the full probe.
    fn probe_identity(&self, root: &Path, budget: &ProbeBudget) -> WorktreeProbe {
        self.probe(root, budget)
    }
    /// The filesystem's canonical form of an existing directory; `None` when it is not one.
    fn canonical_root(&self, path: &str) -> Option<String>;
    fn observe(&self, expected: &WorktreeIdentity, anchor: Option<&Path>) -> WorktreeObservation;
}

pub(crate) trait WorktreeRemovalPort: Send + Sync {
    /// The one destructive command. `anchor` is the directory the command runs from and must be
    /// outside `target`; the adapter passes exactly `worktree remove <target>` and nothing else.
    fn remove(&self, anchor: &Path, target: &Path, timeout: Duration) -> WorktreeRemovalOutcome;
}

pub(crate) trait WorktreeUseGatePort: Send + Sync {
    fn claim(
        &self,
        worktree_id: &str,
        canonical_root: &str,
        owner: &GateOwner,
    ) -> Result<GateClaim, GateRejection>;
    fn release(&self, claim: &GateClaim) -> Result<(), WorkspaceApplicationError>;
    fn holder_for_root(
        &self,
        canonical_root: &str,
    ) -> Result<Option<GateHolder>, WorkspaceApplicationError>;
    /// Whether the holder's instance still runs. `Err` is "unknown", which callers must treat as
    /// alive.
    fn owner_is_alive(&self, holder: &GateHolder) -> Result<bool, WorkspaceApplicationError>;
}

pub(crate) trait WorktreeIdPort: Send + Sync {
    fn next_worktree_id(&self) -> String;
}

pub(crate) trait WorktreeCleanupClockPort: Send + Sync {
    fn now(&self) -> String;
}
