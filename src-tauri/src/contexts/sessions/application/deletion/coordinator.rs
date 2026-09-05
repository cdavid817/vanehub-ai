//! Previews, authorizes, executes and recovers session deletions.
//!
//! The coordinator is the only path that deletes a session, and the only caller of the one
//! workspace method that removes a directory. Its order of operations is the safety argument:
//! journal before side effect, quiesce before revalidation, revalidation before removal,
//! observation before finalization — and a database failure after a Git success is recorded as
//! exactly that rather than rolled back into a fiction.

use super::models::{
    error_code, DeletionCheckCompleteness, DeletionExternalReference, DeletionGroupResult,
    DeletionGroupStatus, DeletionOutcome, DeletionPhase, DeletionPreviewSession,
    DeletionPreviewWorktree, DeletionRuntimeEffect, DeletionWorkspaceKind,
    ExecuteSessionDeletionRequest, PreviewSessionDeletionRequest, RetrySessionDeletionRequest,
    SessionDbEffect, SessionDeletionHandle, SessionDeletionOperation, SessionDeletionPreview,
    WorktreeDeletionPolicy, WorktreeEffect, PREVIEW_TTL_SECONDS, QUIESCE_DEADLINE_SECONDS,
};
use super::policy::{aggregate_outcome, normalize_selection, request_hash, resolve_choices};
use super::ports::{
    DeletionClockPort, DeletionEventPort, DeletionIdPort, DeletionJournalPort, DeletionOwner,
    DeletionOwnerPort, DeletionPreviewStore, DeletionReferencePort, DeletionWorkspacePort,
    GateOutcome, GateToken, GroupPatch, JournalCreateOutcome, NewDeletionGroup,
    NewDeletionOperation, OperationPatch, ReferenceInput, ReferenceScan, RemovalOutcomeView,
    ResolvedWorktree, SessionDeletionRuntimePort, StoredPreview, WorktreeAssessment,
    WorktreeIdentityView,
};
use crate::contexts::sessions::application::{
    SessionApplicationLog, SessionApplicationLogLevel, SessionLoggingPort, SessionRecord,
    SessionRepository, SessionsApplicationError,
};
use crate::contexts::sessions::domain::SessionId;
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

const LOG_CATEGORY: &str = "session.delete";

#[derive(Clone)]
pub(crate) struct SessionDeletionPorts {
    pub(crate) sessions: Arc<dyn SessionRepository>,
    pub(crate) journal: Arc<dyn DeletionJournalPort>,
    pub(crate) runtime: Arc<dyn SessionDeletionRuntimePort>,
    pub(crate) workspace: Arc<dyn DeletionWorkspacePort>,
    pub(crate) references: Arc<dyn DeletionReferencePort>,
    pub(crate) previews: Arc<dyn DeletionPreviewStore>,
    pub(crate) clock: Arc<dyn DeletionClockPort>,
    pub(crate) ids: Arc<dyn DeletionIdPort>,
    pub(crate) owner: Arc<dyn DeletionOwnerPort>,
    pub(crate) logging: Arc<dyn SessionLoggingPort>,
    pub(crate) events: Arc<dyn DeletionEventPort>,
}

#[derive(Clone)]
pub(crate) struct SessionDeletionCoordinator {
    ports: SessionDeletionPorts,
    runtime_effect: DeletionRuntimeEffect,
}

impl SessionDeletionCoordinator {
    pub(crate) fn new(ports: SessionDeletionPorts) -> Self {
        Self {
            ports,
            runtime_effect: DeletionRuntimeEffect::Native,
        }
    }

    // --- Admission -------------------------------------------------------------------------

    /// Whether the session may start new work. A claimed session is mid-deletion.
    pub(crate) fn ensure_session_admits_execution(
        &self,
        session_id: &str,
    ) -> Result<(), SessionsApplicationError> {
        match self.ports.journal.active_claim(session_id)? {
            Some(_) => Err(SessionsApplicationError::Validation(
                error_code::SESSION_CLAIMED.to_string(),
            )),
            None => Ok(()),
        }
    }

    // --- Preview ----------------------------------------------------------------------------

    /// Read-only. Resolves the selection to sessions and deduplicated worktree rows, inspects
    /// each verified worktree, and records the facts the execution will be checked against.
    pub(crate) fn preview(
        &self,
        request: PreviewSessionDeletionRequest,
    ) -> Result<SessionDeletionPreview, SessionsApplicationError> {
        let session_ids = normalize_selection(&request.session_ids)?;
        let active_id = self
            .ports
            .sessions
            .active_session()?
            .map(|session| session.id().to_string());
        let mut sessions = Vec::new();
        let mut resolutions: BTreeMap<String, (ResolvedWorktree, Vec<String>)> = BTreeMap::new();
        for session_id in &session_ids {
            let record = self.load(session_id)?;
            let resolution = self.ports.workspace.resolve(&record)?;
            let workspace_kind = workspace_kind(&record, resolution.as_ref());
            let display_path = match &resolution {
                Some(resolution) => Some(resolution.display_root.clone()),
                None => record
                    .workspace
                    .remote_workspace
                    .as_ref()
                    .map(|remote| remote.uri.clone())
                    .or_else(|| record.workspace.folder.clone())
                    .or_else(|| record.workspace.project_path.clone()),
            };
            let worktree_key = resolution.as_ref().map(|resolution| resolution.key.clone());
            if let Some(resolution) = resolution {
                resolutions
                    .entry(resolution.key.clone())
                    .or_insert_with(|| (resolution, Vec::new()))
                    .1
                    .push(session_id.clone());
            }
            sessions.push(DeletionPreviewSession {
                session_id: session_id.clone(),
                title: record.aggregate.title().as_str().to_string(),
                archived: record.aggregate.is_archived(),
                active: active_id.as_deref() == Some(session_id.as_str()),
                workspace_kind,
                worktree_key,
                display_path,
            });
        }
        let mut worktrees = Vec::new();
        let mut assessments = BTreeMap::new();
        for (key, (resolution, group_sessions)) in resolutions {
            let (row, assessment) = self.preview_worktree(&key, &resolution, &group_sessions)?;
            if let Some(assessment) = assessment {
                assessments.insert(key.clone(), assessment);
            }
            worktrees.push(row);
        }
        let now_unix = self.ports.clock.unix_now();
        let preview = SessionDeletionPreview {
            preview_id: self.ports.ids.next_preview_id(),
            runtime_effect: self.runtime_effect,
            created_at: self.ports.clock.now(),
            expires_at: format_unix(now_unix + PREVIEW_TTL_SECONDS),
            sessions,
            worktrees,
        };
        self.ports.previews.put(StoredPreview {
            preview: preview.clone(),
            expires_at_unix: now_unix + PREVIEW_TTL_SECONDS,
            assessments,
        });
        Ok(preview)
    }

    fn preview_worktree(
        &self,
        key: &str,
        resolution: &ResolvedWorktree,
        group_sessions: &[String],
    ) -> Result<(DeletionPreviewWorktree, Option<WorktreeAssessment>), SessionsApplicationError>
    {
        let (scan, external_references) = self.external_references(resolution, group_sessions)?;
        let Some(worktree_id) = resolution.worktree_id.as_deref() else {
            return Ok((
                DeletionPreviewWorktree {
                    worktree_key: key.to_string(),
                    worktree_id: None,
                    display_path: resolution.display_root.clone(),
                    branch: resolution.branch.clone(),
                    session_ids: group_sessions.to_vec(),
                    external_references,
                    allowed_policies: vec![WorktreeDeletionPolicy::Keep],
                    blockers: vec![resolution.provenance.clone()],
                    checks: if scan.complete {
                        DeletionCheckCompleteness::Complete
                    } else {
                        DeletionCheckCompleteness::Incomplete
                    },
                    changes: None,
                    ignored: None,
                    requires_ignored_acknowledgement: false,
                    origin: resolution.origin.clone(),
                    provenance: resolution.provenance.clone(),
                    resource_status: resolution.resource_status.clone(),
                },
                None,
            ));
        };
        let assessment = self.ports.workspace.assess(
            worktree_id,
            ReferenceInput {
                external_count: external_references.len(),
                complete: scan.complete,
            },
            None,
        )?;
        let row = DeletionPreviewWorktree {
            worktree_key: key.to_string(),
            worktree_id: Some(worktree_id.to_string()),
            display_path: resolution.display_root.clone(),
            branch: assessment
                .identity
                .as_ref()
                .and_then(|identity| identity.branch.clone())
                .or_else(|| resolution.branch.clone()),
            session_ids: group_sessions.to_vec(),
            external_references,
            allowed_policies: assessment.allowed_policies.clone(),
            blockers: assessment.blockers.clone(),
            checks: assessment.checks,
            changes: assessment.changes.clone(),
            ignored: assessment.ignored.clone(),
            requires_ignored_acknowledgement: assessment.requires_ignored_acknowledgement,
            origin: resolution.origin.clone(),
            provenance: resolution.provenance.clone(),
            resource_status: Some(assessment.resource_status.clone()),
        };
        Ok((row, Some(assessment)))
    }

    /// Everything that binds the directory and is *not* in this group: other sessions, Loop
    /// runs, and sessions the resource itself remembers. A scan that could not complete is
    /// reported incomplete rather than empty.
    fn external_references(
        &self,
        resolution: &ResolvedWorktree,
        group_sessions: &[String],
    ) -> Result<(ReferenceScan, Vec<DeletionExternalReference>), SessionsApplicationError> {
        let mut scan = self.ports.references.references_to_root(
            resolution.canonical_root.as_deref(),
            &resolution.display_root,
        )?;
        let mut references = Vec::new();
        for session in &scan.sessions {
            if group_sessions.contains(&session.session_id) {
                continue;
            }
            references.push(DeletionExternalReference {
                kind: if session.loop_owned {
                    "loop_session".to_string()
                } else if session.archived {
                    "archived_session".to_string()
                } else {
                    "session".to_string()
                },
                id: session.session_id.clone(),
                label: session.title.clone(),
            });
        }
        for run_id in &scan.loop_runs {
            references.push(DeletionExternalReference {
                kind: "loop_run".to_string(),
                id: run_id.clone(),
                label: run_id.clone(),
            });
        }
        if let Some(worktree_id) = resolution.worktree_id.as_deref() {
            match self.ports.workspace.bound_sessions(worktree_id) {
                Ok(bound) => {
                    for session_id in bound {
                        if group_sessions.contains(&session_id)
                            || references
                                .iter()
                                .any(|reference| reference.id == session_id)
                        {
                            continue;
                        }
                        // A binding whose session no longer exists is stale, not a reference.
                        if self
                            .ports
                            .sessions
                            .find(&SessionId::parse(&session_id)?)?
                            .is_none()
                        {
                            continue;
                        }
                        references.push(DeletionExternalReference {
                            kind: "bound_session".to_string(),
                            id: session_id.clone(),
                            label: session_id,
                        });
                    }
                }
                Err(_) => scan.complete = false,
            }
        }
        Ok((scan, references))
    }

    // --- Execute ----------------------------------------------------------------------------

    /// Turns a preview plus explicit choices into a journaled operation. Nothing is stopped or
    /// deleted here; `run` does that, after this returns a handle.
    pub(crate) fn execute(
        &self,
        request: ExecuteSessionDeletionRequest,
    ) -> Result<SessionDeletionHandle, SessionsApplicationError> {
        let request_id = request.request_id.trim().to_string();
        if request_id.is_empty() {
            return Err(SessionsApplicationError::Validation(
                "Deletion request id cannot be empty.".to_string(),
            ));
        }
        let Some(stored) = self.ports.previews.get(&request.preview_id) else {
            return Err(SessionsApplicationError::Validation(
                error_code::PREVIEW_EXPIRED.to_string(),
            ));
        };
        if stored.expires_at_unix < self.ports.clock.unix_now() {
            self.ports.previews.remove(&request.preview_id);
            return Err(SessionsApplicationError::Validation(
                error_code::PREVIEW_EXPIRED.to_string(),
            ));
        }
        let session_ids: Vec<String> = stored
            .preview
            .sessions
            .iter()
            .map(|session| session.session_id.clone())
            .collect();
        let resolved = resolve_choices(&stored.preview.worktrees, &request.worktree_choices)?;
        let hash = request_hash(&session_ids, &resolved);
        let groups = self.build_groups(&stored, &resolved);
        let owner = self.ports.owner.current();
        let operation = NewDeletionOperation {
            operation_id: self.ports.ids.next_operation_id(),
            request_id: request_id.clone(),
            request_hash: hash,
            runtime_effect: self.runtime_effect,
            owner,
            created_at: self.ports.clock.now(),
            operation_task_id: None,
            groups,
        };
        match self.ports.journal.create(&operation)? {
            JournalCreateOutcome::Created(created) => {
                self.ports.previews.remove(&request.preview_id);
                self.log(
                    SessionApplicationLogLevel::Info,
                    format!(
                        "Session deletion accepted: {} group(s), request {}",
                        created.groups.len(),
                        request_id
                    ),
                    Some(&created.operation_id),
                );
                Ok(handle(&created, false))
            }
            JournalCreateOutcome::Existing(existing) => Ok(handle(&existing, true)),
            JournalCreateOutcome::RequestConflict => Err(SessionsApplicationError::Validation(
                error_code::REQUEST_ID_CONFLICT.to_string(),
            )),
            JournalCreateOutcome::SessionClaimed { .. } => Err(
                SessionsApplicationError::Validation(error_code::SESSION_CLAIMED.to_string()),
            ),
        }
    }

    fn build_groups(
        &self,
        stored: &StoredPreview,
        resolved: &[(String, WorktreeDeletionPolicy, Option<String>)],
    ) -> Vec<NewDeletionGroup> {
        let mut groups = Vec::new();
        for worktree in &stored.preview.worktrees {
            let (policy, acknowledged) = resolved
                .iter()
                .find(|(key, _, _)| *key == worktree.worktree_key)
                .map(|(_, policy, ack)| (*policy, ack.clone()))
                .unwrap_or((WorktreeDeletionPolicy::Keep, None));
            let authorization = stored
                .assessments
                .get(&worktree.worktree_key)
                .filter(|_| policy == WorktreeDeletionPolicy::RemoveSafe)
                .map(|assessment| {
                    serde_json::json!({
                        "identity": assessment.identity,
                        "recordRevision": assessment.record_revision,
                        "ignoredFingerprint": acknowledged,
                    })
                });
            groups.push(NewDeletionGroup {
                group_id: self.ports.ids.next_group_id(),
                worktree_key: Some(worktree.worktree_key.clone()),
                worktree_id: worktree.worktree_id.clone(),
                policy,
                session_ids: worktree.session_ids.clone(),
                retained_path: Some(worktree.display_path.clone()),
                authorization,
            });
        }
        for session in &stored.preview.sessions {
            if session.worktree_key.is_some() {
                continue;
            }
            groups.push(NewDeletionGroup {
                group_id: self.ports.ids.next_group_id(),
                worktree_key: None,
                worktree_id: None,
                policy: WorktreeDeletionPolicy::Keep,
                session_ids: vec![session.session_id.clone()],
                retained_path: None,
                authorization: None,
            });
        }
        groups
    }

    // --- Run --------------------------------------------------------------------------------

    /// Drives every unfinished group of an accepted operation to a recorded end. Idempotent:
    /// a group that already reached a terminal or decision state is left alone.
    pub(crate) fn run(
        &self,
        operation_id: &str,
    ) -> Result<SessionDeletionOperation, SessionsApplicationError> {
        let Some(operation) = self.ports.journal.load(operation_id)? else {
            return Err(SessionsApplicationError::Validation(
                error_code::OPERATION_NOT_FOUND.to_string(),
            ));
        };
        if operation.outcome.is_terminal() {
            return Ok(operation);
        }
        let owner = self.ports.owner.current();
        self.ports.journal.update_operation(
            operation_id,
            operation.revision,
            &OperationPatch {
                outcome: None,
                phase: Some(DeletionPhase::Quiescing),
                error_code: None,
                completed: false,
                owner: Some(owner.clone()),
                last_retry_request_id: None,
            },
        )?;
        for group in &operation.groups {
            if !matches!(
                group.status,
                DeletionGroupStatus::Pending | DeletionGroupStatus::Running
            ) {
                continue;
            }
            // One group's internal failure must not leave the whole operation without an end:
            // the dialog follows the journal, and a journal that never settles is a dialog that
            // never closes. The group is parked according to how far it got; the rest continue.
            if let Err(error) = self.run_group(operation_id, group, &owner) {
                self.log(
                    SessionApplicationLogLevel::Error,
                    format!("Deletion group {} aborted: {error}", group.group_id),
                    Some(operation_id),
                );
                self.park_aborted_group(operation_id, &group.group_id);
            }
        }
        self.finish(operation_id)
    }

    /// Records an internal failure for a group whose step raised an error. A removal that had
    /// already begun keeps its claims and waits for a person (or, if the directory is confirmed
    /// gone, for the database-only retry); one that had not frees its sessions.
    fn park_aborted_group(&self, operation_id: &str, group_id: &str) {
        let Ok(Some(operation)) = self.ports.journal.load(operation_id) else {
            return;
        };
        let Some(group) = operation
            .groups
            .iter()
            .find(|group| group.group_id == group_id)
        else {
            return;
        };
        if !matches!(
            group.status,
            DeletionGroupStatus::Pending | DeletionGroupStatus::Running
        ) {
            return;
        }
        let patch = match group.worktree_effect {
            WorktreeEffect::Removed => GroupPatch {
                status: Some(DeletionGroupStatus::FinalizePending),
                db_effect: Some(SessionDbEffect::Pending),
                error_code: Some(Some(error_code::RUN_FAILED.to_string())),
                ..GroupPatch::default()
            },
            WorktreeEffect::RemoveStarted | WorktreeEffect::RemovalUnknown => GroupPatch {
                status: Some(DeletionGroupStatus::NeedsAttention),
                error_code: Some(Some(error_code::RUN_FAILED.to_string())),
                ..GroupPatch::default()
            },
            WorktreeEffect::NotRequested | WorktreeEffect::Retained => GroupPatch {
                status: Some(DeletionGroupStatus::Failed),
                worktree_effect: Some(retained_effect(
                    group.policy == WorktreeDeletionPolicy::RemoveSafe,
                )),
                db_effect: Some(SessionDbEffect::Retained),
                error_code: Some(Some(error_code::RUN_FAILED.to_string())),
                ..GroupPatch::default()
            },
        };
        let frees_sessions = patch.status == Some(DeletionGroupStatus::Failed);
        let _ = self
            .ports
            .journal
            .update_group(operation_id, group_id, group.revision, &patch);
        if frees_sessions {
            let _ = self
                .ports
                .journal
                .release_group_claims(operation_id, group_id);
        }
    }

    fn run_group(
        &self,
        operation_id: &str,
        group: &DeletionGroupResult,
        owner: &DeletionOwner,
    ) -> Result<(), SessionsApplicationError> {
        let revision = self.ports.journal.update_group(
            operation_id,
            &group.group_id,
            group.revision,
            &GroupPatch {
                status: Some(DeletionGroupStatus::Running),
                phase: Some(DeletionPhase::Quiescing),
                attempt: Some(group.attempt + 1),
                error_code: Some(None),
                ..GroupPatch::default()
            },
        )?;
        // A retried finalize: the directory is confirmed gone, so the only work left is the
        // database step. No gate, no revalidation, and above all no second removal.
        let already_removed = group.worktree_effect == WorktreeEffect::Removed;
        let remove = group.policy == WorktreeDeletionPolicy::RemoveSafe && !already_removed;
        let worktree_id = group.worktree_id.as_deref();

        // 1. Exclusive gate for a removal, before anything is stopped.
        let gate = match (remove, worktree_id) {
            (true, Some(worktree_id)) => {
                match self
                    .ports
                    .workspace
                    .claim_gate(worktree_id, owner, operation_id)?
                {
                    GateOutcome::Claimed(token) => Some(token),
                    GateOutcome::Held { .. } => {
                        self.settle(
                            operation_id,
                            &group.group_id,
                            revision,
                            DeletionGroupStatus::AwaitingDecision,
                            WorktreeEffect::Retained,
                            SessionDbEffect::Retained,
                            error_code::GATE_HELD,
                        )?;
                        return Ok(());
                    }
                }
            }
            _ => None,
        };
        let outcome = self.run_group_steps(
            operation_id,
            group,
            revision,
            remove,
            already_removed,
            worktree_id,
            gate.as_ref(),
            owner,
        );
        if outcome.is_err() {
            // The gate must not outlive the attempt that took it; the journal state is settled
            // by the caller, which can still read what this attempt recorded.
            self.release(gate.as_ref());
        }
        outcome
    }

    /// Everything after the gate: quiesce, revalidate, remove, delete rows. Split out so the gate
    /// can be released on any error path without threading the token through each `?`.
    #[allow(clippy::too_many_arguments)]
    fn run_group_steps(
        &self,
        operation_id: &str,
        group: &DeletionGroupResult,
        mut revision: u64,
        remove: bool,
        already_removed: bool,
        worktree_id: Option<&str>,
        gate: Option<&GateToken>,
        owner: &DeletionOwner,
    ) -> Result<(), SessionsApplicationError> {
        // 2. Quiesce every session in the group. Sessions already gone are quiet.
        for session_id in &group.session_ids {
            if self.load(session_id).is_err() {
                continue;
            }
            let report = match self
                .ports
                .runtime
                .quiesce(session_id, Duration::from_secs(QUIESCE_DEADLINE_SECONDS))
            {
                Ok(report) => report,
                Err(error) => {
                    self.log(
                        SessionApplicationLogLevel::Warn,
                        format!("Quiesce failed for session {session_id}: {error}"),
                        Some(operation_id),
                    );
                    self.release(gate);
                    self.settle(
                        operation_id,
                        &group.group_id,
                        revision,
                        DeletionGroupStatus::Failed,
                        retained_effect(remove),
                        SessionDbEffect::Retained,
                        error_code::QUIESCE_FAILED,
                    )?;
                    return Ok(());
                }
            };
            if !report.quiet {
                self.log(
                    SessionApplicationLogLevel::Warn,
                    format!(
                        "Session {session_id} did not quiesce: {}",
                        report.blockers.join(",")
                    ),
                    Some(operation_id),
                );
                self.release(gate);
                self.settle(
                    operation_id,
                    &group.group_id,
                    revision,
                    DeletionGroupStatus::Failed,
                    retained_effect(remove),
                    SessionDbEffect::Retained,
                    error_code::QUIESCE_TIMEOUT,
                )?;
                return Ok(());
            }
        }

        // 3. Removal, only after a revalidation taken now.
        let mut worktree_effect = if already_removed {
            WorktreeEffect::Removed
        } else {
            WorktreeEffect::NotRequested
        };
        if let (true, Some(worktree_id), Some(token)) = (remove, worktree_id, gate) {
            revision = self.ports.journal.update_group(
                operation_id,
                &group.group_id,
                revision,
                &GroupPatch {
                    phase: Some(DeletionPhase::Revalidating),
                    ..GroupPatch::default()
                },
            )?;
            match self.remove_worktree(operation_id, group, revision, worktree_id, token, owner)? {
                Ok((new_revision, effect)) => {
                    revision = new_revision;
                    worktree_effect = effect;
                }
                Err(()) => return Ok(()),
            }
        } else if remove {
            worktree_effect = WorktreeEffect::Retained;
        }

        // 4. The session rows, in one transaction.
        revision = self.ports.journal.update_group(
            operation_id,
            &group.group_id,
            revision,
            &GroupPatch {
                phase: Some(DeletionPhase::DeletingSessions),
                ..GroupPatch::default()
            },
        )?;
        self.finalize_group(operation_id, group, revision, worktree_effect, gate)
    }

    /// The removal itself. `Ok(Err(()))` means the group was settled without a removal and the
    /// caller must stop; `Ok(Ok(_))` means the worktree is confirmed gone.
    #[allow(clippy::too_many_arguments)]
    fn remove_worktree(
        &self,
        operation_id: &str,
        group: &DeletionGroupResult,
        revision: u64,
        worktree_id: &str,
        token: &GateToken,
        owner: &DeletionOwner,
    ) -> Result<Result<(u64, WorktreeEffect), ()>, SessionsApplicationError> {
        let snapshot = self
            .ports
            .journal
            .group_snapshot(operation_id, &group.group_id)?;
        let authorization = snapshot.and_then(|snapshot| snapshot.authorization);
        let authorized_identity: Option<WorktreeIdentityView> = authorization
            .as_ref()
            .and_then(|value| value.get("identity").cloned())
            .and_then(|value| serde_json::from_value(value).ok());
        let authorized_fingerprint: Option<String> = authorization
            .as_ref()
            .and_then(|value| value.get("ignoredFingerprint").cloned())
            .and_then(|value| serde_json::from_value(value).ok());
        let (scan, external) = self.external_references_for_group(worktree_id, group)?;
        let assessment = self.ports.workspace.assess(
            worktree_id,
            ReferenceInput {
                external_count: external,
                complete: scan.complete,
            },
            Some((owner, operation_id)),
        )?;
        let refusal = if !assessment.allows_removal() {
            Some(
                assessment
                    .blockers
                    .first()
                    .cloned()
                    .unwrap_or_else(|| error_code::REMOVAL_REFUSED.to_string()),
            )
        } else if assessment.identity.is_none()
            || authorized_identity.as_ref() != assessment.identity.as_ref()
        {
            Some(error_code::IDENTITY_CHANGED.to_string())
        } else if assessment.requires_ignored_acknowledgement
            && assessment
                .ignored
                .as_ref()
                .map(|ignored| ignored.fingerprint.as_str())
                != authorized_fingerprint.as_deref()
        {
            Some(error_code::IGNORED_ACKNOWLEDGEMENT_STALE.to_string())
        } else {
            None
        };
        if let Some(code) = refusal {
            self.release(Some(token));
            self.settle(
                operation_id,
                &group.group_id,
                revision,
                DeletionGroupStatus::AwaitingDecision,
                WorktreeEffect::Retained,
                SessionDbEffect::Retained,
                &code,
            )?;
            return Ok(Err(()));
        }
        let identity = assessment.identity.clone().ok_or_else(|| {
            SessionsApplicationError::Validation(error_code::IDENTITY_CHANGED.to_string())
        })?;

        // Journal first, then the record, then the command. A crash after any of these leaves a
        // state recovery can read.
        let revision = self.ports.journal.update_group(
            operation_id,
            &group.group_id,
            revision,
            &GroupPatch {
                phase: Some(DeletionPhase::RemovingWorktree),
                worktree_effect: Some(WorktreeEffect::RemoveStarted),
                execution_snapshot: Some(serde_json::json!({
                    "identity": identity,
                    "recordRevision": assessment.record_revision,
                    "startedAt": self.ports.clock.now(),
                })),
                ..GroupPatch::default()
            },
        )?;
        self.ports
            .workspace
            .begin_removal(worktree_id, assessment.record_revision)?;
        let report = self
            .ports
            .workspace
            .remove_safely(worktree_id, &identity, token)?;
        self.log(
            SessionApplicationLogLevel::Info,
            format!(
                "Worktree removal outcome for group {}: {:?}",
                group.group_id, report.outcome
            ),
            Some(operation_id),
        );
        let observation = &report.observation;
        if observation.confirmed_removed {
            let receipt_kind = match report.outcome {
                RemovalOutcomeView::Succeeded => "git_success",
                RemovalOutcomeView::TimedOut => "removed_observed_after_timeout",
                _ => "removed_observed",
            };
            let revision = self.ports.journal.update_group(
                operation_id,
                &group.group_id,
                revision,
                &GroupPatch {
                    worktree_effect: Some(WorktreeEffect::Removed),
                    receipt: Some(serde_json::json!({
                        "kind": receipt_kind,
                        "observedAt": self.ports.clock.now(),
                    })),
                    ..GroupPatch::default()
                },
            )?;
            return Ok(Ok((revision, WorktreeEffect::Removed)));
        }
        if observation.confirmed_intact {
            let code = match &report.outcome {
                RemovalOutcomeView::Refused { code } => code.clone(),
                RemovalOutcomeView::TimedOut => error_code::REMOVAL_TIMED_OUT.to_string(),
                RemovalOutcomeView::Unavailable { code } => code.clone(),
                RemovalOutcomeView::Succeeded => error_code::REMOVAL_UNKNOWN.to_string(),
            };
            let _ = self.ports.workspace.removal_refused(worktree_id);
            self.release(Some(token));
            self.settle(
                operation_id,
                &group.group_id,
                revision,
                DeletionGroupStatus::AwaitingDecision,
                WorktreeEffect::Retained,
                SessionDbEffect::Retained,
                &code,
            )?;
            return Ok(Err(()));
        }
        // Neither gone nor intact: nobody may touch it again without a person looking.
        let _ = self
            .ports
            .workspace
            .mark_attention(worktree_id, error_code::REMOVAL_UNKNOWN);
        self.release(Some(token));
        self.ports.journal.update_group(
            operation_id,
            &group.group_id,
            revision,
            &GroupPatch {
                status: Some(DeletionGroupStatus::NeedsAttention),
                worktree_effect: Some(WorktreeEffect::RemovalUnknown),
                error_code: Some(Some(error_code::REMOVAL_UNKNOWN.to_string())),
                ..GroupPatch::default()
            },
        )?;
        Ok(Err(()))
    }

    fn external_references_for_group(
        &self,
        worktree_id: &str,
        group: &DeletionGroupResult,
    ) -> Result<(ReferenceScan, usize), SessionsApplicationError> {
        let display_root = group.retained_path.clone().unwrap_or_default();
        let resolution = ResolvedWorktree {
            key: group.worktree_key.clone().unwrap_or_default(),
            worktree_id: Some(worktree_id.to_string()),
            canonical_root: None,
            display_root,
            branch: None,
            origin: String::new(),
            provenance: String::new(),
            resource_status: None,
        };
        let (scan, references) = self.external_references(&resolution, &group.session_ids)?;
        Ok((scan, references.len()))
    }

    /// Deletes the group's session rows in one transaction and records the outcome. A removed
    /// worktree whose rows could not be deleted stays `FinalizePending` with its claims held.
    fn finalize_group(
        &self,
        operation_id: &str,
        group: &DeletionGroupResult,
        revision: u64,
        worktree_effect: WorktreeEffect,
        gate: Option<&GateToken>,
    ) -> Result<(), SessionsApplicationError> {
        let completion = self.ports.journal.complete_group_deleting_sessions(
            operation_id,
            &group.group_id,
            revision,
            &group.session_ids,
        );
        match completion {
            Ok(completion) => {
                if let Some(worktree_id) = group.worktree_id.as_deref() {
                    let result = if worktree_effect == WorktreeEffect::Removed {
                        self.ports
                            .workspace
                            .finalize_removed(worktree_id, &group.session_ids)
                    } else {
                        self.ports
                            .workspace
                            .finalize_retained(worktree_id, &group.session_ids)
                    };
                    if let Err(error) = result {
                        self.log(
                            SessionApplicationLogLevel::Warn,
                            format!("Worktree record finalization deferred: {error}"),
                            Some(operation_id),
                        );
                    }
                }
                self.release(gate);
                let final_effect = if worktree_effect == WorktreeEffect::NotRequested
                    && group.worktree_id.is_some()
                {
                    WorktreeEffect::Retained
                } else {
                    worktree_effect
                };
                if final_effect != WorktreeEffect::Removed {
                    let _ = self.ports.journal.update_group(
                        operation_id,
                        &group.group_id,
                        completion.revision,
                        &GroupPatch {
                            worktree_effect: Some(final_effect),
                            ..GroupPatch::default()
                        },
                    );
                }
                self.ports.events.sessions_changed();
                if completion.active_session_cleared {
                    self.ports.events.active_session_cleared();
                }
                Ok(())
            }
            Err(error) => {
                self.log(
                    SessionApplicationLogLevel::Error,
                    format!("Session deletion transaction failed: {error}"),
                    Some(operation_id),
                );
                self.release(gate);
                if worktree_effect == WorktreeEffect::Removed {
                    self.ports.journal.update_group(
                        operation_id,
                        &group.group_id,
                        revision,
                        &GroupPatch {
                            status: Some(DeletionGroupStatus::FinalizePending),
                            db_effect: Some(SessionDbEffect::Pending),
                            error_code: Some(Some(error_code::FINALIZE_FAILED.to_string())),
                            ..GroupPatch::default()
                        },
                    )?;
                } else {
                    self.settle(
                        operation_id,
                        &group.group_id,
                        revision,
                        DeletionGroupStatus::Failed,
                        worktree_effect,
                        SessionDbEffect::Retained,
                        error_code::FINALIZE_FAILED,
                    )?;
                }
                Ok(())
            }
        }
    }

    /// Records a non-destructive end for a group and frees its sessions. Only for groups whose
    /// directory is known intact or untouched.
    #[allow(clippy::too_many_arguments)]
    fn settle(
        &self,
        operation_id: &str,
        group_id: &str,
        revision: u64,
        status: DeletionGroupStatus,
        worktree_effect: WorktreeEffect,
        db_effect: SessionDbEffect,
        code: &str,
    ) -> Result<(), SessionsApplicationError> {
        self.ports.journal.update_group(
            operation_id,
            group_id,
            revision,
            &GroupPatch {
                status: Some(status),
                worktree_effect: Some(worktree_effect),
                db_effect: Some(db_effect),
                error_code: Some(Some(code.to_string())),
                ..GroupPatch::default()
            },
        )?;
        self.ports
            .journal
            .release_group_claims(operation_id, group_id)
    }

    fn release(&self, gate: Option<&GateToken>) {
        if let Some(token) = gate {
            let _ = self.ports.workspace.release_gate(token);
        }
    }

    fn finish(
        &self,
        operation_id: &str,
    ) -> Result<SessionDeletionOperation, SessionsApplicationError> {
        let Some(operation) = self.ports.journal.load(operation_id)? else {
            return Err(SessionsApplicationError::Validation(
                error_code::OPERATION_NOT_FOUND.to_string(),
            ));
        };
        let outcome = aggregate_outcome(&operation.groups);
        let error_code = operation
            .groups
            .iter()
            .find_map(|group| group.error_code.clone());
        self.ports.journal.update_operation(
            operation_id,
            operation.revision,
            &OperationPatch {
                outcome: Some(outcome),
                phase: Some(DeletionPhase::Completed),
                error_code: Some(error_code),
                completed: outcome.is_terminal(),
                owner: None,
                last_retry_request_id: None,
            },
        )?;
        self.log(
            SessionApplicationLogLevel::Info,
            format!(
                "Session deletion finished with outcome {}",
                outcome.as_str()
            ),
            Some(operation_id),
        );
        self.ports.journal.load(operation_id)?.ok_or_else(|| {
            SessionsApplicationError::Validation(error_code::OPERATION_NOT_FOUND.to_string())
        })
    }

    // --- Query -------------------------------------------------------------------------------

    pub(crate) fn get(
        &self,
        operation_id: &str,
    ) -> Result<SessionDeletionOperation, SessionsApplicationError> {
        self.ports.journal.load(operation_id)?.ok_or_else(|| {
            SessionsApplicationError::Validation(error_code::OPERATION_NOT_FOUND.to_string())
        })
    }

    pub(crate) fn list_pending(
        &self,
    ) -> Result<Vec<SessionDeletionOperation>, SessionsApplicationError> {
        self.ports.journal.list_pending()
    }

    // --- Retry -------------------------------------------------------------------------------

    /// A new, explicitly authorized attempt for the groups that did not finish. Groups that
    /// succeeded are never replayed; a group that wants its directory removed again needs a
    /// fresh preview; a group whose directory is gone only retries the database step.
    pub(crate) fn retry(
        &self,
        request: RetrySessionDeletionRequest,
    ) -> Result<SessionDeletionHandle, SessionsApplicationError> {
        let operation = self.get(&request.operation_id)?;
        if operation.revision != request.expected_revision {
            return Err(SessionsApplicationError::Validation(
                error_code::REVISION_CONFLICT.to_string(),
            ));
        }
        let retry_id = request.retry_request_id.trim().to_string();
        if retry_id.is_empty() {
            return Err(SessionsApplicationError::Validation(
                "Retry request id cannot be empty.".to_string(),
            ));
        }
        if let Some(ownership) = self.ports.journal.ownership(&operation.operation_id)? {
            if ownership.last_retry_request_id.as_deref() == Some(retry_id.as_str()) {
                return Ok(handle(&operation, true));
            }
        }
        if operation.groups.iter().any(|group| {
            matches!(
                group.status,
                DeletionGroupStatus::Running | DeletionGroupStatus::Pending
            )
        }) {
            return Err(SessionsApplicationError::Validation(
                error_code::RETRY_NOT_ALLOWED.to_string(),
            ));
        }
        let preview = request
            .preview_id
            .as_deref()
            .and_then(|preview_id| self.ports.previews.get(preview_id));
        let resolved = match &preview {
            Some(stored) if stored.expires_at_unix >= self.ports.clock.unix_now() => {
                // A retry preview must describe exactly the groups it is retrying: a worktree
                // row whose sessions differ from the journaled group is a different target.
                for worktree in &stored.preview.worktrees {
                    let Some(group) = operation.groups.iter().find(|group| {
                        group.worktree_key.as_deref() == Some(worktree.worktree_key.as_str())
                    }) else {
                        continue;
                    };
                    let mut expected = worktree.session_ids.clone();
                    expected.sort();
                    let mut actual = group.session_ids.clone();
                    actual.sort();
                    if expected != actual {
                        return Err(SessionsApplicationError::Validation(
                            error_code::PREVIEW_TARGET_MISMATCH.to_string(),
                        ));
                    }
                }
                resolve_choices(&stored.preview.worktrees, &request.worktree_choices)?
            }
            Some(_) => {
                return Err(SessionsApplicationError::Validation(
                    error_code::PREVIEW_EXPIRED.to_string(),
                ))
            }
            None => Vec::new(),
        };
        let mut reopened = 0usize;
        for group in &operation.groups {
            match group.status {
                DeletionGroupStatus::Succeeded => continue,
                DeletionGroupStatus::NeedsAttention => {
                    return Err(SessionsApplicationError::Validation(
                        error_code::RETRY_NOT_ALLOWED.to_string(),
                    ))
                }
                DeletionGroupStatus::FinalizePending => {
                    // Database only; the directory is already gone.
                    self.ports.journal.update_group(
                        &operation.operation_id,
                        &group.group_id,
                        group.revision,
                        &GroupPatch {
                            status: Some(DeletionGroupStatus::Pending),
                            error_code: Some(None),
                            ..GroupPatch::default()
                        },
                    )?;
                    reopened += 1;
                }
                DeletionGroupStatus::Failed | DeletionGroupStatus::AwaitingDecision => {
                    let (policy, authorization) = match (&group.worktree_key, &preview) {
                        (Some(key), Some(stored)) => {
                            let choice = resolved
                                .iter()
                                .find(|(candidate, _, _)| candidate == key)
                                .map(|(_, policy, ack)| (*policy, ack.clone()))
                                .unwrap_or((WorktreeDeletionPolicy::Keep, None));
                            let same_targets = stored
                                .preview
                                .worktrees
                                .iter()
                                .find(|worktree| &worktree.worktree_key == key)
                                .is_some_and(|worktree| {
                                    let mut expected = worktree.session_ids.clone();
                                    expected.sort();
                                    let mut actual = group.session_ids.clone();
                                    actual.sort();
                                    expected == actual
                                });
                            if choice.0 == WorktreeDeletionPolicy::RemoveSafe && !same_targets {
                                return Err(SessionsApplicationError::Validation(
                                    error_code::PREVIEW_TARGET_MISMATCH.to_string(),
                                ));
                            }
                            let authorization = stored
                                .assessments
                                .get(key)
                                .filter(|_| choice.0 == WorktreeDeletionPolicy::RemoveSafe)
                                .map(|assessment| {
                                    serde_json::json!({
                                        "identity": assessment.identity,
                                        "recordRevision": assessment.record_revision,
                                        "ignoredFingerprint": choice.1,
                                    })
                                });
                            (choice.0, authorization)
                        }
                        _ => {
                            if group.policy == WorktreeDeletionPolicy::RemoveSafe
                                && group.worktree_effect != WorktreeEffect::Removed
                            {
                                return Err(SessionsApplicationError::Validation(
                                    error_code::RETRY_REQUIRES_PREVIEW.to_string(),
                                ));
                            }
                            (WorktreeDeletionPolicy::Keep, None)
                        }
                    };
                    for session_id in &group.session_ids {
                        self.load(session_id)?;
                    }
                    if let Some(claim) = self.ports.journal.reclaim_group(
                        &operation.operation_id,
                        &group.group_id,
                        &group.session_ids,
                    )? {
                        self.log(
                            SessionApplicationLogLevel::Warn,
                            format!(
                                "Retry refused: session {} is claimed by {}",
                                claim.session_id, claim.operation_id
                            ),
                            Some(&operation.operation_id),
                        );
                        return Err(SessionsApplicationError::Validation(
                            error_code::SESSION_CLAIMED.to_string(),
                        ));
                    }
                    self.ports.journal.update_group(
                        &operation.operation_id,
                        &group.group_id,
                        group.revision,
                        &GroupPatch {
                            status: Some(DeletionGroupStatus::Pending),
                            phase: Some(DeletionPhase::Accepted),
                            worktree_effect: Some(WorktreeEffect::NotRequested),
                            db_effect: Some(SessionDbEffect::Pending),
                            error_code: Some(None),
                            policy: Some(policy),
                            authorization,
                            ..GroupPatch::default()
                        },
                    )?;
                    reopened += 1;
                }
                DeletionGroupStatus::Pending | DeletionGroupStatus::Running => {}
            }
        }
        if reopened == 0 {
            return Err(SessionsApplicationError::Validation(
                error_code::RETRY_NOT_ALLOWED.to_string(),
            ));
        }
        if let Some(preview_id) = request.preview_id.as_deref() {
            self.ports.previews.remove(preview_id);
        }
        self.ports.journal.update_operation(
            &operation.operation_id,
            operation.revision,
            &OperationPatch {
                outcome: Some(DeletionOutcome::Pending),
                phase: Some(DeletionPhase::Accepted),
                error_code: Some(None),
                completed: false,
                owner: Some(self.ports.owner.current()),
                last_retry_request_id: Some(retry_id),
            },
        )?;
        let operation = self.get(&operation.operation_id)?;
        Ok(handle(&operation, false))
    }

    // --- Recovery ----------------------------------------------------------------------------

    /// Startup reconciliation. Observes rather than replays: a removal that was started is
    /// confirmed from the filesystem and Git, never re-run, and anything ambiguous is parked.
    pub(crate) fn reconcile_pending(&self) -> Result<Vec<String>, SessionsApplicationError> {
        let mut touched = Vec::new();
        for operation in self.ports.journal.list_pending()? {
            if let Some(ownership) = self.ports.journal.ownership(&operation.operation_id)? {
                let current = self.ports.owner.current();
                if ownership.owner.instance_id != current.instance_id
                    && self
                        .ports
                        .owner
                        .is_alive(&ownership.owner.instance_id)
                        .unwrap_or(true)
                {
                    continue;
                }
            }
            for group in &operation.groups {
                if !matches!(
                    group.status,
                    DeletionGroupStatus::Pending
                        | DeletionGroupStatus::Running
                        | DeletionGroupStatus::FinalizePending
                ) {
                    continue;
                }
                self.reconcile_group(&operation, group)?;
            }
            self.finish(&operation.operation_id)?;
            touched.push(operation.operation_id.clone());
        }
        Ok(touched)
    }

    fn reconcile_group(
        &self,
        operation: &SessionDeletionOperation,
        group: &DeletionGroupResult,
    ) -> Result<(), SessionsApplicationError> {
        let operation_id = operation.operation_id.as_str();
        match (group.worktree_effect, group.worktree_id.as_deref()) {
            (WorktreeEffect::Removed, Some(worktree_id))
            | (WorktreeEffect::RemoveStarted, Some(worktree_id)) => {
                let observation = self.ports.workspace.observe(worktree_id)?;
                match observation {
                    Some(observation) if observation.confirmed_removed => {
                        let receipt_kind = if group.worktree_effect == WorktreeEffect::Removed {
                            "confirmed_on_restart"
                        } else {
                            "removed_observed_after_interruption"
                        };
                        let revision = self.ports.journal.update_group(
                            operation_id,
                            &group.group_id,
                            group.revision,
                            &GroupPatch {
                                worktree_effect: Some(WorktreeEffect::Removed),
                                receipt: Some(serde_json::json!({
                                    "kind": receipt_kind,
                                    "observedAt": self.ports.clock.now(),
                                })),
                                ..GroupPatch::default()
                            },
                        )?;
                        self.finalize_group(
                            operation_id,
                            group,
                            revision,
                            WorktreeEffect::Removed,
                            None,
                        )
                    }
                    Some(observation)
                        if observation.confirmed_intact
                            && group.worktree_effect == WorktreeEffect::RemoveStarted =>
                    {
                        let _ = self.ports.workspace.removal_refused(worktree_id);
                        self.settle(
                            operation_id,
                            &group.group_id,
                            group.revision,
                            DeletionGroupStatus::AwaitingDecision,
                            WorktreeEffect::Retained,
                            SessionDbEffect::Retained,
                            error_code::INTERRUPTED,
                        )
                    }
                    _ => {
                        let _ = self
                            .ports
                            .workspace
                            .mark_attention(worktree_id, error_code::REMOVAL_UNKNOWN);
                        self.ports.journal.update_group(
                            operation_id,
                            &group.group_id,
                            group.revision,
                            &GroupPatch {
                                status: Some(DeletionGroupStatus::NeedsAttention),
                                worktree_effect: Some(WorktreeEffect::RemovalUnknown),
                                error_code: Some(Some(error_code::REMOVAL_UNKNOWN.to_string())),
                                ..GroupPatch::default()
                            },
                        )?;
                        Ok(())
                    }
                }
            }
            _ => {
                // Nothing irreversible had started. The sessions are released; a person decides
                // whether to run it again.
                self.settle(
                    operation_id,
                    &group.group_id,
                    group.revision,
                    DeletionGroupStatus::AwaitingDecision,
                    retained_effect(group.policy == WorktreeDeletionPolicy::RemoveSafe),
                    SessionDbEffect::Retained,
                    error_code::INTERRUPTED,
                )
            }
        }
    }

    // --- Legacy keep-only path ---------------------------------------------------------------

    /// `deleteSession(sessionId)`: the same coordinator, keep policy, run to completion. Refuses
    /// a session another operation holds and reports the real result, not the acceptance.
    pub(crate) fn delete_keep_only(
        &self,
        session_id: &str,
    ) -> Result<(), SessionsApplicationError> {
        let preview = self.preview(PreviewSessionDeletionRequest {
            session_ids: vec![session_id.to_string()],
        })?;
        let handle = self.execute(ExecuteSessionDeletionRequest {
            request_id: format!("keep-only:{}:{}", session_id, preview.preview_id),
            preview_id: preview.preview_id,
            worktree_choices: Vec::new(),
        })?;
        let operation = self.run(&handle.operation_id)?;
        match operation.outcome {
            DeletionOutcome::Succeeded => Ok(()),
            _ => Err(SessionsApplicationError::Validation(
                operation
                    .error_code
                    .unwrap_or_else(|| error_code::FINALIZE_FAILED.to_string()),
            )),
        }
    }

    // --- Helpers -----------------------------------------------------------------------------

    fn load(&self, session_id: &str) -> Result<SessionRecord, SessionsApplicationError> {
        self.ports
            .sessions
            .find(&SessionId::parse(session_id)?)?
            .ok_or_else(|| SessionsApplicationError::SessionNotFound(session_id.to_string()))
    }

    fn log(&self, level: SessionApplicationLogLevel, message: String, operation_id: Option<&str>) {
        let _ = self.ports.logging.write(SessionApplicationLog {
            level,
            category: LOG_CATEGORY.to_string(),
            message,
            session_id: None,
            operation_id: operation_id.map(str::to_string),
            execution_run_id: None,
            recovery_report_id: None,
        });
    }
}

fn handle(operation: &SessionDeletionOperation, existing: bool) -> SessionDeletionHandle {
    SessionDeletionHandle {
        operation_id: operation.operation_id.clone(),
        runtime_effect: operation.runtime_effect,
        operation_task_id: operation.operation_task_id.clone(),
        existing,
    }
}

fn retained_effect(remove_requested: bool) -> WorktreeEffect {
    if remove_requested {
        WorktreeEffect::Retained
    } else {
        WorktreeEffect::NotRequested
    }
}

fn workspace_kind(
    record: &SessionRecord,
    resolution: Option<&ResolvedWorktree>,
) -> DeletionWorkspaceKind {
    if record.workspace.remote_workspace.is_some() {
        DeletionWorkspaceKind::Remote
    } else if resolution.is_some() {
        DeletionWorkspaceKind::Worktree
    } else if record.workspace.project_path.is_some() || record.workspace.folder.is_some() {
        DeletionWorkspaceKind::Project
    } else {
        DeletionWorkspaceKind::None
    }
}

fn format_unix(seconds: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(seconds, 0)
        .map(|value| value.to_rfc3339())
        .unwrap_or_else(|| seconds.to_string())
}
