//! What the deletion coordinator asks of the outside world.
//!
//! Each port is narrow and phrased in the coordinator's own terms. The workspace port in
//! particular is the *only* route by which a directory can be affected, and it never accepts a
//! path: every destructive call names a managed worktree id the backend resolved itself.

use super::models::{
    DeletionChangeSummary, DeletionCheckCompleteness, DeletionGroupStatus, DeletionIgnoredSummary,
    DeletionOutcome, DeletionPhase, DeletionRuntimeEffect, SessionDbEffect,
    SessionDeletionOperation, SessionDeletionPreview, WorktreeDeletionPolicy, WorktreeEffect,
};
use crate::contexts::sessions::application::{SessionRecord, SessionsApplicationError};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Duration;

/// Which process runs the operation. Recorded on the journal so another instance can tell an
/// abandoned operation from a live one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeletionOwner {
    pub(crate) instance_id: String,
    pub(crate) epoch: u64,
}

pub(crate) trait DeletionOwnerPort: Send + Sync {
    fn current(&self) -> DeletionOwner;
    /// `None` means "could not tell", which every caller treats as alive.
    fn is_alive(&self, instance_id: &str) -> Option<bool>;
}

pub(crate) trait DeletionClockPort: Send + Sync {
    fn now(&self) -> String;
    fn unix_now(&self) -> i64;
}

pub(crate) trait DeletionIdPort: Send + Sync {
    fn next_operation_id(&self) -> String;
    fn next_group_id(&self) -> String;
    fn next_preview_id(&self) -> String;
}

/// The worktree a session resolves to, in the coordinator's terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedWorktree {
    pub(crate) key: String,
    pub(crate) worktree_id: Option<String>,
    pub(crate) canonical_root: Option<String>,
    pub(crate) display_root: String,
    pub(crate) branch: Option<String>,
    pub(crate) origin: String,
    pub(crate) provenance: String,
    pub(crate) resource_status: Option<String>,
}

/// The Git identity a removal is authorized against. Persisted with the journal so recovery
/// observes the same object the user was shown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeIdentityView {
    pub(crate) canonical_root: String,
    pub(crate) git_dir: String,
    pub(crate) common_dir: String,
    pub(crate) branch: Option<String>,
    pub(crate) head: Option<String>,
    pub(crate) fs_identity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReferenceInput {
    pub(crate) external_count: usize,
    pub(crate) complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorktreeAssessment {
    pub(crate) identity: Option<WorktreeIdentityView>,
    pub(crate) record_revision: u64,
    pub(crate) allowed_policies: Vec<WorktreeDeletionPolicy>,
    pub(crate) blockers: Vec<String>,
    pub(crate) checks: DeletionCheckCompleteness,
    pub(crate) requires_ignored_acknowledgement: bool,
    pub(crate) changes: Option<DeletionChangeSummary>,
    pub(crate) ignored: Option<DeletionIgnoredSummary>,
    pub(crate) resource_status: String,
}

impl WorktreeAssessment {
    pub(crate) fn allows_removal(&self) -> bool {
        self.allowed_policies
            .contains(&WorktreeDeletionPolicy::RemoveSafe)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GateToken {
    pub(crate) worktree_id: String,
    pub(crate) canonical_root: String,
    pub(crate) instance_id: String,
    pub(crate) epoch: u64,
    pub(crate) operation_id: String,
    pub(crate) claimed_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tri {
    Present,
    Absent,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ObservationView {
    pub(crate) root_present: Tri,
    pub(crate) registered: Tri,
    pub(crate) confirmed_removed: bool,
    pub(crate) confirmed_intact: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RemovalOutcomeView {
    Succeeded,
    Refused { code: String },
    TimedOut,
    Unavailable { code: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemovalReportView {
    pub(crate) outcome: RemovalOutcomeView,
    pub(crate) observation: ObservationView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GateOutcome {
    Claimed(GateToken),
    Held { operation_id: String },
}

pub(crate) trait DeletionWorkspacePort: Send + Sync {
    fn resolve(
        &self,
        session: &SessionRecord,
    ) -> Result<Option<ResolvedWorktree>, SessionsApplicationError>;
    fn assess(
        &self,
        worktree_id: &str,
        references: ReferenceInput,
        owner: Option<(&DeletionOwner, &str)>,
    ) -> Result<WorktreeAssessment, SessionsApplicationError>;
    fn bound_sessions(&self, worktree_id: &str) -> Result<Vec<String>, SessionsApplicationError>;
    fn claim_gate(
        &self,
        worktree_id: &str,
        owner: &DeletionOwner,
        operation_id: &str,
    ) -> Result<GateOutcome, SessionsApplicationError>;
    fn release_gate(&self, token: &GateToken) -> Result<(), SessionsApplicationError>;
    fn begin_removal(
        &self,
        worktree_id: &str,
        expected_revision: u64,
    ) -> Result<(), SessionsApplicationError>;
    fn remove_safely(
        &self,
        worktree_id: &str,
        identity: &WorktreeIdentityView,
        token: &GateToken,
    ) -> Result<RemovalReportView, SessionsApplicationError>;
    fn observe(
        &self,
        worktree_id: &str,
    ) -> Result<Option<ObservationView>, SessionsApplicationError>;
    fn finalize_removed(
        &self,
        worktree_id: &str,
        session_ids: &[String],
    ) -> Result<(), SessionsApplicationError>;
    fn finalize_retained(
        &self,
        worktree_id: &str,
        session_ids: &[String],
    ) -> Result<(), SessionsApplicationError>;
    fn removal_refused(&self, worktree_id: &str) -> Result<(), SessionsApplicationError>;
    fn mark_attention(
        &self,
        worktree_id: &str,
        reason: &str,
    ) -> Result<(), SessionsApplicationError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionReference {
    pub(crate) session_id: String,
    pub(crate) title: String,
    pub(crate) archived: bool,
    pub(crate) loop_owned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ReferenceScan {
    pub(crate) sessions: Vec<SessionReference>,
    pub(crate) loop_runs: Vec<String>,
    pub(crate) complete: bool,
}

pub(crate) trait DeletionReferencePort: Send + Sync {
    /// Every persisted binding that resolves to `canonical_root` or inside it. `display_root`
    /// is matched as well for bindings whose directory cannot be canonicalized right now.
    fn references_to_root(
        &self,
        canonical_root: Option<&str>,
        display_root: &str,
    ) -> Result<ReferenceScan, SessionsApplicationError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QuiescenceReport {
    pub(crate) quiet: bool,
    pub(crate) blockers: Vec<String>,
}

pub(crate) trait SessionDeletionRuntimePort: Send + Sync {
    /// Stops everything this application runs for the session and waits, bounded, for it to be
    /// gone. A cancellation that was merely accepted is not quiet.
    fn quiesce(
        &self,
        session_id: &str,
        deadline: Duration,
    ) -> Result<QuiescenceReport, SessionsApplicationError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredPreview {
    pub(crate) preview: SessionDeletionPreview,
    pub(crate) expires_at_unix: i64,
    pub(crate) assessments: BTreeMap<String, WorktreeAssessment>,
}

pub(crate) trait DeletionPreviewStore: Send + Sync {
    fn put(&self, stored: StoredPreview);
    fn get(&self, preview_id: &str) -> Option<StoredPreview>;
    fn remove(&self, preview_id: &str);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewDeletionGroup {
    pub(crate) group_id: String,
    pub(crate) worktree_key: Option<String>,
    pub(crate) worktree_id: Option<String>,
    pub(crate) policy: WorktreeDeletionPolicy,
    pub(crate) session_ids: Vec<String>,
    pub(crate) retained_path: Option<String>,
    pub(crate) authorization: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewDeletionOperation {
    pub(crate) operation_id: String,
    pub(crate) request_id: String,
    pub(crate) request_hash: String,
    pub(crate) runtime_effect: DeletionRuntimeEffect,
    pub(crate) owner: DeletionOwner,
    pub(crate) created_at: String,
    pub(crate) operation_task_id: Option<String>,
    pub(crate) groups: Vec<NewDeletionGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JournalCreateOutcome {
    Created(SessionDeletionOperation),
    /// Same request id, same content: the earlier operation is the answer.
    Existing(SessionDeletionOperation),
    /// Same request id, different content.
    RequestConflict,
    /// A session in the request is already held by another operation.
    SessionClaimed {
        session_id: String,
        operation_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct GroupPatch {
    pub(crate) status: Option<DeletionGroupStatus>,
    pub(crate) phase: Option<DeletionPhase>,
    pub(crate) worktree_effect: Option<WorktreeEffect>,
    pub(crate) db_effect: Option<SessionDbEffect>,
    /// `Some(None)` clears the code.
    pub(crate) error_code: Option<Option<String>>,
    pub(crate) execution_snapshot: Option<serde_json::Value>,
    pub(crate) receipt: Option<serde_json::Value>,
    pub(crate) attempt: Option<u32>,
    pub(crate) policy: Option<WorktreeDeletionPolicy>,
    pub(crate) authorization: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperationPatch {
    pub(crate) outcome: Option<DeletionOutcome>,
    pub(crate) phase: Option<DeletionPhase>,
    pub(crate) error_code: Option<Option<String>>,
    pub(crate) completed: bool,
    pub(crate) owner: Option<DeletionOwner>,
    pub(crate) last_retry_request_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroupCompletion {
    pub(crate) revision: u64,
    pub(crate) active_session_cleared: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionDeletionClaim {
    pub(crate) session_id: String,
    pub(crate) operation_id: String,
    pub(crate) group_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroupSnapshot {
    pub(crate) execution_snapshot: Option<serde_json::Value>,
    pub(crate) receipt: Option<serde_json::Value>,
    pub(crate) authorization: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperationOwnership {
    pub(crate) owner: DeletionOwner,
    pub(crate) last_retry_request_id: Option<String>,
}

pub(crate) trait DeletionJournalPort: Send + Sync {
    /// One transaction: the operation, its groups, and an exclusive claim on every session.
    fn create(
        &self,
        operation: &NewDeletionOperation,
    ) -> Result<JournalCreateOutcome, SessionsApplicationError>;
    fn load(
        &self,
        operation_id: &str,
    ) -> Result<Option<SessionDeletionOperation>, SessionsApplicationError>;
    fn list_pending(&self) -> Result<Vec<SessionDeletionOperation>, SessionsApplicationError>;
    fn ownership(
        &self,
        operation_id: &str,
    ) -> Result<Option<OperationOwnership>, SessionsApplicationError>;
    fn update_operation(
        &self,
        operation_id: &str,
        expected_revision: u64,
        patch: &OperationPatch,
    ) -> Result<u64, SessionsApplicationError>;
    /// Compare-and-set on the group's own revision.
    fn update_group(
        &self,
        operation_id: &str,
        group_id: &str,
        expected_revision: u64,
        patch: &GroupPatch,
    ) -> Result<u64, SessionsApplicationError>;
    fn group_snapshot(
        &self,
        operation_id: &str,
        group_id: &str,
    ) -> Result<Option<GroupSnapshot>, SessionsApplicationError>;
    /// One transaction: the sessions and their cascades are deleted, the active selection is
    /// cleared only if it names one of them, the group becomes `Succeeded`, and its claims go.
    fn complete_group_deleting_sessions(
        &self,
        operation_id: &str,
        group_id: &str,
        expected_revision: u64,
        session_ids: &[String],
    ) -> Result<GroupCompletion, SessionsApplicationError>;
    fn release_group_claims(
        &self,
        operation_id: &str,
        group_id: &str,
    ) -> Result<(), SessionsApplicationError>;
    /// Takes the claims back for a retry. Conflicts if another operation holds any of them.
    fn reclaim_group(
        &self,
        operation_id: &str,
        group_id: &str,
        session_ids: &[String],
    ) -> Result<Option<SessionDeletionClaim>, SessionsApplicationError>;
    fn active_claim(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionDeletionClaim>, SessionsApplicationError>;
}

/// Consulted by the session service before it starts new work in a session.
pub(crate) trait SessionExecutionAdmissionPort: Send + Sync {
    fn ensure_session_admits_execution(
        &self,
        session_id: &str,
    ) -> Result<(), SessionsApplicationError>;
}

pub(crate) trait DeletionEventPort: Send + Sync {
    /// Published only after a commit, and only when the active selection really changed.
    fn active_session_cleared(&self);
    fn sessions_changed(&self);
}
