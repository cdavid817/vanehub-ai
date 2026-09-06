//! Provenance, inspection and conservative removal of ordinary-session worktrees.
//!
//! The service owns three facts nobody else may write: that a worktree is ours, that it is
//! being removed, and that it is gone. Sessions asks it questions and hands it permits; Git and
//! SQLite answer through ports. Nothing here deletes a directory except through the one
//! `WorktreeRemovalPort::remove` call, and that call is preceded by a persisted `Removing` state
//! and followed by an observation of what actually happened.

use super::worktree_cleanup_models::{
    GateClaim, GateHolder, GateOwner, GateRejection, ManagedWorktreeRepository, ProbeBudget,
    WorktreeCleanupClockPort, WorktreeIdPort, WorktreeInspection, WorktreeObservation,
    WorktreeProbe, WorktreeProbePort, WorktreeRemovalOutcome, WorktreeRemovalPort,
    WorktreeRemovalReport, WorktreeResolution, WorktreeSessionView, WorktreeUseGatePort,
    REMOVAL_TIMEOUT,
};
use super::worktree_cleanup_policy::reason;
use super::WorkspaceApplicationError;
use crate::contexts::workspaces::domain::{
    ManagedWorktree, ManagedWorktreeStatus, WorktreeIdentity, WorktreeOrigin,
};
use std::path::Path;
use std::sync::Arc;

pub(crate) const SESSION_BINDING_OWNER: &str = "owner";
pub(crate) const SESSION_BINDING_LEGACY: &str = "legacy";

#[derive(Clone)]
pub(crate) struct WorktreeCleanupService {
    repository: Arc<dyn ManagedWorktreeRepository>,
    probe: Arc<dyn WorktreeProbePort>,
    removal: Arc<dyn WorktreeRemovalPort>,
    gate: Arc<dyn WorktreeUseGatePort>,
    ids: Arc<dyn WorktreeIdPort>,
    clock: Arc<dyn WorktreeCleanupClockPort>,
}

impl WorktreeCleanupService {
    pub(crate) fn new(
        repository: Arc<dyn ManagedWorktreeRepository>,
        probe: Arc<dyn WorktreeProbePort>,
        removal: Arc<dyn WorktreeRemovalPort>,
        gate: Arc<dyn WorktreeUseGatePort>,
        ids: Arc<dyn WorktreeIdPort>,
        clock: Arc<dyn WorktreeCleanupClockPort>,
    ) -> Self {
        Self {
            repository,
            probe,
            removal,
            gate,
            ids,
            clock,
        }
    }

    /// Records that the application is about to create a worktree. Persisted *before* Git runs:
    /// a record that failed to write means Git does not run either.
    pub(crate) fn register_intent(
        &self,
        origin: WorktreeOrigin,
        project_root: &str,
        requested_root: &str,
        creation_operation_id: Option<&str>,
    ) -> Result<ManagedWorktree, WorkspaceApplicationError> {
        let record = ManagedWorktree::provisioning(
            self.ids.next_worktree_id(),
            origin,
            project_root.to_string(),
            requested_root.to_string(),
            creation_operation_id.map(str::to_string),
            self.clock.now(),
        )?;
        self.repository.insert(&record)?;
        Ok(record)
    }

    /// Binds Git's own account of the new worktree to the intent and to its session.
    ///
    /// A probe that cannot confirm the identity, or a binding that fails to persist, leaves the
    /// record in `NeedsAttention` rather than deleting anything: the directory was created and
    /// the user's work is in it.
    pub(crate) fn confirm_created(
        &self,
        worktree_id: &str,
        session_id: &str,
    ) -> Result<ManagedWorktree, WorkspaceApplicationError> {
        let mut record = self.require(worktree_id)?;
        let probe = self
            .probe
            .probe_identity(Path::new(&record.requested_root), &ProbeBudget::DEFAULT);
        let expected_revision = record.revision;
        match probe.identity {
            Some(identity) if probe.is_linked && probe.registered => {
                record.confirm_created(identity, self.clock.now())?;
            }
            _ => {
                record.mark_needs_attention(
                    probe.failure.unwrap_or("post_create_identity_unverified"),
                    self.clock.now(),
                );
                self.repository.save(&record, expected_revision)?;
                return Ok(record);
            }
        }
        if !self.repository.save(&record, expected_revision)? {
            return Err(WorkspaceApplicationError::Conflict(
                "worktree_revision_conflict",
            ));
        }
        if let Err(error) =
            self.repository
                .bind_session(&record.id, session_id, SESSION_BINDING_OWNER)
        {
            let expected_revision = record.revision;
            record.mark_needs_attention("session_binding_failed", self.clock.now());
            let _ = self.repository.save(&record, expected_revision);
            return Err(error);
        }
        Ok(record)
    }

    pub(crate) fn mark_needs_attention(
        &self,
        worktree_id: &str,
        reason: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        let mut record = self.require(worktree_id)?;
        let expected_revision = record.revision;
        record.mark_needs_attention(reason, self.clock.now());
        self.repository.save(&record, expected_revision)?;
        Ok(())
    }

    /// Whether a managed record already names this session. Callers use it to skip gathering
    /// legacy evidence that `resolve_for_session` would never read.
    pub(crate) fn has_record_for_session(
        &self,
        session_id: &str,
    ) -> Result<bool, WorkspaceApplicationError> {
        Ok(self.repository.find_by_session(session_id)?.is_some())
    }

    /// Which worktree, if any, a session is bound to, and whether the application may act on it.
    ///
    /// A session that predates provenance tracking is verified on demand: it needs its own
    /// worktree metadata, surviving evidence of a successful creation, and a current Git identity
    /// that agrees with both. Anything less is reported as unverified and only ever kept.
    pub(crate) fn resolve_for_session(
        &self,
        view: &WorktreeSessionView,
    ) -> Result<Option<WorktreeResolution>, WorkspaceApplicationError> {
        if view.remote {
            return Ok(None);
        }
        let Some(worktree_path) = view
            .worktree_path
            .as_deref()
            .filter(|p| !p.trim().is_empty())
        else {
            return Ok(None);
        };
        let canonical_root = self.probe.canonical_root(worktree_path);
        let record = match self.repository.find_by_session(&view.session_id)? {
            Some(record) => Some(record),
            None => match canonical_root.as_deref() {
                Some(root) => self.repository.find_by_root(root)?,
                None => None,
            }
            .or(self.repository.find_by_root(worktree_path)?),
        };
        if let Some(record) = record {
            let origin = record.origin;
            let branch = record
                .identity
                .as_ref()
                .and_then(|identity| identity.branch.clone())
                .or_else(|| view.worktree_branch.clone());
            return Ok(Some(WorktreeResolution {
                canonical_root: record
                    .identity
                    .as_ref()
                    .map(|identity| identity.canonical_root.clone())
                    .or(canonical_root),
                display_root: worktree_path.to_string(),
                branch,
                origin,
                provenance_reason: record.provenance.as_str(),
                record: Some(record),
            }));
        }
        if view.loop_owned {
            return Ok(Some(WorktreeResolution {
                record: None,
                canonical_root,
                display_root: worktree_path.to_string(),
                branch: view.worktree_branch.clone(),
                origin: WorktreeOrigin::Loop,
                provenance_reason: reason::ORIGIN_NOT_ORDINARY,
            }));
        }
        let unverified = |canonical_root: Option<String>| WorktreeResolution {
            record: None,
            canonical_root,
            display_root: worktree_path.to_string(),
            branch: view.worktree_branch.clone(),
            origin: WorktreeOrigin::OrdinarySession,
            provenance_reason: reason::PROVENANCE_UNVERIFIED,
        };
        let (Some(root), Some(expected_branch), Some(project_path), true) = (
            canonical_root.clone(),
            view.worktree_branch.as_deref(),
            view.project_path.as_deref(),
            view.creation_evidence,
        ) else {
            return Ok(Some(unverified(canonical_root)));
        };
        let probe = self
            .probe
            .probe_identity(Path::new(&root), &ProbeBudget::DEFAULT);
        let Some(identity) = probe.identity.clone() else {
            return Ok(Some(unverified(canonical_root)));
        };
        let anchor_matches_project = probe
            .anchor
            .as_deref()
            .zip(self.probe.canonical_root(project_path))
            .is_some_and(|(anchor, project)| same_path(anchor, &project));
        let branch_matches = identity.branch.as_deref() == Some(expected_branch);
        if !(probe.is_linked && probe.registered && branch_matches && anchor_matches_project) {
            return Ok(Some(unverified(canonical_root)));
        }
        let record = ManagedWorktree::legacy_verified(
            self.ids.next_worktree_id(),
            project_path.to_string(),
            identity,
            None,
            self.clock.now(),
        )?;
        self.repository.insert(&record)?;
        self.repository
            .bind_session(&record.id, &view.session_id, SESSION_BINDING_LEGACY)?;
        Ok(Some(WorktreeResolution {
            canonical_root: Some(root),
            display_root: worktree_path.to_string(),
            branch: view.worktree_branch.clone(),
            origin: WorktreeOrigin::OrdinarySession,
            provenance_reason: record.provenance.as_str(),
            record: Some(record),
        }))
    }

    /// Read-only. Probes the recorded root and says whether Git still describes the same object.
    pub(crate) fn inspect(
        &self,
        worktree_id: &str,
        budget: &ProbeBudget,
    ) -> Result<WorktreeInspection, WorkspaceApplicationError> {
        let record = self.require(worktree_id)?;
        let Some(recorded) = record.identity.clone() else {
            return Ok(WorktreeInspection {
                record,
                probe: WorktreeProbe::failed(reason::PROVENANCE_UNVERIFIED),
                identity_matches: false,
            });
        };
        let probe = self
            .probe
            .probe(Path::new(&recorded.canonical_root), budget);
        let identity_matches = probe
            .identity
            .as_ref()
            .is_some_and(|current| identities_agree(current, &recorded));
        Ok(WorktreeInspection {
            record,
            probe,
            identity_matches,
        })
    }

    pub(crate) fn bound_sessions(
        &self,
        worktree_id: &str,
    ) -> Result<Vec<String>, WorkspaceApplicationError> {
        self.repository.bound_sessions(worktree_id)
    }

    pub(crate) fn claim_gate(
        &self,
        worktree_id: &str,
        owner: &GateOwner,
    ) -> Result<GateClaim, GateRejection> {
        let record = self
            .require(worktree_id)
            .map_err(|error| GateRejection::Storage(error.to_string()))?;
        let root = record
            .identity
            .as_ref()
            .map(|identity| identity.canonical_root.clone())
            .unwrap_or(record.requested_root);
        self.gate.claim(worktree_id, &root, owner)
    }

    pub(crate) fn release_gate(&self, claim: &GateClaim) -> Result<(), WorkspaceApplicationError> {
        self.gate.release(claim)
    }

    /// The holder of a gate over `canonical_root`, if it is not `owner` and still alive. A dead
    /// holder's row is reported as `None` so a crashed instance does not freeze a directory
    /// forever; a holder whose liveness cannot be determined is reported as present.
    pub(crate) fn foreign_gate_holder(
        &self,
        canonical_root: &str,
        owner: Option<&GateOwner>,
    ) -> Result<Option<GateHolder>, WorkspaceApplicationError> {
        let Some(holder) = self.gate.holder_for_root(canonical_root)? else {
            return Ok(None);
        };
        if owner.is_some_and(|owner| {
            owner.instance_id == holder.owner.instance_id
                && owner.operation_id == holder.owner.operation_id
        }) {
            return Ok(None);
        }
        match self.gate.owner_is_alive(&holder) {
            Ok(false) => Ok(None),
            _ => Ok(Some(holder)),
        }
    }

    /// Whether any live instance is mid-cleanup on the directory at `path` or a parent of it.
    pub(crate) fn is_path_gated(&self, path: &str) -> Result<bool, WorkspaceApplicationError> {
        // A path that does not exist yet is judged by its nearest existing ancestor: a session
        // bound to a subdirectory of a gated worktree is still inside the gated worktree.
        let mut candidate = Some(Path::new(path).to_path_buf());
        let mut root = None;
        while let Some(current) = candidate {
            if let Some(found) = self.probe.canonical_root(&current.to_string_lossy()) {
                root = Some(found);
                break;
            }
            candidate = current.parent().map(Path::to_path_buf);
        }
        let Some(root) = root else {
            return Ok(false);
        };
        let mut current = Some(Path::new(&root).to_path_buf());
        while let Some(candidate) = current {
            if self
                .foreign_gate_holder(&candidate.to_string_lossy(), None)?
                .is_some()
            {
                return Ok(true);
            }
            current = candidate.parent().map(Path::to_path_buf);
        }
        Ok(false)
    }

    /// Persists `Removing` for a record whose caller has already revalidated everything. The
    /// revision check is what stops two operations from both believing they own the transition.
    pub(crate) fn begin_removal(
        &self,
        worktree_id: &str,
        expected_revision: u64,
    ) -> Result<ManagedWorktree, WorkspaceApplicationError> {
        let mut record = self.require(worktree_id)?;
        if record.revision != expected_revision {
            return Err(WorkspaceApplicationError::Conflict(
                "worktree_revision_conflict",
            ));
        }
        // The transition itself re-reads Git: a caller's earlier revalidation is not trusted to
        // still hold by the time the record says "removing".
        let still_the_same = record.identity.as_ref().is_some_and(|recorded| {
            self.probe
                .probe_identity(Path::new(&recorded.canonical_root), &ProbeBudget::DEFAULT)
                .identity
                .as_ref()
                .is_some_and(|current| identities_agree(current, recorded))
        });
        if !still_the_same {
            return Err(WorkspaceApplicationError::Conflict(
                reason::IDENTITY_MISMATCH,
            ));
        }
        record.begin_removal(self.clock.now())?;
        if !self.repository.save(&record, expected_revision)? {
            return Err(WorkspaceApplicationError::Conflict(
                "worktree_revision_conflict",
            ));
        }
        Ok(record)
    }

    /// The one destructive step. Requires a `Removing` record, a gate claim for it, and a probe
    /// taken *now* that still equals `expected`; runs `git worktree remove` from the anchor and
    /// reports what the world looks like afterwards. The report is the truth; the exit code is
    /// only one input to it.
    pub(crate) fn remove_safely(
        &self,
        worktree_id: &str,
        expected: &WorktreeIdentity,
        claim: &GateClaim,
    ) -> Result<WorktreeRemovalReport, WorkspaceApplicationError> {
        let record = self.require(worktree_id)?;
        if record.status != ManagedWorktreeStatus::Removing {
            return Err(WorkspaceApplicationError::Conflict("worktree_not_removing"));
        }
        if claim.worktree_id != worktree_id {
            return Err(WorkspaceApplicationError::Conflict("gate_mismatch"));
        }
        let probe = self
            .probe
            .probe_identity(Path::new(&expected.canonical_root), &ProbeBudget::DEFAULT);
        let anchor = match (&probe.identity, &probe.anchor) {
            (Some(current), Some(anchor)) if identities_agree(current, expected) => anchor.clone(),
            _ => {
                return Ok(WorktreeRemovalReport {
                    outcome: WorktreeRemovalOutcome::Refused {
                        exit_code: None,
                        diagnostic: reason::IDENTITY_MISMATCH.to_string(),
                    },
                    observation: self
                        .probe
                        .observe(expected, probe.anchor.as_deref().map(Path::new)),
                });
            }
        };
        let target = Path::new(&expected.canonical_root);
        if Path::new(&anchor).starts_with(target) {
            return Err(WorkspaceApplicationError::Conflict(reason::NO_ANCHOR));
        }
        let outcome = self
            .removal
            .remove(Path::new(&anchor), target, REMOVAL_TIMEOUT);
        let observation = self.probe.observe(expected, Some(Path::new(&anchor)));
        Ok(WorktreeRemovalReport {
            outcome,
            observation,
        })
    }

    pub(crate) fn observe(
        &self,
        worktree_id: &str,
    ) -> Result<Option<WorktreeObservation>, WorkspaceApplicationError> {
        let record = self.require(worktree_id)?;
        let Some(identity) = record.identity.as_ref() else {
            return Ok(None);
        };
        let anchor = self
            .probe
            .probe_identity(Path::new(&identity.canonical_root), &ProbeBudget::DEFAULT)
            .anchor;
        Ok(Some(
            self.probe
                .observe(identity, anchor.as_deref().map(Path::new)),
        ))
    }

    /// Removal confirmed: the record is `Removed` and the deleted sessions are unbound.
    pub(crate) fn finalize_removed(
        &self,
        worktree_id: &str,
        session_ids: &[String],
    ) -> Result<(), WorkspaceApplicationError> {
        let mut record = self.require(worktree_id)?;
        if record.status != ManagedWorktreeStatus::Removed {
            let expected_revision = record.revision;
            record.mark_removed(self.clock.now())?;
            self.repository.save(&record, expected_revision)?;
        }
        self.repository.unbind_sessions(worktree_id, session_ids)
    }

    /// The user kept the directory: the record survives the sessions that pointed at it.
    pub(crate) fn finalize_retained(
        &self,
        worktree_id: &str,
        session_ids: &[String],
    ) -> Result<(), WorkspaceApplicationError> {
        let mut record = self.require(worktree_id)?;
        if matches!(
            record.status,
            ManagedWorktreeStatus::Attached | ManagedWorktreeStatus::Retained
        ) {
            let expected_revision = record.revision;
            record.mark_retained(self.clock.now())?;
            self.repository.save(&record, expected_revision)?;
        }
        self.repository.unbind_sessions(worktree_id, session_ids)
    }

    /// Removal was refused and the directory observed intact: back to `Attached`.
    pub(crate) fn removal_refused(
        &self,
        worktree_id: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        let mut record = self.require(worktree_id)?;
        if record.status == ManagedWorktreeStatus::Removing {
            let expected_revision = record.revision;
            record.removal_refused(self.clock.now())?;
            self.repository.save(&record, expected_revision)?;
        }
        Ok(())
    }

    fn require(&self, worktree_id: &str) -> Result<ManagedWorktree, WorkspaceApplicationError> {
        self.repository.find(worktree_id)?.ok_or_else(|| {
            WorkspaceApplicationError::Validation(format!(
                "Managed worktree not found: {worktree_id}"
            ))
        })
    }
}

/// Root, admin directory and common directory must all agree; the branch and head may move
/// between probe and execution and are checked separately by the policy.
fn identities_agree(current: &WorktreeIdentity, recorded: &WorktreeIdentity) -> bool {
    same_path(&current.canonical_root, &recorded.canonical_root)
        && same_path(&current.git_dir, &recorded.git_dir)
        && same_path(&current.common_dir, &recorded.common_dir)
        && match (&current.fs_identity, &recorded.fs_identity) {
            (Some(left), Some(right)) => left == right,
            _ => true,
        }
}

fn same_path(left: &str, right: &str) -> bool {
    normalize(left) == normalize(right)
}

/// Comparison form only, never an execution target: the extended-length prefix is dropped and
/// separators unified so two spellings of one directory compare equal. Case is preserved — a
/// case-sensitive volume really does distinguish them.
fn normalize(path: &str) -> String {
    let stripped = path
        .strip_prefix(r"\\?\UNC\")
        .map(|rest| format!(r"\\{rest}"))
        .or_else(|| path.strip_prefix(r"\\?\").map(str::to_string))
        .unwrap_or_else(|| path.to_string());
    stripped
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string()
}
