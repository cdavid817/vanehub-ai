//! The vocabulary of session deletion: what a preview says, what an execution asks for, and what
//! an operation reports afterwards.
//!
//! These types are serialized as the command contract on purpose. The frontend, the journal and
//! the coordinator all read the same literal for a phase or an effect, so there is no mapping
//! layer in which "removed" could quietly become "succeeded".

use serde::{Deserialize, Serialize};

/// Sessions per request. A ceiling, not a target: anything above it is refused before any
/// session is read.
pub(crate) const MAX_DELETION_BATCH: usize = 100;
/// How long a preview stays executable. Long enough to read a dialog, short enough that the
/// facts it carries are still plausibly the facts on disk.
pub(crate) const PREVIEW_TTL_SECONDS: i64 = 10 * 60;
/// How long every managed writer of a session gets to stop before the group is failed.
pub(crate) const QUIESCE_DEADLINE_SECONDS: u64 = 15;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum WorktreeDeletionPolicy {
    Keep,
    RemoveSafe,
}

impl WorktreeDeletionPolicy {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::RemoveSafe => "remove-safe",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "keep" => Some(Self::Keep),
            "remove-safe" => Some(Self::RemoveSafe),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DeletionRuntimeEffect {
    Native,
    Simulated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DeletionCheckCompleteness {
    Complete,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DeletionWorkspaceKind {
    Project,
    Remote,
    Worktree,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewSessionDeletionRequest {
    pub(crate) session_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeletionPreviewSession {
    pub(crate) session_id: String,
    pub(crate) title: String,
    pub(crate) archived: bool,
    pub(crate) active: bool,
    pub(crate) workspace_kind: DeletionWorkspaceKind,
    /// Which worktree row this session belongs to, when it has one.
    pub(crate) worktree_key: Option<String>,
    pub(crate) display_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeletionExternalReference {
    pub(crate) kind: String,
    pub(crate) id: String,
    pub(crate) label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeletionChangeSummary {
    pub(crate) tracked_modified: usize,
    pub(crate) staged: usize,
    pub(crate) conflicted: usize,
    pub(crate) untracked: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeletionIgnoredSample {
    pub(crate) path: String,
    pub(crate) kind: String,
    pub(crate) size: u64,
    pub(crate) modified_unix: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeletionIgnoredSummary {
    pub(crate) total_entries: usize,
    pub(crate) samples: Vec<DeletionIgnoredSample>,
    pub(crate) samples_truncated: bool,
    pub(crate) completeness: DeletionCheckCompleteness,
    pub(crate) fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeletionPreviewWorktree {
    /// Opaque group key: the managed record id when verified, else a derived key that is never
    /// accepted as a removal target.
    pub(crate) worktree_key: String,
    pub(crate) worktree_id: Option<String>,
    pub(crate) display_path: String,
    pub(crate) branch: Option<String>,
    pub(crate) session_ids: Vec<String>,
    pub(crate) external_references: Vec<DeletionExternalReference>,
    pub(crate) allowed_policies: Vec<WorktreeDeletionPolicy>,
    pub(crate) blockers: Vec<String>,
    pub(crate) checks: DeletionCheckCompleteness,
    pub(crate) changes: Option<DeletionChangeSummary>,
    pub(crate) ignored: Option<DeletionIgnoredSummary>,
    pub(crate) requires_ignored_acknowledgement: bool,
    pub(crate) origin: String,
    pub(crate) provenance: String,
    pub(crate) resource_status: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionDeletionPreview {
    pub(crate) preview_id: String,
    pub(crate) runtime_effect: DeletionRuntimeEffect,
    pub(crate) created_at: String,
    pub(crate) expires_at: String,
    pub(crate) sessions: Vec<DeletionPreviewSession>,
    pub(crate) worktrees: Vec<DeletionPreviewWorktree>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IgnoredFilesAcknowledgement {
    pub(crate) fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeDeletionChoice {
    pub(crate) worktree_key: String,
    pub(crate) policy: WorktreeDeletionPolicy,
    #[serde(default)]
    pub(crate) ignored_files_acknowledgement: Option<IgnoredFilesAcknowledgement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecuteSessionDeletionRequest {
    pub(crate) request_id: String,
    pub(crate) preview_id: String,
    #[serde(default)]
    pub(crate) worktree_choices: Vec<WorktreeDeletionChoice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionDeletionHandle {
    pub(crate) operation_id: String,
    pub(crate) runtime_effect: DeletionRuntimeEffect,
    pub(crate) operation_task_id: Option<String>,
    /// The same request had already been accepted; nothing new was started.
    pub(crate) existing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DeletionOutcome {
    Pending,
    Succeeded,
    Failed,
    Partial,
    AwaitingDecision,
    NeedsAttention,
}

impl DeletionOutcome {
    pub(crate) fn is_terminal(self) -> bool {
        !matches!(self, Self::Pending)
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Partial => "partial",
            Self::AwaitingDecision => "awaiting_decision",
            Self::NeedsAttention => "needs_attention",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "partial" => Some(Self::Partial),
            "awaiting_decision" => Some(Self::AwaitingDecision),
            "needs_attention" => Some(Self::NeedsAttention),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DeletionPhase {
    Accepted,
    Quiescing,
    Revalidating,
    RemovingWorktree,
    DeletingSessions,
    Completed,
}

impl DeletionPhase {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Quiescing => "quiescing",
            Self::Revalidating => "revalidating",
            Self::RemovingWorktree => "removing_worktree",
            Self::DeletingSessions => "deleting_sessions",
            Self::Completed => "completed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "accepted" => Some(Self::Accepted),
            "quiescing" => Some(Self::Quiescing),
            "revalidating" => Some(Self::Revalidating),
            "removing_worktree" => Some(Self::RemovingWorktree),
            "deleting_sessions" => Some(Self::DeletingSessions),
            "completed" => Some(Self::Completed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorktreeEffect {
    NotRequested,
    Retained,
    RemoveStarted,
    Removed,
    RemovalUnknown,
}

impl WorktreeEffect {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NotRequested => "not_requested",
            Self::Retained => "retained",
            Self::RemoveStarted => "remove_started",
            Self::Removed => "removed",
            Self::RemovalUnknown => "removal_unknown",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "not_requested" => Some(Self::NotRequested),
            "retained" => Some(Self::Retained),
            "remove_started" => Some(Self::RemoveStarted),
            "removed" => Some(Self::Removed),
            "removal_unknown" => Some(Self::RemovalUnknown),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SessionDbEffect {
    Pending,
    Deleted,
    Retained,
}

impl SessionDbEffect {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Deleted => "deleted",
            Self::Retained => "retained",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "deleted" => Some(Self::Deleted),
            "retained" => Some(Self::Retained),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DeletionGroupStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    AwaitingDecision,
    /// The worktree is confirmed gone; the session rows are still to be deleted.
    FinalizePending,
    NeedsAttention,
}

impl DeletionGroupStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::AwaitingDecision => "awaiting_decision",
            Self::FinalizePending => "finalize_pending",
            Self::NeedsAttention => "needs_attention",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "running" => Some(Self::Running),
            "succeeded" => Some(Self::Succeeded),
            "failed" => Some(Self::Failed),
            "awaiting_decision" => Some(Self::AwaitingDecision),
            "finalize_pending" => Some(Self::FinalizePending),
            "needs_attention" => Some(Self::NeedsAttention),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeletionGroupResult {
    pub(crate) group_id: String,
    pub(crate) worktree_key: Option<String>,
    pub(crate) worktree_id: Option<String>,
    pub(crate) policy: WorktreeDeletionPolicy,
    pub(crate) session_ids: Vec<String>,
    pub(crate) status: DeletionGroupStatus,
    pub(crate) phase: DeletionPhase,
    pub(crate) worktree_effect: WorktreeEffect,
    pub(crate) db_effect: SessionDbEffect,
    pub(crate) error_code: Option<String>,
    pub(crate) retained_path: Option<String>,
    pub(crate) attempt: u32,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionDeletionOperation {
    pub(crate) operation_id: String,
    pub(crate) request_id: String,
    pub(crate) outcome: DeletionOutcome,
    pub(crate) phase: DeletionPhase,
    pub(crate) revision: u64,
    pub(crate) runtime_effect: DeletionRuntimeEffect,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) completed_at: Option<String>,
    pub(crate) groups: Vec<DeletionGroupResult>,
    pub(crate) error_code: Option<String>,
    pub(crate) operation_task_id: Option<String>,
}

impl SessionDeletionOperation {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RetrySessionDeletionRequest {
    pub(crate) operation_id: String,
    pub(crate) expected_revision: u64,
    pub(crate) retry_request_id: String,
    /// Required whenever any retried group asks for `remove-safe` again.
    #[serde(default)]
    pub(crate) preview_id: Option<String>,
    #[serde(default)]
    pub(crate) worktree_choices: Vec<WorktreeDeletionChoice>,
}

/// Stable error codes crossing the command boundary. Kept next to the models so the frontend's
/// locale table and the coordinator disagree about nothing.
pub(crate) mod error_code {
    pub(crate) const EMPTY_SELECTION: &str = "deletion_empty_selection";
    pub(crate) const BATCH_TOO_LARGE: &str = "deletion_batch_too_large";
    pub(crate) const PREVIEW_EXPIRED: &str = "deletion_preview_expired";
    pub(crate) const PREVIEW_TARGET_MISMATCH: &str = "deletion_preview_target_mismatch";
    pub(crate) const UNKNOWN_WORKTREE_CHOICE: &str = "deletion_unknown_worktree_choice";
    pub(crate) const DUPLICATE_WORKTREE_CHOICE: &str = "deletion_duplicate_worktree_choice";
    pub(crate) const POLICY_NOT_ALLOWED: &str = "deletion_policy_not_allowed";
    pub(crate) const IGNORED_ACKNOWLEDGEMENT_REQUIRED: &str =
        "deletion_ignored_acknowledgement_required";
    pub(crate) const IGNORED_ACKNOWLEDGEMENT_STALE: &str = "deletion_ignored_acknowledgement_stale";
    pub(crate) const REQUEST_ID_CONFLICT: &str = "deletion_request_id_conflict";
    pub(crate) const SESSION_CLAIMED: &str = "session_deletion_in_progress";
    pub(crate) const REVISION_CONFLICT: &str = "deletion_revision_conflict";
    pub(crate) const OPERATION_NOT_FOUND: &str = "deletion_operation_not_found";
    pub(crate) const RETRY_NOT_ALLOWED: &str = "deletion_retry_not_allowed";
    pub(crate) const RETRY_REQUIRES_PREVIEW: &str = "deletion_retry_requires_preview";
    pub(crate) const QUIESCE_TIMEOUT: &str = "deletion_quiesce_timeout";
    pub(crate) const QUIESCE_FAILED: &str = "deletion_quiesce_failed";
    pub(crate) const GATE_HELD: &str = "gate_held";
    pub(crate) const IDENTITY_CHANGED: &str = "identity_changed";
    pub(crate) const REMOVAL_REFUSED: &str = "worktree_removal_refused";
    pub(crate) const REMOVAL_UNKNOWN: &str = "worktree_removal_unknown";
    pub(crate) const REMOVAL_TIMED_OUT: &str = "worktree_removal_timed_out";
    pub(crate) const FINALIZE_FAILED: &str = "session_finalize_failed";
    pub(crate) const INTERRUPTED: &str = "deletion_interrupted";
    /// An internal step raised an error; how the group was parked depends on how far it got.
    pub(crate) const RUN_FAILED: &str = "deletion_run_failed";
}
