//! The coordinator's view of the outside world: the workspaces context through its published
//! API, the sessions table for references, an in-memory preview store, and the process lease.

use super::rows::{SessionRow, SESSION_SELECT};
use super::runtime_support::AgentSessionRuntimeAdapter;
use crate::contexts::operations::api::{OperationKind, OperationsApi};
use crate::contexts::operations::domain::OperationStatus;
use crate::contexts::sessions::application::{
    DeletionChangeSummary, DeletionCheckCompleteness, DeletionClockPort, DeletionIdPort,
    DeletionIgnoredSample, DeletionIgnoredSummary, DeletionOwner, DeletionOwnerPort,
    DeletionPreviewStore, DeletionReferencePort, DeletionWorkspacePort, GateOutcome, GateToken,
    ObservationView, ReferenceInput, ReferenceScan, RemovalOutcomeView, RemovalReportView,
    ResolvedWorktree, SessionRecord, SessionReference, SessionsApplicationError, StoredPreview,
    Tri, WorktreeAssessment, WorktreeDeletionPolicy, WorktreeIdentityView,
};
use crate::contexts::workspaces::api::{
    evaluate_cleanup, CheckCompleteness, GateOwner, GateRejection, Presence, ProbeBudget,
    ReferenceSummary, WorkspaceApi, WorkspaceError, WorktreeCleanupPolicy, WorktreeIdentity,
    WorktreeObservation, WorktreeRemovalOutcome, WorktreeSessionView,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use crate::platform::instance_lease::InstanceLease;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

// --- Workspace bridge --------------------------------------------------------------------------

#[derive(Clone)]
pub(crate) struct WorkspaceDeletionAdapter {
    workspaces: WorkspaceApi,
    operations: OperationsApi,
}

impl WorkspaceDeletionAdapter {
    pub(crate) fn new(workspaces: WorkspaceApi, operations: OperationsApi) -> Self {
        Self {
            workspaces,
            operations,
        }
    }

    /// Whether a succeeded session-creation operation for this session still exists. Bounded by
    /// the operations list the store keeps; an evicted record is simply absent evidence.
    fn creation_evidence(&self, session_id: &str) -> bool {
        self.operations.list().is_ok_and(|operations| {
            operations.iter().any(|operation| {
                operation.kind == OperationKind::Workspace
                    && operation.status == OperationStatus::Succeeded
                    && operation
                        .result
                        .as_ref()
                        .and_then(|result| result.get("id"))
                        .and_then(|id| id.as_str())
                        == Some(session_id)
            })
        })
    }
}

fn gate_claim(token: &GateToken) -> crate::contexts::workspaces::api::GateClaim {
    crate::contexts::workspaces::api::GateClaim {
        worktree_id: token.worktree_id.clone(),
        canonical_root: token.canonical_root.clone(),
        owner: GateOwner {
            instance_id: token.instance_id.clone(),
            epoch: token.epoch,
            operation_id: token.operation_id.clone(),
        },
        claimed_at: token.claimed_at.clone(),
    }
}

fn identity_view(identity: &WorktreeIdentity) -> WorktreeIdentityView {
    WorktreeIdentityView {
        canonical_root: identity.canonical_root.clone(),
        git_dir: identity.git_dir.clone(),
        common_dir: identity.common_dir.clone(),
        branch: identity.branch.clone(),
        head: identity.head.clone(),
        fs_identity: identity.fs_identity.clone(),
    }
}

fn completeness(value: CheckCompleteness) -> DeletionCheckCompleteness {
    match value {
        CheckCompleteness::Complete => DeletionCheckCompleteness::Complete,
        CheckCompleteness::Incomplete => DeletionCheckCompleteness::Incomplete,
    }
}

fn observation_view(observation: &WorktreeObservation) -> ObservationView {
    let tri = |presence: Presence| match presence {
        Presence::Present => Tri::Present,
        Presence::Absent => Tri::Absent,
        Presence::Unknown => Tri::Unknown,
    };
    ObservationView {
        root_present: tri(observation.root_present),
        registered: tri(observation.registered),
        confirmed_removed: observation.confirmed_removed(),
        confirmed_intact: observation.confirmed_intact(),
    }
}

fn policy_view(policy: WorktreeCleanupPolicy) -> WorktreeDeletionPolicy {
    match policy {
        WorktreeCleanupPolicy::Keep => WorktreeDeletionPolicy::Keep,
        WorktreeCleanupPolicy::RemoveSafe => WorktreeDeletionPolicy::RemoveSafe,
    }
}

impl DeletionWorkspacePort for WorkspaceDeletionAdapter {
    fn resolve(
        &self,
        session: &SessionRecord,
    ) -> Result<Option<ResolvedWorktree>, SessionsApplicationError> {
        let workspace = &session.workspace;
        // The operations list is read only for a session that has no managed record yet: that
        // is the only case in which the evidence is consulted, and the list is the whole store.
        let needs_evidence = workspace.worktree_path.is_some()
            && workspace.remote_workspace.is_none()
            && workspace.loop_ownership.is_none()
            && !self
                .workspaces
                .session_has_managed_worktree(session.id())
                .map_err(workspace_error)?;
        let view = WorktreeSessionView {
            session_id: session.id().to_string(),
            worktree_path: workspace.worktree_path.clone(),
            worktree_branch: workspace.worktree_branch.clone(),
            project_path: workspace.project_path.clone(),
            remote: workspace.remote_workspace.is_some(),
            loop_owned: workspace.loop_ownership.is_some(),
            creation_evidence: needs_evidence && self.creation_evidence(session.id()),
        };
        let resolution = self
            .workspaces
            .resolve_session_worktree(&view)
            .map_err(workspace_error)?;
        Ok(resolution.map(|resolution| ResolvedWorktree {
            key: resolution.group_key(),
            worktree_id: resolution.record.as_ref().map(|record| record.id.clone()),
            canonical_root: resolution.canonical_root.clone(),
            display_root: resolution.display_root.clone(),
            branch: resolution.branch.clone(),
            origin: resolution.origin.as_str().to_string(),
            provenance: resolution.provenance_reason.to_string(),
            resource_status: resolution
                .record
                .as_ref()
                .map(|record| record.status.as_str().to_string()),
        }))
    }

    fn assess(
        &self,
        worktree_id: &str,
        references: ReferenceInput,
        owner: Option<(&DeletionOwner, &str)>,
    ) -> Result<WorktreeAssessment, SessionsApplicationError> {
        let inspection = self
            .workspaces
            .inspect_managed_worktree(worktree_id, &ProbeBudget::DEFAULT)
            .map_err(workspace_error)?;
        let gate_owner = owner.map(|(owner, operation_id)| GateOwner {
            instance_id: owner.instance_id.clone(),
            epoch: owner.epoch,
            operation_id: operation_id.to_string(),
        });
        let gated = match inspection.record.identity.as_ref() {
            Some(identity) => self
                .workspaces
                .foreign_worktree_gate_holder(&identity.canonical_root, gate_owner.as_ref())
                .map_err(workspace_error)?
                .is_some(),
            None => false,
        };
        let evaluation = evaluate_cleanup(
            &inspection,
            ReferenceSummary {
                external_count: references.external_count,
                completeness: Some(if references.complete {
                    CheckCompleteness::Complete
                } else {
                    CheckCompleteness::Incomplete
                }),
            },
            gated,
        );
        let probe = &inspection.probe;
        Ok(WorktreeAssessment {
            identity: probe.identity.as_ref().map(identity_view),
            record_revision: inspection.record.revision,
            allowed_policies: evaluation
                .allowed_policies
                .iter()
                .map(|policy| policy_view(*policy))
                .collect(),
            blockers: evaluation
                .blockers
                .iter()
                .map(|blocker| (*blocker).to_string())
                .collect(),
            checks: completeness(evaluation.checks),
            requires_ignored_acknowledgement: evaluation.requires_ignored_acknowledgement,
            changes: probe.changes.as_ref().map(|changes| DeletionChangeSummary {
                tracked_modified: changes.tracked_modified,
                staged: changes.staged,
                conflicted: changes.conflicted,
                untracked: changes.untracked,
            }),
            ignored: probe
                .ignored
                .as_ref()
                .map(|ignored| DeletionIgnoredSummary {
                    total_entries: ignored.total_entries,
                    samples: ignored
                        .samples
                        .iter()
                        .map(|entry| DeletionIgnoredSample {
                            path: entry.path.clone(),
                            kind: entry.kind.to_string(),
                            size: entry.size,
                            modified_unix: entry.modified_unix,
                        })
                        .collect(),
                    samples_truncated: ignored.samples_truncated,
                    completeness: completeness(ignored.completeness),
                    fingerprint: ignored.fingerprint.clone(),
                }),
            resource_status: inspection.record.status.as_str().to_string(),
        })
    }

    fn bound_sessions(&self, worktree_id: &str) -> Result<Vec<String>, SessionsApplicationError> {
        self.workspaces
            .managed_worktree_sessions(worktree_id)
            .map_err(workspace_error)
    }

    fn claim_gate(
        &self,
        worktree_id: &str,
        owner: &DeletionOwner,
        operation_id: &str,
    ) -> Result<GateOutcome, SessionsApplicationError> {
        let gate_owner = GateOwner {
            instance_id: owner.instance_id.clone(),
            epoch: owner.epoch,
            operation_id: operation_id.to_string(),
        };
        match self
            .workspaces
            .claim_worktree_gate(worktree_id, &gate_owner)
        {
            Ok(claim) => Ok(GateOutcome::Claimed(GateToken {
                worktree_id: claim.worktree_id,
                canonical_root: claim.canonical_root,
                instance_id: claim.owner.instance_id,
                epoch: claim.owner.epoch,
                operation_id: claim.owner.operation_id,
                claimed_at: claim.claimed_at,
            })),
            Err(GateRejection::Held(holder)) => Ok(GateOutcome::Held {
                operation_id: holder.owner.operation_id,
            }),
            Err(GateRejection::Storage(message)) => {
                Err(SessionsApplicationError::Workspace(message))
            }
        }
    }

    fn release_gate(&self, token: &GateToken) -> Result<(), SessionsApplicationError> {
        self.workspaces
            .release_worktree_gate(&gate_claim(token))
            .map_err(workspace_error)
    }

    fn begin_removal(
        &self,
        worktree_id: &str,
        expected_revision: u64,
    ) -> Result<(), SessionsApplicationError> {
        self.workspaces
            .begin_worktree_removal(worktree_id, expected_revision)
            .map(|_| ())
            .map_err(workspace_error)
    }

    fn remove_safely(
        &self,
        worktree_id: &str,
        identity: &WorktreeIdentityView,
        token: &GateToken,
    ) -> Result<RemovalReportView, SessionsApplicationError> {
        let expected = WorktreeIdentity {
            canonical_root: identity.canonical_root.clone(),
            git_dir: identity.git_dir.clone(),
            common_dir: identity.common_dir.clone(),
            branch: identity.branch.clone(),
            head: identity.head.clone(),
            fs_identity: identity.fs_identity.clone(),
        };
        let report = self
            .workspaces
            .remove_worktree_safely(worktree_id, &expected, &gate_claim(token))
            .map_err(workspace_error)?;
        Ok(RemovalReportView {
            outcome: match report.outcome {
                WorktreeRemovalOutcome::Succeeded => RemovalOutcomeView::Succeeded,
                WorktreeRemovalOutcome::Refused { diagnostic, .. } => RemovalOutcomeView::Refused {
                    code: if diagnostic
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_')
                    {
                        diagnostic
                    } else {
                        crate::contexts::sessions::application::deletion_error_code::REMOVAL_REFUSED
                            .to_string()
                    },
                },
                WorktreeRemovalOutcome::TimedOut { .. } => RemovalOutcomeView::TimedOut,
                WorktreeRemovalOutcome::Unavailable(code) => {
                    RemovalOutcomeView::Unavailable { code }
                }
            },
            observation: observation_view(&report.observation),
        })
    }

    fn observe(
        &self,
        worktree_id: &str,
    ) -> Result<Option<ObservationView>, SessionsApplicationError> {
        Ok(self
            .workspaces
            .observe_managed_worktree(worktree_id)
            .map_err(workspace_error)?
            .as_ref()
            .map(observation_view))
    }

    fn finalize_removed(
        &self,
        worktree_id: &str,
        session_ids: &[String],
    ) -> Result<(), SessionsApplicationError> {
        self.workspaces
            .finalize_worktree_removed(worktree_id, session_ids)
            .map_err(workspace_error)
    }

    fn finalize_retained(
        &self,
        worktree_id: &str,
        session_ids: &[String],
    ) -> Result<(), SessionsApplicationError> {
        self.workspaces
            .finalize_worktree_retained(worktree_id, session_ids)
            .map_err(workspace_error)
    }

    fn removal_refused(&self, worktree_id: &str) -> Result<(), SessionsApplicationError> {
        self.workspaces
            .worktree_removal_refused(worktree_id)
            .map_err(workspace_error)
    }

    fn mark_attention(
        &self,
        worktree_id: &str,
        reason: &str,
    ) -> Result<(), SessionsApplicationError> {
        self.workspaces
            .mark_worktree_needs_attention(worktree_id, reason)
            .map_err(workspace_error)
    }
}

// --- References --------------------------------------------------------------------------------

/// Every session whose effective directory is the worktree or inside it, plus Loop runs that
/// recorded the same worktree path. Loop-owned sessions carry their ownership so the caller can
/// label them.
#[derive(Clone)]
pub(crate) struct SqliteDeletionReferences {
    database: NativeDatabase,
    runtime: AgentSessionRuntimeAdapter,
}

impl SqliteDeletionReferences {
    pub(crate) fn new(database: NativeDatabase, runtime: AgentSessionRuntimeAdapter) -> Self {
        Self { database, runtime }
    }

    fn connection(&self) -> Result<PooledSqlite, SessionsApplicationError> {
        self.database
            .connection()
            .map_err(|error| SessionsApplicationError::Repository(error.to_string()))
    }
}

impl DeletionReferencePort for SqliteDeletionReferences {
    fn references_to_root(
        &self,
        canonical_root: Option<&str>,
        display_root: &str,
    ) -> Result<ReferenceScan, SessionsApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare(&format!(
                "{SESSION_SELECT} WHERE remote_workspace_uri IS NULL"
            ))
            .map_err(|error| SessionsApplicationError::Repository(error.to_string()))?;
        let rows = statement
            .query_map([], SessionRow::read)
            .map_err(|error| SessionsApplicationError::Repository(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| SessionsApplicationError::Repository(error.to_string()))?;
        let mut scan = ReferenceScan {
            sessions: Vec::new(),
            loop_runs: Vec::new(),
            complete: true,
        };
        let canonical_root = canonical_root.map(Path::new);
        // Many sessions share a project path; each distinct path is resolved on disk once.
        let mut resolved: BTreeMap<String, Option<PathBuf>> = BTreeMap::new();
        for row in rows {
            let record = row.into_record()?;
            let workspace = &record.workspace;
            let candidates = [
                workspace.worktree_path.as_deref(),
                workspace.folder.as_deref(),
                workspace.project_path.as_deref(),
            ];
            let mut references = false;
            for candidate in candidates.into_iter().flatten() {
                let canonical_candidate = resolved
                    .entry(candidate.to_string())
                    .or_insert_with(|| Path::new(candidate).canonicalize().ok())
                    .as_deref();
                match points_into(candidate, canonical_candidate, canonical_root, display_root) {
                    Some(true) => {
                        references = true;
                        break;
                    }
                    Some(false) => {}
                    None => scan.complete = false,
                }
            }
            if references {
                scan.sessions.push(SessionReference {
                    session_id: record.id().to_string(),
                    title: record.aggregate.title().as_str().to_string(),
                    archived: record.aggregate.is_archived(),
                    loop_owned: workspace.loop_ownership.is_some(),
                });
            }
        }
        match self.runtime.loop_worktree_paths() {
            Ok(paths) => {
                for (run_id, path) in paths {
                    let canonical_candidate = Path::new(&path).canonicalize().ok();
                    match points_into(
                        &path,
                        canonical_candidate.as_deref(),
                        canonical_root,
                        display_root,
                    ) {
                        Some(true) => scan.loop_runs.push(run_id),
                        Some(false) => {}
                        None => scan.complete = false,
                    }
                }
            }
            Err(_) => scan.complete = false,
        }
        Ok(scan)
    }
}

/// `Some(true)` when `candidate` is the root or inside it, `Some(false)` when it is elsewhere,
/// `None` when the answer depends on a directory that could not be resolved.
fn points_into(
    candidate: &str,
    canonical_candidate: Option<&Path>,
    canonical_root: Option<&Path>,
    display_root: &str,
) -> Option<bool> {
    if let Some(canonical_candidate) = canonical_candidate {
        if let Some(root) = canonical_root {
            return Some(canonical_candidate.starts_with(root));
        }
        return Some(same_display(candidate, display_root));
    }
    // The candidate no longer exists on disk. A string match still counts; a mismatch is
    // conclusive only for the display form we have.
    if same_display(candidate, display_root)
        || canonical_root.is_some_and(|root| Path::new(candidate).starts_with(root))
        || display_contains(display_root, candidate)
    {
        return Some(true);
    }
    Some(false)
}

fn normalize_display(value: &str) -> String {
    crate::platform::filesystem::normalize_windows_extended_length_path(value)
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string()
}

fn same_display(left: &str, right: &str) -> bool {
    normalize_display(left) == normalize_display(right)
}

/// `candidate` is inside `root` on a path-component boundary: `/repo/wt2` is not in `/repo/wt`.
fn display_contains(root: &str, candidate: &str) -> bool {
    let root = normalize_display(root);
    let candidate = normalize_display(candidate);
    candidate
        .strip_prefix(root.as_str())
        .is_some_and(|rest| rest.starts_with('/'))
}

// --- Preview store, clock, ids, owner -----------------------------------------------------------

/// Previews live in memory only: they are a snapshot for one dialog, and a restart is exactly
/// the event that should invalidate every one of them.
#[derive(Default)]
pub(crate) struct InMemoryDeletionPreviewStore {
    entries: Mutex<BTreeMap<String, StoredPreview>>,
}

impl DeletionPreviewStore for InMemoryDeletionPreviewStore {
    fn put(&self, stored: StoredPreview) {
        if let Ok(mut entries) = self.entries.lock() {
            let now = crate::platform::clock::SystemClock
                .unix_seconds()
                .parse::<i64>()
                .unwrap_or(0);
            entries.retain(|_, entry| entry.expires_at_unix >= now);
            entries.insert(stored.preview.preview_id.clone(), stored);
        }
    }

    fn get(&self, preview_id: &str) -> Option<StoredPreview> {
        self.entries
            .lock()
            .ok()
            .and_then(|entries| entries.get(preview_id).cloned())
    }

    fn remove(&self, preview_id: &str) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(preview_id);
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemDeletionClock;

impl DeletionClockPort for SystemDeletionClock {
    fn now(&self) -> String {
        crate::platform::clock::SystemClock.rfc3339()
    }

    fn unix_now(&self) -> i64 {
        crate::platform::clock::SystemClock
            .unix_seconds()
            .parse::<i64>()
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct UuidDeletionIds;

impl DeletionIdPort for UuidDeletionIds {
    fn next_operation_id(&self) -> String {
        format!("session-deletion-{}", uuid::Uuid::new_v4())
    }

    fn next_group_id(&self) -> String {
        format!("deletion-group-{}", uuid::Uuid::new_v4())
    }

    fn next_preview_id(&self) -> String {
        format!("deletion-preview-{}", uuid::Uuid::new_v4())
    }
}

#[derive(Clone)]
pub(crate) struct LeaseDeletionOwner {
    lease: InstanceLease,
}

impl LeaseDeletionOwner {
    pub(crate) fn new(lease: InstanceLease) -> Self {
        Self { lease }
    }
}

impl DeletionOwnerPort for LeaseDeletionOwner {
    fn current(&self) -> DeletionOwner {
        DeletionOwner {
            instance_id: self.lease.id().to_string(),
            epoch: self.lease.epoch(),
        }
    }

    fn is_alive(&self, instance_id: &str) -> Option<bool> {
        self.lease.is_alive(instance_id).ok()
    }
}

fn workspace_error(error: WorkspaceError) -> SessionsApplicationError {
    match error {
        WorkspaceError::Domain(error) => SessionsApplicationError::Validation(error.to_string()),
        WorkspaceError::Validation(message) => SessionsApplicationError::Validation(message),
        WorkspaceError::Conflict(code) => SessionsApplicationError::Validation(code.to_string()),
        WorkspaceError::LaunchFailed(message) => SessionsApplicationError::WorkspaceLaunch(message),
        WorkspaceError::SessionNotFound(session_id) => {
            SessionsApplicationError::SessionNotFound(session_id)
        }
        WorkspaceError::PolicyDenied { session_id, action } => {
            SessionsApplicationError::Validation(format!(
                "Verifier session {session_id} cannot perform workspace action: {action}"
            ))
        }
        WorkspaceError::Repository(message)
        | WorkspaceError::Selection(message)
        | WorkspaceError::Filesystem(message)
        | WorkspaceError::Storage(message) => SessionsApplicationError::Workspace(message),
    }
}

#[cfg(test)]
mod tests {
    use super::points_into;
    use std::path::Path;

    #[test]
    fn a_missing_candidate_is_inside_the_root_only_on_a_component_boundary() {
        let root = "/repo/wt";
        assert_eq!(points_into("/repo/wt", None, None, root), Some(true));
        assert_eq!(points_into("/repo/wt/src", None, None, root), Some(true));
        assert_eq!(points_into("/repo/wt/", None, None, root), Some(true));
        // A sibling that merely shares the prefix is not a reference.
        assert_eq!(points_into("/repo/wt2", None, None, root), Some(false));
        assert_eq!(points_into("/repo/wt2/src", None, None, root), Some(false));
    }

    #[test]
    fn an_existing_candidate_is_judged_by_its_canonical_form() {
        let canonical_root = Path::new("/real/wt");
        assert_eq!(
            points_into(
                "/link/wt/src",
                Some(Path::new("/real/wt/src")),
                Some(canonical_root),
                "/link/wt"
            ),
            Some(true)
        );
        assert_eq!(
            points_into(
                "/link/wt2",
                Some(Path::new("/real/wt2")),
                Some(canonical_root),
                "/link/wt"
            ),
            Some(false)
        );
    }
}
