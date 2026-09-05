//! Coordinator behaviour over deterministic port doubles.
//!
//! Every fake records what it was asked so a test can assert not only the outcome but the
//! order: journal before Git, quiesce before revalidation, no second `remove` after an unknown
//! effect. No fake touches a filesystem, a database or a process.

use super::models::*;
use super::ports::*;
use super::{SessionDeletionCoordinator, SessionDeletionPorts};
use crate::contexts::sessions::application::{
    SessionApplicationLog, SessionListScope, SessionLoggingPort, SessionRecord,
    SessionRemoteWorkspace, SessionRepository, SessionSearchQuery, SessionSearchResult,
    SessionWorkspace, SessionsApplicationError,
};
use crate::contexts::sessions::domain::{
    SessionAggregate, SessionId, SessionLifecycle, SessionOwner, SessionPersonalizationMode,
    SessionTitle,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

// --- Session store -----------------------------------------------------------------------------

#[derive(Default)]
struct FakeSessions {
    records: Mutex<Vec<SessionRecord>>,
    active: Mutex<Option<String>>,
}

impl FakeSessions {
    fn insert(&self, record: SessionRecord) {
        self.records.lock().unwrap().push(record);
    }

    fn ids(&self) -> Vec<String> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .map(|record| record.id().to_string())
            .collect()
    }
}

impl SessionRepository for FakeSessions {
    fn find(
        &self,
        session_id: &SessionId,
    ) -> Result<Option<SessionRecord>, SessionsApplicationError> {
        Ok(self
            .records
            .lock()
            .unwrap()
            .iter()
            .find(|record| record.id() == session_id.as_str())
            .cloned())
    }

    fn list(
        &self,
        _scope: SessionListScope,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        Ok(self.records.lock().unwrap().clone())
    }

    fn search(
        &self,
        _query: &SessionSearchQuery,
    ) -> Result<Vec<SessionSearchResult>, SessionsApplicationError> {
        Ok(Vec::new())
    }

    fn active_session(&self) -> Result<Option<SessionRecord>, SessionsApplicationError> {
        let active = self.active.lock().unwrap().clone();
        Ok(active.and_then(|id| {
            self.records
                .lock()
                .unwrap()
                .iter()
                .find(|record| record.id() == id)
                .cloned()
        }))
    }

    fn save(&self, session: &SessionRecord) -> Result<SessionRecord, SessionsApplicationError> {
        Ok(session.clone())
    }

    fn inactive_sessions(
        &self,
        _cutoff: &str,
    ) -> Result<Vec<SessionRecord>, SessionsApplicationError> {
        Ok(Vec::new())
    }
}

fn record(id: &str, workspace: SessionWorkspace) -> SessionRecord {
    SessionRecord {
        personalization_mode: SessionPersonalizationMode::Standard,
        aggregate: SessionAggregate::rehydrate(
            SessionId::parse(id).expect("id"),
            SessionTitle::for_creation(Some(&format!("Title {id}"))),
            SessionLifecycle::Idle,
            SessionOwner::desktop(),
            None,
            false,
            false,
        ),
        agent_id: "codex-cli".to_string(),
        seats: Vec::new(),
        interaction_mode: "cli".to_string(),
        workspace,
        runtime_session_id: None,
        execution_origin_kind: "user".to_string(),
        execution_origin_id: None,
        created_at: "t0".to_string(),
        updated_at: "t0".to_string(),
    }
}

fn project_session(id: &str) -> SessionRecord {
    record(
        id,
        SessionWorkspace {
            folder: Some("/repo".to_string()),
            project_path: Some("/repo".to_string()),
            ..SessionWorkspace::default()
        },
    )
}

fn worktree_session(id: &str, root: &str) -> SessionRecord {
    record(
        id,
        SessionWorkspace {
            folder: Some(root.to_string()),
            project_path: Some("/repo".to_string()),
            worktree_path: Some(root.to_string()),
            worktree_name: Some("feature".to_string()),
            worktree_branch: Some("vanehub/feature".to_string()),
            ..SessionWorkspace::default()
        },
    )
}

fn remote_session(id: &str) -> SessionRecord {
    record(
        id,
        SessionWorkspace {
            folder: Some("ssh://host/srv".to_string()),
            remote_workspace: Some(SessionRemoteWorkspace {
                host: "host".to_string(),
                port: Some(22),
                user: None,
                path: "/srv".to_string(),
                display_name: "host:/srv".to_string(),
                uri: "ssh://host/srv".to_string(),
            }),
            ..SessionWorkspace::default()
        },
    )
}

// --- Journal -----------------------------------------------------------------------------------

#[derive(Clone)]
struct StoredGroup {
    result: DeletionGroupResult,
    authorization: Option<serde_json::Value>,
    execution_snapshot: Option<serde_json::Value>,
    receipt: Option<serde_json::Value>,
}

#[derive(Clone)]
struct StoredOperation {
    operation: SessionDeletionOperation,
    request_hash: String,
    owner: DeletionOwner,
    last_retry_request_id: Option<String>,
    groups: Vec<StoredGroup>,
}

struct FakeJournal {
    operations: Mutex<BTreeMap<String, StoredOperation>>,
    claims: Mutex<BTreeMap<String, SessionDeletionClaim>>,
    sessions: Arc<FakeSessions>,
    fail_complete: AtomicBool,
    fail_create: AtomicBool,
    calls: Mutex<Vec<String>>,
}

impl FakeJournal {
    fn new(sessions: Arc<FakeSessions>) -> Self {
        Self {
            operations: Mutex::new(BTreeMap::new()),
            claims: Mutex::new(BTreeMap::new()),
            sessions,
            fail_complete: AtomicBool::new(false),
            fail_create: AtomicBool::new(false),
            calls: Mutex::new(Vec::new()),
        }
    }

    fn call(&self, label: impl Into<String>) {
        self.calls.lock().unwrap().push(label.into());
    }

    fn view(&self, stored: &StoredOperation) -> SessionDeletionOperation {
        let mut operation = stored.operation.clone();
        operation.groups = stored
            .groups
            .iter()
            .map(|group| group.result.clone())
            .collect();
        operation
    }
}

impl DeletionJournalPort for FakeJournal {
    fn create(
        &self,
        operation: &NewDeletionOperation,
    ) -> Result<JournalCreateOutcome, SessionsApplicationError> {
        if self.fail_create.load(Ordering::SeqCst) {
            return Err(SessionsApplicationError::Repository(
                "journal write failed".to_string(),
            ));
        }
        let mut operations = self.operations.lock().unwrap();
        if let Some(existing) = operations
            .values()
            .find(|stored| stored.operation.request_id == operation.request_id)
        {
            return Ok(if existing.request_hash == operation.request_hash {
                JournalCreateOutcome::Existing(self.view(existing))
            } else {
                JournalCreateOutcome::RequestConflict
            });
        }
        let mut claims = self.claims.lock().unwrap();
        for group in &operation.groups {
            for session_id in &group.session_ids {
                if let Some(claim) = claims.get(session_id) {
                    return Ok(JournalCreateOutcome::SessionClaimed {
                        session_id: session_id.clone(),
                        operation_id: claim.operation_id.clone(),
                    });
                }
            }
        }
        let groups: Vec<StoredGroup> = operation
            .groups
            .iter()
            .map(|group| StoredGroup {
                result: DeletionGroupResult {
                    group_id: group.group_id.clone(),
                    worktree_key: group.worktree_key.clone(),
                    worktree_id: group.worktree_id.clone(),
                    policy: group.policy,
                    session_ids: group.session_ids.clone(),
                    status: DeletionGroupStatus::Pending,
                    phase: DeletionPhase::Accepted,
                    worktree_effect: WorktreeEffect::NotRequested,
                    db_effect: SessionDbEffect::Pending,
                    error_code: None,
                    retained_path: group.retained_path.clone(),
                    attempt: 0,
                    revision: 1,
                },
                authorization: group.authorization.clone(),
                execution_snapshot: None,
                receipt: None,
            })
            .collect();
        for group in &operation.groups {
            for session_id in &group.session_ids {
                claims.insert(
                    session_id.clone(),
                    SessionDeletionClaim {
                        session_id: session_id.clone(),
                        operation_id: operation.operation_id.clone(),
                        group_id: group.group_id.clone(),
                    },
                );
            }
        }
        let stored = StoredOperation {
            operation: SessionDeletionOperation {
                operation_id: operation.operation_id.clone(),
                request_id: operation.request_id.clone(),
                outcome: DeletionOutcome::Pending,
                phase: DeletionPhase::Accepted,
                revision: 1,
                runtime_effect: operation.runtime_effect,
                created_at: operation.created_at.clone(),
                updated_at: operation.created_at.clone(),
                completed_at: None,
                groups: Vec::new(),
                error_code: None,
                operation_task_id: None,
            },
            request_hash: operation.request_hash.clone(),
            owner: operation.owner.clone(),
            last_retry_request_id: None,
            groups,
        };
        let view = self.view(&stored);
        operations.insert(operation.operation_id.clone(), stored);
        self.call("create");
        Ok(JournalCreateOutcome::Created(view))
    }

    fn load(
        &self,
        operation_id: &str,
    ) -> Result<Option<SessionDeletionOperation>, SessionsApplicationError> {
        Ok(self
            .operations
            .lock()
            .unwrap()
            .get(operation_id)
            .map(|stored| self.view(stored)))
    }

    fn list_pending(&self) -> Result<Vec<SessionDeletionOperation>, SessionsApplicationError> {
        Ok(self
            .operations
            .lock()
            .unwrap()
            .values()
            .filter(|stored| !stored.operation.outcome.is_terminal())
            .map(|stored| self.view(stored))
            .collect())
    }

    fn ownership(
        &self,
        operation_id: &str,
    ) -> Result<Option<OperationOwnership>, SessionsApplicationError> {
        Ok(self
            .operations
            .lock()
            .unwrap()
            .get(operation_id)
            .map(|stored| OperationOwnership {
                owner: stored.owner.clone(),
                last_retry_request_id: stored.last_retry_request_id.clone(),
            }))
    }

    fn update_operation(
        &self,
        operation_id: &str,
        expected_revision: u64,
        patch: &OperationPatch,
    ) -> Result<u64, SessionsApplicationError> {
        let mut operations = self.operations.lock().unwrap();
        let stored = operations
            .get_mut(operation_id)
            .ok_or_else(|| SessionsApplicationError::Validation("missing".to_string()))?;
        if stored.operation.revision != expected_revision {
            return Err(SessionsApplicationError::Validation(
                error_code::REVISION_CONFLICT.to_string(),
            ));
        }
        if let Some(outcome) = patch.outcome {
            stored.operation.outcome = outcome;
        }
        if let Some(phase) = patch.phase {
            stored.operation.phase = phase;
        }
        if let Some(code) = &patch.error_code {
            stored.operation.error_code = code.clone();
        }
        if let Some(owner) = &patch.owner {
            stored.owner = owner.clone();
        }
        if let Some(retry) = &patch.last_retry_request_id {
            stored.last_retry_request_id = Some(retry.clone());
        }
        if patch.completed {
            stored.operation.completed_at = Some("t-done".to_string());
        }
        stored.operation.revision += 1;
        Ok(stored.operation.revision)
    }

    fn update_group(
        &self,
        operation_id: &str,
        group_id: &str,
        expected_revision: u64,
        patch: &GroupPatch,
    ) -> Result<u64, SessionsApplicationError> {
        let mut operations = self.operations.lock().unwrap();
        let stored = operations
            .get_mut(operation_id)
            .ok_or_else(|| SessionsApplicationError::Validation("missing".to_string()))?;
        let group = stored
            .groups
            .iter_mut()
            .find(|group| group.result.group_id == group_id)
            .ok_or_else(|| SessionsApplicationError::Validation("missing group".to_string()))?;
        if group.result.revision != expected_revision {
            return Err(SessionsApplicationError::Validation(
                error_code::REVISION_CONFLICT.to_string(),
            ));
        }
        if let Some(status) = patch.status {
            group.result.status = status;
        }
        if let Some(phase) = patch.phase {
            group.result.phase = phase;
        }
        if let Some(effect) = patch.worktree_effect {
            group.result.worktree_effect = effect;
        }
        if let Some(effect) = patch.db_effect {
            group.result.db_effect = effect;
        }
        if let Some(code) = &patch.error_code {
            group.result.error_code = code.clone();
        }
        if let Some(snapshot) = &patch.execution_snapshot {
            group.execution_snapshot = Some(snapshot.clone());
        }
        if let Some(receipt) = &patch.receipt {
            group.receipt = Some(receipt.clone());
        }
        if let Some(attempt) = patch.attempt {
            group.result.attempt = attempt;
        }
        if let Some(policy) = patch.policy {
            group.result.policy = policy;
        }
        if let Some(authorization) = &patch.authorization {
            group.authorization = Some(authorization.clone());
        }
        group.result.revision += 1;
        self.calls.lock().unwrap().push(format!(
            "group:{}:{}:{}",
            group.result.status.as_str(),
            group.result.worktree_effect.as_str(),
            group.result.phase.as_str()
        ));
        Ok(group.result.revision)
    }

    fn group_snapshot(
        &self,
        operation_id: &str,
        group_id: &str,
    ) -> Result<Option<GroupSnapshot>, SessionsApplicationError> {
        Ok(self
            .operations
            .lock()
            .unwrap()
            .get(operation_id)
            .and_then(|stored| {
                stored
                    .groups
                    .iter()
                    .find(|group| group.result.group_id == group_id)
                    .map(|group| GroupSnapshot {
                        execution_snapshot: group.execution_snapshot.clone(),
                        receipt: group.receipt.clone(),
                        authorization: group.authorization.clone(),
                    })
            }))
    }

    fn complete_group_deleting_sessions(
        &self,
        operation_id: &str,
        group_id: &str,
        expected_revision: u64,
        session_ids: &[String],
    ) -> Result<GroupCompletion, SessionsApplicationError> {
        self.call("complete");
        if self.fail_complete.load(Ordering::SeqCst) {
            return Err(SessionsApplicationError::Transaction(
                "disk full".to_string(),
            ));
        }
        let mut cleared = false;
        {
            let mut records = self.sessions.records.lock().unwrap();
            records.retain(|record| !session_ids.contains(&record.id().to_string()));
            let mut active = self.sessions.active.lock().unwrap();
            if active.as_ref().is_some_and(|id| session_ids.contains(id)) {
                *active = None;
                cleared = true;
            }
        }
        let revision = self.update_group(
            operation_id,
            group_id,
            expected_revision,
            &GroupPatch {
                status: Some(DeletionGroupStatus::Succeeded),
                phase: Some(DeletionPhase::Completed),
                db_effect: Some(SessionDbEffect::Deleted),
                error_code: Some(None),
                ..GroupPatch::default()
            },
        )?;
        self.release_group_claims(operation_id, group_id)?;
        Ok(GroupCompletion {
            revision,
            active_session_cleared: cleared,
        })
    }

    fn release_group_claims(
        &self,
        operation_id: &str,
        group_id: &str,
    ) -> Result<(), SessionsApplicationError> {
        self.claims
            .lock()
            .unwrap()
            .retain(|_, claim| !(claim.operation_id == operation_id && claim.group_id == group_id));
        Ok(())
    }

    fn reclaim_group(
        &self,
        operation_id: &str,
        group_id: &str,
        session_ids: &[String],
    ) -> Result<Option<SessionDeletionClaim>, SessionsApplicationError> {
        let mut claims = self.claims.lock().unwrap();
        for session_id in session_ids {
            if let Some(claim) = claims.get(session_id) {
                if claim.operation_id != operation_id {
                    return Ok(Some(claim.clone()));
                }
            }
        }
        for session_id in session_ids {
            claims.insert(
                session_id.clone(),
                SessionDeletionClaim {
                    session_id: session_id.clone(),
                    operation_id: operation_id.to_string(),
                    group_id: group_id.to_string(),
                },
            );
        }
        Ok(None)
    }

    fn active_claim(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionDeletionClaim>, SessionsApplicationError> {
        Ok(self.claims.lock().unwrap().get(session_id).cloned())
    }
}

// --- Workspace ---------------------------------------------------------------------------------

#[derive(Clone)]
struct FakeWorktree {
    assessment: WorktreeAssessment,
    gate_held: bool,
    report: RemovalReportView,
    observation: Option<ObservationView>,
    bound: Vec<String>,
}

struct FakeWorkspace {
    resolutions: Mutex<BTreeMap<String, ResolvedWorktree>>,
    worktrees: Mutex<BTreeMap<String, FakeWorktree>>,
    calls: Mutex<Vec<String>>,
    /// Worktree ids whose `claim_gate` raises an internal error.
    fail_claim: Mutex<Vec<String>>,
    /// Worktree ids whose `begin_removal` raises an internal error.
    fail_begin_removal: Mutex<Vec<String>>,
}

impl FakeWorkspace {
    fn new() -> Self {
        Self {
            resolutions: Mutex::new(BTreeMap::new()),
            worktrees: Mutex::new(BTreeMap::new()),
            calls: Mutex::new(Vec::new()),
            fail_claim: Mutex::new(Vec::new()),
            fail_begin_removal: Mutex::new(Vec::new()),
        }
    }

    fn call(&self, label: impl Into<String>) {
        self.calls.lock().unwrap().push(label.into());
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    fn count(&self, prefix: &str) -> usize {
        self.calls()
            .iter()
            .filter(|call| call.starts_with(prefix))
            .count()
    }

    fn worktree(&self, id: &str) -> FakeWorktree {
        self.worktrees
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .expect("worktree fixture")
    }

    fn set_assessment(&self, id: &str, update: impl FnOnce(&mut WorktreeAssessment)) {
        let mut worktrees = self.worktrees.lock().unwrap();
        update(&mut worktrees.get_mut(id).expect("worktree").assessment);
    }
}

impl DeletionWorkspacePort for FakeWorkspace {
    fn resolve(
        &self,
        session: &SessionRecord,
    ) -> Result<Option<ResolvedWorktree>, SessionsApplicationError> {
        Ok(self.resolutions.lock().unwrap().get(session.id()).cloned())
    }

    fn assess(
        &self,
        worktree_id: &str,
        references: ReferenceInput,
        _owner: Option<(&DeletionOwner, &str)>,
    ) -> Result<WorktreeAssessment, SessionsApplicationError> {
        self.call(format!("assess:{worktree_id}"));
        let mut assessment = self.worktree(worktree_id).assessment;
        if !references.complete {
            assessment.allowed_policies = vec![WorktreeDeletionPolicy::Keep];
            assessment
                .blockers
                .push("references_incomplete".to_string());
            assessment.checks = DeletionCheckCompleteness::Incomplete;
        } else if references.external_count > 0 {
            assessment.allowed_policies = vec![WorktreeDeletionPolicy::Keep];
            assessment.blockers.push("external_references".to_string());
        }
        Ok(assessment)
    }

    fn bound_sessions(&self, worktree_id: &str) -> Result<Vec<String>, SessionsApplicationError> {
        Ok(self.worktree(worktree_id).bound)
    }

    fn claim_gate(
        &self,
        worktree_id: &str,
        _owner: &DeletionOwner,
        operation_id: &str,
    ) -> Result<GateOutcome, SessionsApplicationError> {
        self.call(format!("claim:{worktree_id}"));
        if self
            .fail_claim
            .lock()
            .unwrap()
            .iter()
            .any(|id| id == worktree_id)
        {
            return Err(SessionsApplicationError::Workspace(
                "gate storage unavailable".to_string(),
            ));
        }
        if self.worktree(worktree_id).gate_held {
            return Ok(GateOutcome::Held {
                operation_id: "other".to_string(),
            });
        }
        Ok(GateOutcome::Claimed(GateToken {
            worktree_id: worktree_id.to_string(),
            canonical_root: format!("/canon/{worktree_id}"),
            instance_id: "me".to_string(),
            epoch: 1,
            operation_id: operation_id.to_string(),
            claimed_at: "t".to_string(),
        }))
    }

    fn release_gate(&self, token: &GateToken) -> Result<(), SessionsApplicationError> {
        self.call(format!("release:{}", token.worktree_id));
        Ok(())
    }

    fn begin_removal(
        &self,
        worktree_id: &str,
        _expected_revision: u64,
    ) -> Result<(), SessionsApplicationError> {
        self.call(format!("begin_removal:{worktree_id}"));
        if self
            .fail_begin_removal
            .lock()
            .unwrap()
            .iter()
            .any(|id| id == worktree_id)
        {
            return Err(SessionsApplicationError::Workspace(
                "record storage unavailable".to_string(),
            ));
        }
        Ok(())
    }

    fn remove_safely(
        &self,
        worktree_id: &str,
        _identity: &WorktreeIdentityView,
        _token: &GateToken,
    ) -> Result<RemovalReportView, SessionsApplicationError> {
        self.call(format!("remove:{worktree_id}"));
        Ok(self.worktree(worktree_id).report)
    }

    fn observe(
        &self,
        worktree_id: &str,
    ) -> Result<Option<ObservationView>, SessionsApplicationError> {
        self.call(format!("observe:{worktree_id}"));
        Ok(self.worktree(worktree_id).observation)
    }

    fn finalize_removed(
        &self,
        worktree_id: &str,
        _session_ids: &[String],
    ) -> Result<(), SessionsApplicationError> {
        self.call(format!("finalize_removed:{worktree_id}"));
        Ok(())
    }

    fn finalize_retained(
        &self,
        worktree_id: &str,
        _session_ids: &[String],
    ) -> Result<(), SessionsApplicationError> {
        self.call(format!("finalize_retained:{worktree_id}"));
        Ok(())
    }

    fn removal_refused(&self, worktree_id: &str) -> Result<(), SessionsApplicationError> {
        self.call(format!("removal_refused:{worktree_id}"));
        Ok(())
    }

    fn mark_attention(
        &self,
        worktree_id: &str,
        reason: &str,
    ) -> Result<(), SessionsApplicationError> {
        self.call(format!("attention:{worktree_id}:{reason}"));
        Ok(())
    }
}

// --- Other ports -------------------------------------------------------------------------------

struct FakeReferences {
    scan: Mutex<ReferenceScan>,
}

impl DeletionReferencePort for FakeReferences {
    fn references_to_root(
        &self,
        _canonical_root: Option<&str>,
        _display_root: &str,
    ) -> Result<ReferenceScan, SessionsApplicationError> {
        Ok(self.scan.lock().unwrap().clone())
    }
}

struct FakeRuntime {
    reports: Mutex<BTreeMap<String, QuiescenceReport>>,
    calls: Mutex<Vec<String>>,
}

impl SessionDeletionRuntimePort for FakeRuntime {
    fn quiesce(
        &self,
        session_id: &str,
        _deadline: Duration,
    ) -> Result<QuiescenceReport, SessionsApplicationError> {
        self.calls.lock().unwrap().push(session_id.to_string());
        Ok(self
            .reports
            .lock()
            .unwrap()
            .get(session_id)
            .cloned()
            .unwrap_or(QuiescenceReport {
                quiet: true,
                blockers: Vec::new(),
            }))
    }
}

#[derive(Default)]
struct FakePreviews {
    stored: Mutex<BTreeMap<String, StoredPreview>>,
}

impl DeletionPreviewStore for FakePreviews {
    fn put(&self, stored: StoredPreview) {
        self.stored
            .lock()
            .unwrap()
            .insert(stored.preview.preview_id.clone(), stored);
    }

    fn get(&self, preview_id: &str) -> Option<StoredPreview> {
        self.stored.lock().unwrap().get(preview_id).cloned()
    }

    fn remove(&self, preview_id: &str) {
        self.stored.lock().unwrap().remove(preview_id);
    }
}

struct FakeClock {
    unix: AtomicU64,
}

impl DeletionClockPort for FakeClock {
    fn now(&self) -> String {
        format!("t{}", self.unix.load(Ordering::SeqCst))
    }

    fn unix_now(&self) -> i64 {
        self.unix.load(Ordering::SeqCst) as i64
    }
}

#[derive(Default)]
struct FakeIds {
    next: AtomicU64,
}

impl DeletionIdPort for FakeIds {
    fn next_operation_id(&self) -> String {
        format!("op-{}", self.next.fetch_add(1, Ordering::SeqCst))
    }

    fn next_group_id(&self) -> String {
        format!("g-{}", self.next.fetch_add(1, Ordering::SeqCst))
    }

    fn next_preview_id(&self) -> String {
        format!("pv-{}", self.next.fetch_add(1, Ordering::SeqCst))
    }
}

struct FakeOwner {
    instance: String,
    alive: Mutex<BTreeMap<String, bool>>,
}

impl DeletionOwnerPort for FakeOwner {
    fn current(&self) -> DeletionOwner {
        DeletionOwner {
            instance_id: self.instance.clone(),
            epoch: 1,
        }
    }

    fn is_alive(&self, instance_id: &str) -> Option<bool> {
        self.alive.lock().unwrap().get(instance_id).copied()
    }
}

#[derive(Default)]
struct FakeLogging;

impl SessionLoggingPort for FakeLogging {
    fn write(&self, _log: SessionApplicationLog) -> Result<(), SessionsApplicationError> {
        Ok(())
    }
}

#[derive(Default)]
struct FakeEvents {
    cleared: AtomicU64,
    changed: AtomicU64,
}

impl DeletionEventPort for FakeEvents {
    fn active_session_cleared(&self) {
        self.cleared.fetch_add(1, Ordering::SeqCst);
    }

    fn sessions_changed(&self) {
        self.changed.fetch_add(1, Ordering::SeqCst);
    }
}

// --- Harness -----------------------------------------------------------------------------------

struct Harness {
    coordinator: SessionDeletionCoordinator,
    sessions: Arc<FakeSessions>,
    journal: Arc<FakeJournal>,
    workspace: Arc<FakeWorkspace>,
    references: Arc<FakeReferences>,
    runtime: Arc<FakeRuntime>,
    previews: Arc<FakePreviews>,
    clock: Arc<FakeClock>,
    owner: Arc<FakeOwner>,
    events: Arc<FakeEvents>,
}

fn identity(root: &str) -> WorktreeIdentityView {
    WorktreeIdentityView {
        canonical_root: root.to_string(),
        git_dir: "/repo/.git/worktrees/x".to_string(),
        common_dir: "/repo/.git".to_string(),
        branch: Some("vanehub/feature".to_string()),
        head: Some("abc".to_string()),
        fs_identity: Some("1:2".to_string()),
    }
}

fn clean_assessment(root: &str) -> WorktreeAssessment {
    WorktreeAssessment {
        identity: Some(identity(root)),
        record_revision: 2,
        allowed_policies: vec![
            WorktreeDeletionPolicy::Keep,
            WorktreeDeletionPolicy::RemoveSafe,
        ],
        blockers: Vec::new(),
        checks: DeletionCheckCompleteness::Complete,
        requires_ignored_acknowledgement: false,
        changes: Some(DeletionChangeSummary::default()),
        ignored: Some(DeletionIgnoredSummary {
            total_entries: 0,
            samples: Vec::new(),
            samples_truncated: false,
            completeness: DeletionCheckCompleteness::Complete,
            fingerprint: "empty".to_string(),
        }),
        resource_status: "attached".to_string(),
    }
}

fn removed_observation() -> ObservationView {
    ObservationView {
        root_present: Tri::Absent,
        registered: Tri::Absent,
        confirmed_removed: true,
        confirmed_intact: false,
    }
}

fn intact_observation() -> ObservationView {
    ObservationView {
        root_present: Tri::Present,
        registered: Tri::Present,
        confirmed_removed: false,
        confirmed_intact: true,
    }
}

fn unknown_observation() -> ObservationView {
    ObservationView {
        root_present: Tri::Unknown,
        registered: Tri::Unknown,
        confirmed_removed: false,
        confirmed_intact: false,
    }
}

impl Harness {
    fn new() -> Self {
        let sessions = Arc::new(FakeSessions::default());
        let journal = Arc::new(FakeJournal::new(sessions.clone()));
        let workspace = Arc::new(FakeWorkspace::new());
        let references = Arc::new(FakeReferences {
            scan: Mutex::new(ReferenceScan {
                sessions: Vec::new(),
                loop_runs: Vec::new(),
                complete: true,
            }),
        });
        let runtime = Arc::new(FakeRuntime {
            reports: Mutex::new(BTreeMap::new()),
            calls: Mutex::new(Vec::new()),
        });
        let previews = Arc::new(FakePreviews::default());
        let clock = Arc::new(FakeClock {
            unix: AtomicU64::new(1_000),
        });
        let owner = Arc::new(FakeOwner {
            instance: "me".to_string(),
            alive: Mutex::new(BTreeMap::new()),
        });
        let events = Arc::new(FakeEvents::default());
        let coordinator = SessionDeletionCoordinator::new(SessionDeletionPorts {
            sessions: sessions.clone(),
            journal: journal.clone(),
            runtime: runtime.clone(),
            workspace: workspace.clone(),
            references: references.clone(),
            previews: previews.clone(),
            clock: clock.clone(),
            ids: Arc::new(FakeIds::default()),
            owner: owner.clone(),
            logging: Arc::new(FakeLogging),
            events: events.clone(),
        });
        Self {
            coordinator,
            sessions,
            journal,
            workspace,
            references,
            runtime,
            previews,
            clock,
            owner,
            events,
        }
    }

    /// A verified worktree bound to `session_ids`, clean, removable, removal succeeds.
    fn add_worktree(&self, worktree_id: &str, root: &str, session_ids: &[&str]) {
        for session_id in session_ids {
            self.sessions.insert(worktree_session(session_id, root));
            self.workspace.resolutions.lock().unwrap().insert(
                (*session_id).to_string(),
                ResolvedWorktree {
                    key: worktree_id.to_string(),
                    worktree_id: Some(worktree_id.to_string()),
                    canonical_root: Some(root.to_string()),
                    display_root: root.to_string(),
                    branch: Some("vanehub/feature".to_string()),
                    origin: "ordinary_session".to_string(),
                    provenance: "verified".to_string(),
                    resource_status: Some("attached".to_string()),
                },
            );
        }
        self.workspace.worktrees.lock().unwrap().insert(
            worktree_id.to_string(),
            FakeWorktree {
                assessment: clean_assessment(root),
                gate_held: false,
                report: RemovalReportView {
                    outcome: RemovalOutcomeView::Succeeded,
                    observation: removed_observation(),
                },
                observation: Some(removed_observation()),
                bound: session_ids.iter().map(|id| (*id).to_string()).collect(),
            },
        );
    }

    fn add_unverified_worktree(&self, session_id: &str, root: &str) {
        self.sessions.insert(worktree_session(session_id, root));
        self.workspace.resolutions.lock().unwrap().insert(
            session_id.to_string(),
            ResolvedWorktree {
                key: format!("unverified:{root}"),
                worktree_id: None,
                canonical_root: Some(root.to_string()),
                display_root: root.to_string(),
                branch: Some("vanehub/feature".to_string()),
                origin: "ordinary_session".to_string(),
                provenance: "provenance_unverified".to_string(),
                resource_status: None,
            },
        );
    }

    fn preview(&self, ids: &[&str]) -> SessionDeletionPreview {
        self.coordinator
            .preview(PreviewSessionDeletionRequest {
                session_ids: ids.iter().map(|id| (*id).to_string()).collect(),
            })
            .expect("preview")
    }

    fn execute(
        &self,
        preview: &SessionDeletionPreview,
        request_id: &str,
        choices: Vec<WorktreeDeletionChoice>,
    ) -> SessionDeletionHandle {
        self.coordinator
            .execute(ExecuteSessionDeletionRequest {
                request_id: request_id.to_string(),
                preview_id: preview.preview_id.clone(),
                worktree_choices: choices,
            })
            .expect("execute")
    }

    fn remove_choice(key: &str) -> WorktreeDeletionChoice {
        WorktreeDeletionChoice {
            worktree_key: key.to_string(),
            policy: WorktreeDeletionPolicy::RemoveSafe,
            ignored_files_acknowledgement: None,
        }
    }
}

fn validation_code(error: SessionsApplicationError) -> String {
    match error {
        SessionsApplicationError::Validation(code) => code,
        other => panic!("expected validation error, got {other:?}"),
    }
}

// --- Preview -----------------------------------------------------------------------------------

#[test]
fn preview_classifies_project_remote_and_worktree_sessions_and_deduplicates_worktrees() {
    let harness = Harness::new();
    harness.sessions.insert(project_session("p1"));
    harness.sessions.insert(remote_session("r1"));
    harness.add_worktree("wt-1", "/repo-feature", &["w1", "w2"]);
    harness.add_unverified_worktree("u1", "/repo-old");
    *harness.sessions.active.lock().unwrap() = Some("w1".to_string());

    let preview = harness.preview(&["p1", "r1", "w1", "w2", "u1", "w1"]);

    assert_eq!(preview.runtime_effect, DeletionRuntimeEffect::Native);
    assert_eq!(preview.sessions.len(), 5);
    let kinds: Vec<_> = preview
        .sessions
        .iter()
        .map(|session| session.workspace_kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            DeletionWorkspaceKind::Project,
            DeletionWorkspaceKind::Remote,
            DeletionWorkspaceKind::Worktree,
            DeletionWorkspaceKind::Worktree,
            DeletionWorkspaceKind::Worktree,
        ]
    );
    assert!(preview.sessions[2].active);
    assert_eq!(preview.worktrees.len(), 2);
    let verified = preview
        .worktrees
        .iter()
        .find(|row| row.worktree_key == "wt-1")
        .expect("verified row");
    assert_eq!(verified.session_ids, vec!["w1", "w2"]);
    assert!(verified
        .allowed_policies
        .contains(&WorktreeDeletionPolicy::RemoveSafe));
    assert!(verified.blockers.is_empty());
    let unverified = preview
        .worktrees
        .iter()
        .find(|row| row.worktree_id.is_none())
        .expect("unverified row");
    assert_eq!(
        unverified.allowed_policies,
        vec![WorktreeDeletionPolicy::Keep]
    );
    assert_eq!(unverified.blockers, vec!["provenance_unverified"]);
    // Preview is read-only: nothing stopped, nothing removed.
    assert!(harness.runtime.calls.lock().unwrap().is_empty());
    assert_eq!(harness.workspace.count("remove:"), 0);
    assert_eq!(harness.sessions.ids().len(), 5);
}

#[test]
fn preview_counts_external_references_and_never_treats_an_incomplete_scan_as_empty() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    harness
        .references
        .scan
        .lock()
        .unwrap()
        .sessions
        .push(SessionReference {
            session_id: "other".to_string(),
            title: "Other".to_string(),
            archived: true,
            loop_owned: false,
        });
    let preview = harness.preview(&["w1"]);
    let row = &preview.worktrees[0];
    assert_eq!(row.external_references.len(), 1);
    assert_eq!(row.external_references[0].kind, "archived_session");
    assert_eq!(row.allowed_policies, vec![WorktreeDeletionPolicy::Keep]);

    harness.references.scan.lock().unwrap().sessions.clear();
    harness.references.scan.lock().unwrap().complete = false;
    let preview = harness.preview(&["w1"]);
    let row = &preview.worktrees[0];
    assert_eq!(row.checks, DeletionCheckCompleteness::Incomplete);
    assert_eq!(row.allowed_policies, vec![WorktreeDeletionPolicy::Keep]);
}

#[test]
fn preview_refuses_empty_oversized_unknown_and_system_selections() {
    let harness = Harness::new();
    assert_eq!(
        validation_code(
            harness
                .coordinator
                .preview(PreviewSessionDeletionRequest {
                    session_ids: vec![]
                })
                .unwrap_err()
        ),
        error_code::EMPTY_SELECTION
    );
    let many: Vec<String> = (0..=MAX_DELETION_BATCH)
        .map(|index| format!("s{index}"))
        .collect();
    assert_eq!(
        validation_code(
            harness
                .coordinator
                .preview(PreviewSessionDeletionRequest { session_ids: many })
                .unwrap_err()
        ),
        error_code::BATCH_TOO_LARGE
    );
    assert!(matches!(
        harness.coordinator.preview(PreviewSessionDeletionRequest {
            session_ids: vec!["missing".to_string()]
        }),
        Err(SessionsApplicationError::SessionNotFound(_))
    ));
    assert!(matches!(
        harness.coordinator.preview(PreviewSessionDeletionRequest {
            session_ids: vec!["system-activity-v1-x".to_string()]
        }),
        Err(SessionsApplicationError::Domain(_))
    ));
    assert!(harness.journal.calls.lock().unwrap().is_empty());
}

// --- Execute -----------------------------------------------------------------------------------

#[test]
fn execute_rejects_expired_previews_and_binds_requests_idempotently() {
    let harness = Harness::new();
    harness.sessions.insert(project_session("p1"));
    let preview = harness.preview(&["p1"]);
    harness
        .clock
        .unix
        .store(1_000 + PREVIEW_TTL_SECONDS as u64 + 1, Ordering::SeqCst);
    assert_eq!(
        validation_code(
            harness
                .coordinator
                .execute(ExecuteSessionDeletionRequest {
                    request_id: "r1".to_string(),
                    preview_id: preview.preview_id.clone(),
                    worktree_choices: vec![],
                })
                .unwrap_err()
        ),
        error_code::PREVIEW_EXPIRED
    );
    harness.clock.unix.store(1_000, Ordering::SeqCst);
    let preview = harness.preview(&["p1"]);
    let first = harness.execute(&preview, "r1", vec![]);
    assert!(!first.existing);
    // The preview is single-use.
    assert!(harness.previews.get(&preview.preview_id).is_none());
    // Same request id and same content, even through a stale preview id: same operation.
    let again = harness.coordinator.execute(ExecuteSessionDeletionRequest {
        request_id: "r1".to_string(),
        preview_id: preview.preview_id.clone(),
        worktree_choices: vec![],
    });
    assert_eq!(
        validation_code(again.unwrap_err()),
        error_code::PREVIEW_EXPIRED
    );
    harness.journal.claims.lock().unwrap().clear();
    let preview2 = harness.preview(&["p1"]);
    let same = harness.execute(&preview2, "r1", vec![]);
    assert!(same.existing);
    assert_eq!(same.operation_id, first.operation_id);
    // The session is claimed by the first operation, so a different request cannot take it.
    harness.journal.claims.lock().unwrap().insert(
        "p1".to_string(),
        SessionDeletionClaim {
            session_id: "p1".to_string(),
            operation_id: first.operation_id.clone(),
            group_id: "g".to_string(),
        },
    );
    let preview3 = harness.preview(&["p1"]);
    assert_eq!(
        validation_code(
            harness
                .coordinator
                .execute(ExecuteSessionDeletionRequest {
                    request_id: "r2".to_string(),
                    preview_id: preview3.preview_id,
                    worktree_choices: vec![],
                })
                .unwrap_err()
        ),
        error_code::SESSION_CLAIMED
    );
}

#[test]
fn execute_reuses_a_request_id_only_for_identical_content() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    let preview = harness.preview(&["w1"]);
    let _ = harness.execute(&preview, "r1", vec![]);
    harness.journal.claims.lock().unwrap().clear();
    let preview = harness.preview(&["w1"]);
    let error = harness
        .coordinator
        .execute(ExecuteSessionDeletionRequest {
            request_id: "r1".to_string(),
            preview_id: preview.preview_id,
            worktree_choices: vec![Harness::remove_choice("wt-1")],
        })
        .unwrap_err();
    assert_eq!(validation_code(error), error_code::REQUEST_ID_CONFLICT);
}

#[test]
fn execute_never_accepts_a_removal_the_preview_did_not_allow() {
    let harness = Harness::new();
    harness.add_unverified_worktree("u1", "/repo-old");
    let preview = harness.preview(&["u1"]);
    let key = preview.worktrees[0].worktree_key.clone();
    let error = harness
        .coordinator
        .execute(ExecuteSessionDeletionRequest {
            request_id: "r1".to_string(),
            preview_id: preview.preview_id,
            worktree_choices: vec![Harness::remove_choice(&key)],
        })
        .unwrap_err();
    assert_eq!(validation_code(error), error_code::POLICY_NOT_ALLOWED);
    assert!(harness.journal.calls.lock().unwrap().is_empty());
}

#[test]
fn a_journal_failure_before_acceptance_starts_nothing() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    harness.journal.fail_create.store(true, Ordering::SeqCst);
    let preview = harness.preview(&["w1"]);
    let error = harness
        .coordinator
        .execute(ExecuteSessionDeletionRequest {
            request_id: "r1".to_string(),
            preview_id: preview.preview_id,
            worktree_choices: vec![Harness::remove_choice("wt-1")],
        })
        .unwrap_err();
    assert!(matches!(error, SessionsApplicationError::Repository(_)));
    assert!(harness.runtime.calls.lock().unwrap().is_empty());
    assert_eq!(harness.workspace.count("remove:"), 0);
    assert_eq!(harness.sessions.ids(), vec!["w1"]);
}

// --- Keep --------------------------------------------------------------------------------------

#[test]
fn keep_deletes_sessions_after_quiescence_and_clears_the_active_session_only_when_included() {
    let harness = Harness::new();
    harness.sessions.insert(project_session("p1"));
    harness.sessions.insert(project_session("p2"));
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    *harness.sessions.active.lock().unwrap() = Some("p2".to_string());

    let preview = harness.preview(&["p1", "w1"]);
    let handle = harness.execute(&preview, "r1", vec![]);
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");

    assert_eq!(operation.outcome, DeletionOutcome::Succeeded);
    assert_eq!(harness.sessions.ids(), vec!["p2"]);
    assert_eq!(
        *harness.sessions.active.lock().unwrap(),
        Some("p2".to_string())
    );
    assert_eq!(harness.events.cleared.load(Ordering::SeqCst), 0);
    assert!(harness.events.changed.load(Ordering::SeqCst) >= 1);
    let quiesced = harness.runtime.calls.lock().unwrap().clone();
    assert!(quiesced.contains(&"p1".to_string()) && quiesced.contains(&"w1".to_string()));
    // Keep never touches Git and leaves the record retained.
    assert_eq!(harness.workspace.count("remove:"), 0);
    assert_eq!(harness.workspace.count("claim:"), 0);
    assert_eq!(harness.workspace.count("finalize_retained:wt-1"), 1);
    let worktree_group = operation
        .groups
        .iter()
        .find(|group| group.worktree_id.is_some())
        .expect("group");
    assert_eq!(worktree_group.worktree_effect, WorktreeEffect::Retained);
    assert_eq!(worktree_group.db_effect, SessionDbEffect::Deleted);
    assert_eq!(
        worktree_group.retained_path.as_deref(),
        Some("/repo-feature")
    );
    assert!(harness.journal.claims.lock().unwrap().is_empty());
}

#[test]
fn deleting_the_active_session_clears_the_selection_and_publishes_once() {
    let harness = Harness::new();
    harness.sessions.insert(project_session("p1"));
    *harness.sessions.active.lock().unwrap() = Some("p1".to_string());
    let preview = harness.preview(&["p1"]);
    let handle = harness.execute(&preview, "r1", vec![]);
    harness.coordinator.run(&handle.operation_id).expect("run");
    assert!(harness.sessions.active.lock().unwrap().is_none());
    assert_eq!(harness.events.cleared.load(Ordering::SeqCst), 1);
}

#[test]
fn a_session_that_will_not_quiesce_keeps_its_record_and_directory() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    harness.runtime.reports.lock().unwrap().insert(
        "w1".to_string(),
        QuiescenceReport {
            quiet: false,
            blockers: vec!["shell".to_string()],
        },
    );
    let preview = harness.preview(&["w1"]);
    let handle = harness.execute(&preview, "r1", vec![Harness::remove_choice("wt-1")]);
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");
    assert_eq!(operation.outcome, DeletionOutcome::Failed);
    assert_eq!(
        operation.groups[0].error_code.as_deref(),
        Some(error_code::QUIESCE_TIMEOUT)
    );
    assert_eq!(
        operation.groups[0].worktree_effect,
        WorktreeEffect::Retained
    );
    assert_eq!(operation.groups[0].db_effect, SessionDbEffect::Retained);
    assert_eq!(harness.sessions.ids(), vec!["w1"]);
    assert_eq!(harness.workspace.count("remove:"), 0);
    assert_eq!(harness.workspace.count("release:"), 1);
    assert!(harness.journal.claims.lock().unwrap().is_empty());
}

// --- Remove-safe -------------------------------------------------------------------------------

#[test]
fn remove_safe_journals_before_git_and_deletes_sessions_only_after_confirmed_removal() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1", "w2"]);
    let preview = harness.preview(&["w1", "w2"]);
    let handle = harness.execute(&preview, "r1", vec![Harness::remove_choice("wt-1")]);
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");

    assert_eq!(operation.outcome, DeletionOutcome::Succeeded);
    assert_eq!(operation.groups.len(), 1);
    assert_eq!(operation.groups[0].worktree_effect, WorktreeEffect::Removed);
    assert_eq!(operation.groups[0].db_effect, SessionDbEffect::Deleted);
    assert!(harness.sessions.ids().is_empty());
    assert_eq!(harness.workspace.count("remove:"), 1, "exactly one removal");

    let calls = harness.workspace.calls();
    let position = |needle: &str| calls.iter().position(|call| call == needle).expect(needle);
    let last = |needle: &str| calls.iter().rposition(|call| call == needle).expect(needle);
    assert!(position("claim:wt-1") < last("assess:wt-1"));
    assert!(last("assess:wt-1") < position("begin_removal:wt-1"));
    assert!(position("begin_removal:wt-1") < position("remove:wt-1"));
    assert!(position("remove:wt-1") < position("finalize_removed:wt-1"));
    assert!(position("finalize_removed:wt-1") <= position("release:wt-1"));

    let journal = harness.journal.calls.lock().unwrap().clone();
    let started = journal
        .iter()
        .position(|call| call.contains("remove_started"))
        .expect("remove_started journaled");
    let completed = journal
        .iter()
        .position(|call| call == "complete")
        .expect("complete");
    assert!(started < completed);
    // Quiescence preceded the final assessment.
    assert!(!harness.runtime.calls.lock().unwrap().is_empty());
}

#[test]
fn a_refused_removal_with_an_intact_directory_awaits_a_decision_and_frees_the_sessions() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    harness
        .workspace
        .worktrees
        .lock()
        .unwrap()
        .get_mut("wt-1")
        .unwrap()
        .report = RemovalReportView {
        outcome: RemovalOutcomeView::Refused {
            code: "worktree_dirty".to_string(),
        },
        observation: intact_observation(),
    };
    let preview = harness.preview(&["w1"]);
    let handle = harness.execute(&preview, "r1", vec![Harness::remove_choice("wt-1")]);
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");
    assert_eq!(operation.outcome, DeletionOutcome::AwaitingDecision);
    let group = &operation.groups[0];
    assert_eq!(group.status, DeletionGroupStatus::AwaitingDecision);
    assert_eq!(group.error_code.as_deref(), Some("worktree_dirty"));
    assert_eq!(group.worktree_effect, WorktreeEffect::Retained);
    assert_eq!(group.db_effect, SessionDbEffect::Retained);
    assert_eq!(harness.sessions.ids(), vec!["w1"]);
    assert_eq!(harness.workspace.count("removal_refused:wt-1"), 1);
    assert_eq!(harness.workspace.count("release:wt-1"), 1);
    assert!(
        harness.journal.claims.lock().unwrap().is_empty(),
        "claims released"
    );
}

#[test]
fn an_uncertain_removal_effect_parks_the_group_and_never_removes_again() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    harness
        .workspace
        .worktrees
        .lock()
        .unwrap()
        .get_mut("wt-1")
        .unwrap()
        .report = RemovalReportView {
        outcome: RemovalOutcomeView::TimedOut,
        observation: unknown_observation(),
    };
    let preview = harness.preview(&["w1"]);
    let handle = harness.execute(&preview, "r1", vec![Harness::remove_choice("wt-1")]);
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");
    assert_eq!(operation.outcome, DeletionOutcome::NeedsAttention);
    assert_eq!(
        operation.groups[0].worktree_effect,
        WorktreeEffect::RemovalUnknown
    );
    assert_eq!(harness.sessions.ids(), vec!["w1"]);
    assert_eq!(harness.workspace.count("attention:wt-1"), 1);
    // The claim is kept: the session must not run again until a person looks.
    assert!(harness.journal.active_claim("w1").unwrap().is_some());

    // Neither a second run nor a retry replays the removal.
    let again = harness
        .coordinator
        .run(&handle.operation_id)
        .expect("rerun");
    assert_eq!(again.outcome, DeletionOutcome::NeedsAttention);
    let retry = harness.coordinator.retry(RetrySessionDeletionRequest {
        operation_id: handle.operation_id.clone(),
        expected_revision: again.revision,
        retry_request_id: "retry-1".to_string(),
        preview_id: None,
        worktree_choices: vec![],
    });
    assert_eq!(
        validation_code(retry.unwrap_err()),
        error_code::RETRY_NOT_ALLOWED
    );
    assert_eq!(harness.workspace.count("remove:"), 1);
}

#[test]
fn a_timed_out_removal_that_is_observed_gone_counts_as_removed() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    harness
        .workspace
        .worktrees
        .lock()
        .unwrap()
        .get_mut("wt-1")
        .unwrap()
        .report = RemovalReportView {
        outcome: RemovalOutcomeView::TimedOut,
        observation: removed_observation(),
    };
    let preview = harness.preview(&["w1"]);
    let handle = harness.execute(&preview, "r1", vec![Harness::remove_choice("wt-1")]);
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");
    assert_eq!(operation.outcome, DeletionOutcome::Succeeded);
    assert_eq!(operation.groups[0].worktree_effect, WorktreeEffect::Removed);
    let snapshot = harness
        .journal
        .group_snapshot(&handle.operation_id, &operation.groups[0].group_id)
        .unwrap()
        .unwrap();
    assert_eq!(
        snapshot.receipt.unwrap()["kind"],
        "removed_observed_after_timeout"
    );
}

#[test]
fn git_success_followed_by_a_database_failure_leaves_finalize_pending_with_claims_held() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    harness.journal.fail_complete.store(true, Ordering::SeqCst);
    let preview = harness.preview(&["w1"]);
    let handle = harness.execute(&preview, "r1", vec![Harness::remove_choice("wt-1")]);
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");
    assert_eq!(operation.outcome, DeletionOutcome::NeedsAttention);
    let group = &operation.groups[0];
    assert_eq!(group.status, DeletionGroupStatus::FinalizePending);
    assert_eq!(group.worktree_effect, WorktreeEffect::Removed);
    assert_eq!(group.db_effect, SessionDbEffect::Pending);
    assert!(harness.journal.active_claim("w1").unwrap().is_some());
    assert!(harness
        .coordinator
        .ensure_session_admits_execution("w1")
        .is_err());

    // Retry is database-only: no new preview needed, no second removal.
    harness.journal.fail_complete.store(false, Ordering::SeqCst);
    let retry = harness
        .coordinator
        .retry(RetrySessionDeletionRequest {
            operation_id: handle.operation_id.clone(),
            expected_revision: operation.revision,
            retry_request_id: "retry-1".to_string(),
            preview_id: None,
            worktree_choices: vec![],
        })
        .expect("retry");
    let finished = harness
        .coordinator
        .run(&retry.operation_id)
        .expect("run retry");
    assert_eq!(finished.outcome, DeletionOutcome::Succeeded);
    assert!(harness.sessions.ids().is_empty());
    assert_eq!(harness.workspace.count("remove:"), 1);
    assert!(harness.journal.claims.lock().unwrap().is_empty());
}

#[test]
fn an_internal_error_before_any_removal_fails_only_that_group_and_frees_its_sessions() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-a", &["w1"]);
    harness.add_worktree("wt-2", "/repo-b", &["w2"]);
    harness
        .workspace
        .fail_claim
        .lock()
        .unwrap()
        .push("wt-1".to_string());
    let preview = harness.preview(&["w1", "w2"]);
    let handle = harness.execute(
        &preview,
        "r1",
        vec![
            Harness::remove_choice("wt-1"),
            Harness::remove_choice("wt-2"),
        ],
    );
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");
    // The operation reached a recorded end even though one step raised.
    assert_eq!(operation.outcome, DeletionOutcome::Partial);
    let failed = operation
        .groups
        .iter()
        .find(|group| group.worktree_id.as_deref() == Some("wt-1"))
        .expect("wt-1 group");
    assert_eq!(failed.status, DeletionGroupStatus::Failed);
    assert_eq!(failed.worktree_effect, WorktreeEffect::Retained);
    assert_eq!(failed.db_effect, SessionDbEffect::Retained);
    assert_eq!(failed.error_code.as_deref(), Some(error_code::RUN_FAILED));
    assert!(harness.journal.active_claim("w1").unwrap().is_none());
    assert!(harness.sessions.ids().contains(&"w1".to_string()));
    // The other group was not affected.
    let succeeded = operation
        .groups
        .iter()
        .find(|group| group.worktree_id.as_deref() == Some("wt-2"))
        .expect("wt-2 group");
    assert_eq!(succeeded.status, DeletionGroupStatus::Succeeded);
    assert_eq!(harness.workspace.count("remove:wt-2"), 1);
    assert_eq!(harness.workspace.count("remove:wt-1"), 0);
}

#[test]
fn an_internal_error_after_remove_started_parks_the_group_with_its_claims_held() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    harness
        .workspace
        .fail_begin_removal
        .lock()
        .unwrap()
        .push("wt-1".to_string());
    let preview = harness.preview(&["w1"]);
    let handle = harness.execute(&preview, "r1", vec![Harness::remove_choice("wt-1")]);
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");
    assert_eq!(operation.outcome, DeletionOutcome::NeedsAttention);
    let group = &operation.groups[0];
    assert_eq!(group.status, DeletionGroupStatus::NeedsAttention);
    // `remove_started` was journaled before the failing step, so nobody may assume the
    // directory is intact: the sessions stay claimed and the removal was never asked for.
    assert_eq!(group.worktree_effect, WorktreeEffect::RemoveStarted);
    assert_eq!(group.error_code.as_deref(), Some(error_code::RUN_FAILED));
    assert!(harness.journal.active_claim("w1").unwrap().is_some());
    assert_eq!(harness.workspace.count("remove:"), 0);
    assert_eq!(harness.workspace.count("release:wt-1"), 1);
}

#[test]
fn changes_after_the_preview_are_caught_by_the_final_revalidation() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    let preview = harness.preview(&["w1"]);
    let handle = harness.execute(&preview, "r1", vec![Harness::remove_choice("wt-1")]);
    // A tracked file changed between preview and execution.
    harness.workspace.set_assessment("wt-1", |assessment| {
        assessment.allowed_policies = vec![WorktreeDeletionPolicy::Keep];
        assessment.blockers = vec!["tracked_changes".to_string()];
    });
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");
    assert_eq!(operation.outcome, DeletionOutcome::AwaitingDecision);
    assert_eq!(
        operation.groups[0].error_code.as_deref(),
        Some("tracked_changes")
    );
    assert_eq!(harness.workspace.count("remove:"), 0);
    assert_eq!(harness.workspace.count("begin_removal:"), 0);
    assert_eq!(harness.sessions.ids(), vec!["w1"]);
}

#[test]
fn an_identity_that_drifted_after_the_preview_is_refused_before_git_runs() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    let preview = harness.preview(&["w1"]);
    let handle = harness.execute(&preview, "r1", vec![Harness::remove_choice("wt-1")]);
    harness.workspace.set_assessment("wt-1", |assessment| {
        assessment.identity = Some(WorktreeIdentityView {
            head: Some("moved".to_string()),
            ..identity("/repo-feature")
        });
    });
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");
    assert_eq!(
        operation.groups[0].error_code.as_deref(),
        Some(error_code::IDENTITY_CHANGED)
    );
    assert_eq!(harness.workspace.count("remove:"), 0);
}

#[test]
fn ignored_acknowledgements_are_rechecked_against_the_current_inventory() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    harness.workspace.set_assessment("wt-1", |assessment| {
        assessment.requires_ignored_acknowledgement = true;
        assessment.ignored.as_mut().unwrap().total_entries = 2;
        assessment.ignored.as_mut().unwrap().fingerprint = "fp-a".to_string();
    });
    let preview = harness.preview(&["w1"]);
    assert!(preview.worktrees[0].requires_ignored_acknowledgement);
    let unacknowledged = harness.coordinator.execute(ExecuteSessionDeletionRequest {
        request_id: "r0".to_string(),
        preview_id: preview.preview_id.clone(),
        worktree_choices: vec![Harness::remove_choice("wt-1")],
    });
    assert_eq!(
        validation_code(unacknowledged.unwrap_err()),
        error_code::IGNORED_ACKNOWLEDGEMENT_REQUIRED
    );
    let handle = harness.execute(
        &preview,
        "r1",
        vec![WorktreeDeletionChoice {
            worktree_key: "wt-1".to_string(),
            policy: WorktreeDeletionPolicy::RemoveSafe,
            ignored_files_acknowledgement: Some(IgnoredFilesAcknowledgement {
                fingerprint: "fp-a".to_string(),
            }),
        }],
    );
    // Ignored metadata changed while the Agent was being stopped.
    harness.workspace.set_assessment("wt-1", |assessment| {
        assessment.ignored.as_mut().unwrap().fingerprint = "fp-b".to_string();
    });
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");
    assert_eq!(
        operation.groups[0].error_code.as_deref(),
        Some(error_code::IGNORED_ACKNOWLEDGEMENT_STALE)
    );
    assert_eq!(harness.workspace.count("remove:"), 0);
}

#[test]
fn a_gate_held_elsewhere_blocks_removal_without_stopping_anything() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    harness
        .workspace
        .worktrees
        .lock()
        .unwrap()
        .get_mut("wt-1")
        .unwrap()
        .gate_held = true;
    let preview = harness.preview(&["w1"]);
    let handle = harness.execute(&preview, "r1", vec![Harness::remove_choice("wt-1")]);
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");
    assert_eq!(
        operation.groups[0].error_code.as_deref(),
        Some(error_code::GATE_HELD)
    );
    assert!(harness.runtime.calls.lock().unwrap().is_empty());
    assert_eq!(harness.sessions.ids(), vec!["w1"]);
}

#[test]
fn a_reference_that_appeared_after_the_preview_blocks_removal() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    let preview = harness.preview(&["w1"]);
    let handle = harness.execute(&preview, "r1", vec![Harness::remove_choice("wt-1")]);
    harness
        .references
        .scan
        .lock()
        .unwrap()
        .sessions
        .push(SessionReference {
            session_id: "late".to_string(),
            title: "Late".to_string(),
            archived: false,
            loop_owned: false,
        });
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");
    assert_eq!(
        operation.groups[0].error_code.as_deref(),
        Some("external_references")
    );
    assert_eq!(harness.workspace.count("remove:"), 0);
}

// --- Batch -------------------------------------------------------------------------------------

#[test]
fn independent_groups_keep_their_own_outcomes_and_aggregate_to_partial() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-a", &["a1"]);
    harness.add_worktree("wt-2", "/repo-b", &["b1"]);
    harness
        .workspace
        .worktrees
        .lock()
        .unwrap()
        .get_mut("wt-2")
        .unwrap()
        .report = RemovalReportView {
        outcome: RemovalOutcomeView::Refused {
            code: "locked".to_string(),
        },
        observation: intact_observation(),
    };
    let preview = harness.preview(&["a1", "b1"]);
    let handle = harness.execute(
        &preview,
        "r1",
        vec![
            Harness::remove_choice("wt-1"),
            Harness::remove_choice("wt-2"),
        ],
    );
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");
    assert_eq!(operation.outcome, DeletionOutcome::Partial);
    let a = operation
        .groups
        .iter()
        .find(|group| group.worktree_id.as_deref() == Some("wt-1"))
        .unwrap();
    let b = operation
        .groups
        .iter()
        .find(|group| group.worktree_id.as_deref() == Some("wt-2"))
        .unwrap();
    assert_eq!(a.status, DeletionGroupStatus::Succeeded);
    assert_eq!(b.status, DeletionGroupStatus::AwaitingDecision);
    assert_eq!(harness.sessions.ids(), vec!["b1"]);
    assert_eq!(harness.workspace.count("remove:wt-1"), 1);
    assert_eq!(harness.workspace.count("remove:wt-2"), 1);

    // Retry replays only the unfinished group, and only with a fresh preview for remove-safe.
    let no_preview = harness.coordinator.retry(RetrySessionDeletionRequest {
        operation_id: handle.operation_id.clone(),
        expected_revision: operation.revision,
        retry_request_id: "retry-1".to_string(),
        preview_id: None,
        worktree_choices: vec![],
    });
    assert_eq!(
        validation_code(no_preview.unwrap_err()),
        error_code::RETRY_REQUIRES_PREVIEW
    );
    harness
        .workspace
        .worktrees
        .lock()
        .unwrap()
        .get_mut("wt-2")
        .unwrap()
        .report = RemovalReportView {
        outcome: RemovalOutcomeView::Succeeded,
        observation: removed_observation(),
    };
    let fresh = harness.preview(&["b1"]);
    let retry = harness
        .coordinator
        .retry(RetrySessionDeletionRequest {
            operation_id: handle.operation_id.clone(),
            expected_revision: operation.revision,
            retry_request_id: "retry-2".to_string(),
            preview_id: Some(fresh.preview_id.clone()),
            worktree_choices: vec![Harness::remove_choice("wt-2")],
        })
        .expect("retry");
    // Same retry id again: the same handle, no new attempt.
    let replay = harness
        .coordinator
        .retry(RetrySessionDeletionRequest {
            operation_id: handle.operation_id.clone(),
            expected_revision: harness
                .coordinator
                .get(&handle.operation_id)
                .unwrap()
                .revision,
            retry_request_id: "retry-2".to_string(),
            preview_id: None,
            worktree_choices: vec![],
        })
        .expect("replayed retry");
    assert!(replay.existing);
    let finished = harness
        .coordinator
        .run(&retry.operation_id)
        .expect("run retry");
    assert_eq!(finished.outcome, DeletionOutcome::Succeeded);
    assert_eq!(
        harness.workspace.count("remove:wt-1"),
        1,
        "the finished group is never replayed"
    );
    assert_eq!(harness.workspace.count("remove:wt-2"), 2);
    assert!(harness.sessions.ids().is_empty());
}

#[test]
fn retry_refuses_a_stale_revision_and_a_preview_over_different_targets() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-a", &["a1", "a2"]);
    harness
        .workspace
        .worktrees
        .lock()
        .unwrap()
        .get_mut("wt-1")
        .unwrap()
        .gate_held = true;
    let preview = harness.preview(&["a1", "a2"]);
    let handle = harness.execute(&preview, "r1", vec![Harness::remove_choice("wt-1")]);
    let operation = harness.coordinator.run(&handle.operation_id).expect("run");
    assert_eq!(
        validation_code(
            harness
                .coordinator
                .retry(RetrySessionDeletionRequest {
                    operation_id: handle.operation_id.clone(),
                    expected_revision: operation.revision + 5,
                    retry_request_id: "retry-1".to_string(),
                    preview_id: None,
                    worktree_choices: vec![],
                })
                .unwrap_err()
        ),
        error_code::REVISION_CONFLICT
    );
    harness
        .workspace
        .worktrees
        .lock()
        .unwrap()
        .get_mut("wt-1")
        .unwrap()
        .gate_held = false;
    let narrower = harness.preview(&["a1"]);
    assert_eq!(
        validation_code(
            harness
                .coordinator
                .retry(RetrySessionDeletionRequest {
                    operation_id: handle.operation_id.clone(),
                    expected_revision: operation.revision,
                    retry_request_id: "retry-2".to_string(),
                    preview_id: Some(narrower.preview_id),
                    worktree_choices: vec![Harness::remove_choice("wt-1")],
                })
                .unwrap_err()
        ),
        error_code::PREVIEW_TARGET_MISMATCH
    );
}

// --- Recovery ----------------------------------------------------------------------------------

fn interrupted_operation(harness: &Harness, effect: WorktreeEffect) -> String {
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    let preview = harness.preview(&["w1"]);
    let handle = harness.execute(&preview, "r1", vec![Harness::remove_choice("wt-1")]);
    let group_id = harness
        .coordinator
        .get(&handle.operation_id)
        .unwrap()
        .groups[0]
        .group_id
        .clone();
    harness
        .journal
        .update_group(
            &handle.operation_id,
            &group_id,
            1,
            &GroupPatch {
                status: Some(DeletionGroupStatus::Running),
                worktree_effect: Some(effect),
                ..GroupPatch::default()
            },
        )
        .unwrap();
    handle.operation_id
}

#[test]
fn recovery_finalizes_a_removal_it_can_observe_and_never_reruns_git() {
    let harness = Harness::new();
    let operation_id = interrupted_operation(&harness, WorktreeEffect::RemoveStarted);
    harness.coordinator.reconcile_pending().expect("reconcile");
    let operation = harness.coordinator.get(&operation_id).unwrap();
    assert_eq!(operation.outcome, DeletionOutcome::Succeeded);
    assert_eq!(operation.groups[0].worktree_effect, WorktreeEffect::Removed);
    let receipt = harness
        .journal
        .group_snapshot(&operation_id, &operation.groups[0].group_id)
        .unwrap()
        .unwrap()
        .receipt
        .unwrap();
    assert_eq!(receipt["kind"], "removed_observed_after_interruption");
    assert_eq!(harness.workspace.count("remove:"), 0);
    assert!(harness.sessions.ids().is_empty());
}

#[test]
fn recovery_with_an_intact_directory_asks_for_a_new_decision() {
    let harness = Harness::new();
    let operation_id = interrupted_operation(&harness, WorktreeEffect::RemoveStarted);
    harness
        .workspace
        .worktrees
        .lock()
        .unwrap()
        .get_mut("wt-1")
        .unwrap()
        .observation = Some(intact_observation());
    harness.coordinator.reconcile_pending().expect("reconcile");
    let operation = harness.coordinator.get(&operation_id).unwrap();
    assert_eq!(operation.outcome, DeletionOutcome::AwaitingDecision);
    assert_eq!(
        operation.groups[0].error_code.as_deref(),
        Some(error_code::INTERRUPTED)
    );
    assert_eq!(harness.workspace.count("removal_refused:wt-1"), 1);
    assert_eq!(harness.sessions.ids(), vec!["w1"]);
    assert!(harness.journal.claims.lock().unwrap().is_empty());
}

#[test]
fn recovery_parks_ambiguous_or_offline_resources() {
    let harness = Harness::new();
    let operation_id = interrupted_operation(&harness, WorktreeEffect::RemoveStarted);
    harness
        .workspace
        .worktrees
        .lock()
        .unwrap()
        .get_mut("wt-1")
        .unwrap()
        .observation = Some(unknown_observation());
    harness.coordinator.reconcile_pending().expect("reconcile");
    let operation = harness.coordinator.get(&operation_id).unwrap();
    assert_eq!(operation.outcome, DeletionOutcome::NeedsAttention);
    assert_eq!(harness.workspace.count("attention:wt-1"), 1);
    assert_eq!(harness.workspace.count("remove:"), 0);
    assert!(harness.journal.active_claim("w1").unwrap().is_some());
}

#[test]
fn recovery_of_an_operation_that_never_started_git_releases_the_sessions() {
    let harness = Harness::new();
    let operation_id = interrupted_operation(&harness, WorktreeEffect::NotRequested);
    harness.coordinator.reconcile_pending().expect("reconcile");
    let operation = harness.coordinator.get(&operation_id).unwrap();
    assert_eq!(operation.outcome, DeletionOutcome::AwaitingDecision);
    assert_eq!(harness.workspace.count("observe:"), 0);
    assert!(harness.journal.claims.lock().unwrap().is_empty());
}

#[test]
fn recovery_leaves_an_operation_owned_by_a_live_instance_alone() {
    let harness = Harness::new();
    let operation_id = interrupted_operation(&harness, WorktreeEffect::RemoveStarted);
    harness
        .journal
        .update_operation(
            &operation_id,
            harness.coordinator.get(&operation_id).unwrap().revision,
            &OperationPatch {
                outcome: None,
                phase: None,
                error_code: None,
                completed: false,
                owner: Some(DeletionOwner {
                    instance_id: "other".to_string(),
                    epoch: 9,
                }),
                last_retry_request_id: None,
            },
        )
        .unwrap();
    harness
        .owner
        .alive
        .lock()
        .unwrap()
        .insert("other".to_string(), true);
    harness.coordinator.reconcile_pending().expect("reconcile");
    assert_eq!(
        harness.coordinator.get(&operation_id).unwrap().outcome,
        DeletionOutcome::Pending
    );
    // Once the other instance is provably gone, this one takes over.
    harness
        .owner
        .alive
        .lock()
        .unwrap()
        .insert("other".to_string(), false);
    harness.coordinator.reconcile_pending().expect("reconcile");
    assert_eq!(
        harness.coordinator.get(&operation_id).unwrap().outcome,
        DeletionOutcome::Succeeded
    );
}

// --- Legacy path and admission -----------------------------------------------------------------

#[test]
fn the_legacy_keep_only_path_runs_the_same_coordinator_and_respects_claims() {
    let harness = Harness::new();
    harness.add_worktree("wt-1", "/repo-feature", &["w1"]);
    harness
        .coordinator
        .delete_keep_only("w1")
        .expect("keep-only delete");
    assert!(harness.sessions.ids().is_empty());
    assert_eq!(harness.workspace.count("remove:"), 0);
    assert_eq!(harness.workspace.count("finalize_retained:wt-1"), 1);

    harness.sessions.insert(project_session("p1"));
    harness.journal.claims.lock().unwrap().insert(
        "p1".to_string(),
        SessionDeletionClaim {
            session_id: "p1".to_string(),
            operation_id: "elsewhere".to_string(),
            group_id: "g".to_string(),
        },
    );
    assert_eq!(
        validation_code(harness.coordinator.delete_keep_only("p1").unwrap_err()),
        error_code::SESSION_CLAIMED
    );
    assert_eq!(harness.sessions.ids(), vec!["p1"]);
    assert_eq!(
        validation_code(
            harness
                .coordinator
                .ensure_session_admits_execution("p1")
                .unwrap_err()
        ),
        error_code::SESSION_CLAIMED
    );
    assert!(harness
        .coordinator
        .ensure_session_admits_execution("unknown")
        .is_ok());
}
