use super::loop_scope_evidence::{seal_phase_evidence, PhaseEvidenceOutcome};
use super::{
    ActiveLoopOperation, AgentClockPort, AgentLogLevel, AgentRuntimeApplicationError,
    CanonicalLoopSignal, LoopAssessmentService, LoopEvidenceView, LoopGenerationControlPort,
    LoopGuardRole, LoopIterationRepository, LoopIterationView, LoopOperationContext,
    LoopOperationKind, LoopOperationObserver, LoopProgressApplicationService, LoopProjectPort,
    LoopRepository, LoopRoleGenerationCompletionPort, LoopRunView, LoopScopeBinding,
    LoopScopePlatformPort, LoopScopeRef, LoopVerificationApplicationService,
    LoopVerificationCancellation, LoopVerificationScope, LoopVerifierApplicationService,
    LoopVerifierContextPort, LoopWorkerApplicationService, RunLoopVerificationRequest,
};
use crate::contexts::agent_runtime::domain::{
    LoopRunPhase, LoopRunStatus, LoopTerminalReason, LOOP_SCOPE_SCHEMA_VERSION,
};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct LoopOrchestratorPorts {
    pub(crate) loops: Arc<dyn LoopRepository>,
    pub(crate) iterations: Arc<dyn LoopIterationRepository>,
    pub(crate) projects: Arc<dyn LoopProjectPort>,
    pub(crate) verifier_context: Arc<dyn LoopVerifierContextPort>,
    pub(crate) completions: Arc<dyn LoopRoleGenerationCompletionPort>,
    pub(crate) generations: Arc<dyn LoopGenerationControlPort>,
    pub(crate) worker: LoopWorkerApplicationService,
    pub(crate) verification: LoopVerificationApplicationService,
    pub(crate) verifier: LoopVerifierApplicationService,
    pub(crate) progress: LoopProgressApplicationService,
    pub(crate) observer: LoopOperationObserver,
    pub(crate) clock: Arc<dyn AgentClockPort>,
    pub(crate) scope_platform: Arc<dyn LoopScopePlatformPort>,
    pub(crate) assessment: LoopAssessmentService,
}

#[derive(Clone)]
pub(crate) struct LoopOrchestratorApplicationService {
    pub(super) ports: LoopOrchestratorPorts,
}

impl LoopOrchestratorApplicationService {
    pub(crate) fn new(ports: LoopOrchestratorPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn execute(
        &self,
        run_id: &str,
        cancellation: LoopVerificationCancellation,
    ) -> Result<(), AgentRuntimeApplicationError> {
        loop {
            match self.execute_inner(run_id, &cancellation) {
                Ok(()) => return Ok(()),
                Err(error) if self.handle_failure(run_id, &error)? => {
                    thread::sleep(Duration::from_millis(250));
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn execute_inner(
        &self,
        run_id: &str,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<(), AgentRuntimeApplicationError> {
        loop {
            let view = self.run_view(run_id)?;
            if view.status.is_terminal()
                || view.status == LoopRunStatus::AwaitingAcceptance
                || view.status == LoopRunStatus::Paused
            {
                return Ok(());
            }
            if cancellation.is_cancelled() {
                self.stop_active_role(&view);
                return Ok(());
            }
            let mut limited_run = self.run(run_id)?;
            let snapshot = self.snapshot(run_id)?;
            let expected = limited_run.status();
            if limited_run
                .enforce_elapsed_limits(
                    elapsed_seconds(&view.created_at),
                    0,
                    &snapshot.values().limits,
                )?
                .is_some()
            {
                let now = self.ports.clock.now();
                self.save_run(&limited_run, expected, Some(&now))?;
                return Ok(());
            }
            match (view.status, view.phase) {
                (LoopRunStatus::Queued, LoopRunPhase::Preparing) => {
                    self.prepare(&view, cancellation)?
                }
                (LoopRunStatus::Running, LoopRunPhase::Acting) => self.act(&view, cancellation)?,
                (LoopRunStatus::Running, LoopRunPhase::Verifying) => {
                    self.verify(&view, cancellation)?
                }
                (LoopRunStatus::Running, LoopRunPhase::Deciding) => {
                    self.decide(&view, cancellation)?
                }
                _ => return Ok(()),
            }
            if self.pause_at_boundary(run_id)? {
                return Ok(());
            }
        }
    }

    fn prepare(
        &self,
        view: &LoopRunView,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let context = LoopOperationContext {
            run_id: view.id.clone(),
            iteration_id: None,
            kind: LoopOperationKind::Worktree,
        };
        let operation = ActiveLoopOperation {
            id: required(&view.active_operation_id, "Loop preparation operation")?,
            context,
        };
        self.ports.observer.record(
            &operation.context,
            Some(&operation.id),
            AgentLogLevel::Info,
            "Preparing the isolated Loop worktree.",
        )?;
        let worktree_path = match &view.worktree_path {
            Some(path) => path.clone(),
            None => {
                let created = self.ports.projects.prepare_loop_worktree(
                    &view.project_path,
                    &view.id,
                    &view.definition_snapshot.base_branch,
                )?;
                self.ports.loops.attach_run_worktree(
                    &view.id,
                    &created.path,
                    &created.name,
                    &created.branch,
                    LoopRunStatus::Queued,
                )?;
                created.path
            }
        };
        // The binding is the first thing that happens after the worktree exists and the last
        // thing before any Agent or verification effect. Failure here leaves a paused run with a
        // recorded reason and no child side effect.
        if let Err(pause) = self.bind_scope(view, &worktree_path, &operation, cancellation)? {
            self.pause_for_scope(
                &view.id,
                LoopRunStatus::Queued,
                pause.0,
                &pause.1,
                Some(&operation),
            )?;
            return Ok(());
        }
        self.ports
            .observer
            .complete(&operation, "The isolated Loop worktree is ready.")?;
        let mut run = self.run(&view.id)?;
        run.begin()?;
        let snapshot = self.snapshot(&view.id)?;
        run.record_runtime_outcome(false, &snapshot.values().limits)?;
        self.save_run(&run, LoopRunStatus::Queued, None)?;
        self.ports
            .observer
            .signal_canonical_loop(&view.id, CanonicalLoopSignal::Running)
    }

    /// Binds root identity and baseline once. Returns `Ok(Err((reason, detail)))` when the run
    /// must pause instead.
    fn bind_scope(
        &self,
        view: &LoopRunView,
        worktree_path: &str,
        operation: &ActiveLoopOperation,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<Result<(), (LoopTerminalReason, String)>, AgentRuntimeApplicationError> {
        let record = self.ports.loops.find_run_scope(&view.id)?;
        if record
            .as_ref()
            .is_some_and(|record| record.binding.is_some())
        {
            return Ok(Ok(()));
        }
        let snapshot = self.snapshot(&view.id)?;
        let Some(mode) = snapshot.requested_mode() else {
            return Ok(Err((
                LoopTerminalReason::ScopeBindingMissing,
                "The run's definition snapshot has no verified scope; cancel it and start a new run.".to_string(),
            )));
        };
        let scope = match snapshot.scope() {
            Ok(scope) => scope,
            Err(error) => {
                return Ok(Err((
                    LoopTerminalReason::ScopeBindingMissing,
                    error.to_string(),
                )))
            }
        };
        let frozen = record.and_then(|record| record.assessment);
        let current = self.ports.assessment.assess(&snapshot)?;
        match &frozen {
            Some(frozen)
                if frozen.witness_digest == current.witness_digest
                    && current.satisfies_requested_mode => {}
            _ => {
                return Ok(Err((
                    LoopTerminalReason::ScopeCapabilityChanged,
                    format!(
                        "Execution capability changed before the first effect: {}",
                        current.blockers.join("; ")
                    ),
                )))
            }
        }
        let bound = match self
            .ports
            .scope_platform
            .bind_root(&view.id, worktree_path, cancellation)
        {
            Ok(bound) => bound,
            Err(failure) => {
                return Ok(Err((
                    LoopTerminalReason::ScopeUnverifiable,
                    format!("{}: {}", failure.code, failure.message),
                )))
            }
        };
        let binding = LoopScopeBinding {
            schema_version: LOOP_SCOPE_SCHEMA_VERSION,
            allowed_paths: scope.allowed_display(),
            protected_paths: scope.protected_display(),
            requested_mode: mode.as_str().to_string(),
            definition_version: snapshot.values().version,
            scope_digest: scope.digest(mode),
            root: bound.root,
            base_commit: self.ports.projects.head_commit(worktree_path)?,
            baseline_manifest_id: bound.baseline_manifest_id.clone(),
            baseline_digest: bound.baseline_digest.clone(),
            witness_digest: current.witness_digest.clone(),
            audit_receipt_id: None,
            bound_at: self.ports.clock.now(),
        };
        self.ports
            .loops
            .attach_run_scope_binding(&view.id, &binding, LoopRunStatus::Queued)?;
        self.ports.iterations.append_evidence(&LoopEvidenceView {
            id: format!("loop-evidence-{}", Uuid::new_v4()),
            run_id: view.id.clone(),
            iteration_id: None,
            kind: "scope-binding".to_string(),
            status: "passed".to_string(),
            summary: "Bound the real worktree root identity and sealed the complete baseline."
                .to_string(),
            operation_id: Some(operation.id.clone()),
            command_id: None,
            exit_code: None,
            duration_ms: None,
            details: Some(serde_json::json!({
                "scopeDigest": binding.scope_digest,
                "requestedMode": binding.requested_mode,
                "baselineDigest": binding.baseline_digest,
                "baselineEntries": bound.baseline_entries,
                "witnessDigest": binding.witness_digest,
                "caseRule": binding.root.case_rule,
            })),
            created_at: self.ports.clock.now(),
        })?;
        Ok(Ok(()))
    }

    /// Loads the binding and re-proves root identity and capability before any effect. Returns
    /// `Ok(None)` after pausing the run.
    pub(super) fn admitted_binding(
        &self,
        view: &LoopRunView,
        expected_status: LoopRunStatus,
    ) -> Result<Option<LoopScopeBinding>, AgentRuntimeApplicationError> {
        let record = self.ports.loops.find_run_scope(&view.id)?;
        let Some(binding) = record.and_then(|record| record.binding) else {
            self.pause_for_scope(
                &view.id,
                expected_status,
                LoopTerminalReason::ScopeBindingMissing,
                "This run has no trustworthy scope binding; cancel it and start a new run from the confirmed definition.",
                None,
            )?;
            return Ok(None);
        };
        let worktree_path = required(&view.worktree_path, "Loop worktree path")?;
        if let Err(failure) = self
            .ports
            .scope_platform
            .verify_root(&worktree_path, &binding.root)
        {
            self.pause_for_scope(
                &view.id,
                expected_status,
                LoopTerminalReason::ScopeUnverifiable,
                &format!("{}: {}", failure.code, failure.message),
                None,
            )?;
            return Ok(None);
        }
        let snapshot = self.snapshot(&view.id)?;
        let current = self.ports.assessment.assess(&snapshot)?;
        if current.witness_digest != binding.witness_digest || !current.satisfies_requested_mode {
            self.pause_for_scope(
                &view.id,
                expected_status,
                LoopTerminalReason::ScopeCapabilityChanged,
                &format!(
                    "Execution capability no longer matches the frozen assessment: {}",
                    current.blockers.join("; ")
                ),
                None,
            )?;
            return Ok(None);
        }
        Ok(Some(binding))
    }

    fn act(
        &self,
        view: &LoopRunView,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let Some(binding) = self.admitted_binding(view, LoopRunStatus::Running)? else {
            return Ok(());
        };
        let current = view
            .iterations
            .iter()
            .find(|item| item.sequence == view.current_iteration);
        if current
            .and_then(|item| item.worker_summary.as_ref())
            .is_none()
        {
            let mut request = self.worker_request(view);
            request.scope_ref = Some(scope_ref(&binding, LoopGuardRole::Worker));
            let started = match current {
                Some(item) => self.ports.worker.resume_iteration(&item.id, request)?,
                None => self.ports.worker.start_iteration(request)?,
            };
            let operation = match self.ports.observer.start(
                LoopOperationContext {
                    run_id: view.id.clone(),
                    iteration_id: Some(started.iteration_id.clone()),
                    kind: LoopOperationKind::RoleGeneration,
                },
                "Running the Loop Worker role",
            ) {
                Ok(operation) => operation,
                Err(error) => {
                    let _ = self
                        .ports
                        .generations
                        .stop_loop_generation(&started.session_id);
                    return Err(error);
                }
            };
            let result = (|| {
                let terminal = self.await_terminal(
                    &started.session_id,
                    view.definition_snapshot.limits.step_timeout_seconds,
                    cancellation,
                )?;
                self.ports.worker.complete(terminal)
            })();
            match result {
                Ok(_) => self
                    .ports
                    .observer
                    .complete(&operation, "The Loop Worker role completed.")?,
                Err(error) => {
                    self.finish_role_error(&operation, &error, cancellation);
                    return Err(error);
                }
            }
        }
        let refreshed = self.run_view(&view.id)?;
        let iteration = current_iteration(&refreshed)?.clone();
        if !self.seal_phase(&refreshed, &iteration, "worker", &binding, cancellation)? {
            return Ok(());
        }
        let mut run = self.run(&view.id)?;
        let snapshot = self.snapshot(&view.id)?;
        run.record_runtime_outcome(false, &snapshot.values().limits)?;
        run.move_to(LoopRunPhase::Verifying)?;
        self.save_run(&run, LoopRunStatus::Running, None)?;
        self.ports
            .observer
            .signal_canonical_loop(&view.id, CanonicalLoopSignal::Verifying)
    }

    fn verify(
        &self,
        view: &LoopRunView,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let Some(binding) = self.admitted_binding(view, LoopRunStatus::Running)? else {
            return Ok(());
        };
        let record = self.ports.loops.find_run_scope(&view.id)?;
        let assessment_digest = record
            .and_then(|record| record.assessment)
            .map(|assessment| super::assessment_digest(&assessment))
            .unwrap_or_default();
        // Checks run against a manifest captured now, never against the Worker-phase manifest: a
        // pause boundary or an audited process check may have changed the tree since. A check
        // that rewrites files invalidates its own input, so the phase repeats until the manifest
        // after the checks equals the one they ran on, within a small bound.
        for round in 0..MAX_VERIFICATION_ROUNDS {
            let current = self.run_view(&view.id)?;
            let iteration = current_iteration(&current)?.clone();
            if !self.seal_phase(
                &current,
                &iteration,
                VERIFICATION_INPUT_PHASE,
                &binding,
                cancellation,
            )? {
                return Ok(());
            }
            let current = self.run_view(&view.id)?;
            let iteration = current_iteration(&current)?.clone();
            let Some((input_manifest_id, input_digest)) =
                super::loop_scope_evidence::latest_passed_manifest(
                    &iteration,
                    VERIFICATION_INPUT_PHASE,
                )
            else {
                self.pause_for_scope(
                    &view.id,
                    LoopRunStatus::Running,
                    LoopTerminalReason::ScopeUnverifiable,
                    "The verification input manifest could not be established.",
                    None,
                )?;
                return Ok(());
            };
            let scope = LoopVerificationScope {
                binding: binding.clone(),
                input_manifest_id,
                input_digest: input_digest.clone(),
                assessment_digest: assessment_digest.clone(),
            };
            let existing = view
                .definition_snapshot
                .verification_commands
                .iter()
                .all(|command| {
                    let expected =
                        super::loop_verification::verification_fingerprint(command, &scope);
                    iteration.evidence.iter().any(|item| {
                        item.kind == "verification-command"
                            && item.command_id.as_deref() == Some(command.id.as_str())
                            && item
                                .details
                                .as_ref()
                                .and_then(|details| details.get("fingerprint"))
                                .and_then(serde_json::Value::as_str)
                                == Some(expected.as_str())
                    })
                });
            if !existing {
                let result = self
                    .ports
                    .verification
                    .run_commands(RunLoopVerificationRequest {
                        run_id: view.id.clone(),
                        iteration_id: iteration.id.clone(),
                        worktree_root: required(&view.worktree_path, "Loop worktree path")?,
                        commands: view.definition_snapshot.verification_commands.clone(),
                        cancellation: cancellation.clone(),
                        scope: Some(scope),
                    })?;
                if result.cancelled {
                    return Ok(());
                }
            }
            let refreshed = self.run_view(&view.id)?;
            let iteration = current_iteration(&refreshed)?.clone();
            if !self.seal_phase(
                &refreshed,
                &iteration,
                "verification",
                &binding,
                cancellation,
            )? {
                return Ok(());
            }
            let refreshed = self.run_view(&view.id)?;
            let iteration = current_iteration(&refreshed)?.clone();
            let settled =
                super::loop_scope_evidence::latest_passed_manifest(&iteration, "verification")
                    .map(|(_, digest)| digest);
            if settled.as_deref() == Some(input_digest.as_str()) {
                break;
            }
            if round + 1 == MAX_VERIFICATION_ROUNDS {
                self.pause_for_scope(
                    &view.id,
                    LoopRunStatus::Running,
                    LoopTerminalReason::ScopeUnverifiable,
                    "Verification commands kept changing the tree; the checks could not be bound to a stable result.",
                    None,
                )?;
                return Ok(());
            }
            self.ports.observer.record(
                &LoopOperationContext {
                    run_id: view.id.clone(),
                    iteration_id: Some(iteration.id.clone()),
                    kind: LoopOperationKind::Verification,
                },
                None,
                AgentLogLevel::Warn,
                "Verification commands changed the tree; re-running the checks against the new manifest.",
            )?;
        }
        let mut run = self.run(&view.id)?;
        let snapshot = self.snapshot(&view.id)?;
        run.record_runtime_outcome(false, &snapshot.values().limits)?;
        run.move_to(LoopRunPhase::Deciding)?;
        self.save_run(&run, LoopRunStatus::Running, None)
    }

    /// Records phase evidence and applies the sticky consequences. Returns whether execution may
    /// continue into the next phase.
    pub(super) fn seal_phase(
        &self,
        view: &LoopRunView,
        iteration: &LoopIterationView,
        phase: &str,
        binding: &LoopScopeBinding,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let operation = self.ports.observer.start(
            LoopOperationContext {
                run_id: view.id.clone(),
                iteration_id: Some(iteration.id.clone()),
                kind: LoopOperationKind::ScopeEvidence,
            },
            &format!("Capturing scope evidence after the {phase} phase"),
        )?;
        let worktree_path = required(&view.worktree_path, "Loop worktree path")?;
        let outcome = seal_phase_evidence(
            &self.ports.scope_platform,
            &self.ports.iterations,
            &self.ports.clock,
            binding,
            &view.id,
            iteration,
            &worktree_path,
            phase,
            Some(&operation.id),
            cancellation,
        )?;
        match outcome {
            PhaseEvidenceOutcome::Passed { changes, .. } => {
                self.ports.observer.complete(
                    &operation,
                    &format!("Scope evidence complete: {changes} change(s), all within scope."),
                )?;
                Ok(true)
            }
            PhaseEvidenceOutcome::Violation { violations } => {
                let _ = self.ports.observer.fail(
                    &operation,
                    &format!("{} protected or out-of-scope change(s).", violations.len()),
                );
                let mut run = self.run(&view.id)?;
                let expected = run.status();
                run.fail(LoopTerminalReason::ScopeViolation)?;
                let now = self.ports.clock.now();
                self.ports.iterations.complete_iteration(
                    &view.id,
                    &iteration.id,
                    LoopRunStatus::Failed,
                    &format!(
                        "scope-violation: {}",
                        violations.first().cloned().unwrap_or_default()
                    ),
                    &now,
                )?;
                self.save_run(&run, expected, Some(&now))?;
                self.ports
                    .observer
                    .signal_canonical_loop(&view.id, CanonicalLoopSignal::Failed)?;
                Ok(false)
            }
            PhaseEvidenceOutcome::Unverifiable { reason } => {
                let _ = self.ports.observer.fail(&operation, &reason);
                self.pause_for_scope(
                    &view.id,
                    LoopRunStatus::Running,
                    LoopTerminalReason::ScopeUnverifiable,
                    &reason,
                    None,
                )?;
                Ok(false)
            }
        }
    }

    pub(super) fn pause_for_scope(
        &self,
        run_id: &str,
        expected_status: LoopRunStatus,
        reason: LoopTerminalReason,
        detail: &str,
        operation: Option<&ActiveLoopOperation>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut run = self.run(run_id)?;
        if run.status() != expected_status {
            return Err(AgentRuntimeApplicationError::Loop(
                "Loop run changed before it could be paused for scope.".to_string(),
            ));
        }
        // A role session that is still generating could keep mutating after its authority is
        // gone; it is stopped first so the pause is a real boundary, not a bookkeeping note.
        if let Ok(view) = self.run_view(run_id) {
            self.stop_owned_roles(&view);
        }
        run.pause_for_scope(reason)?;
        self.save_run(&run, expected_status, None)?;
        let context = LoopOperationContext {
            run_id: run_id.to_string(),
            iteration_id: None,
            kind: LoopOperationKind::ScopeEvidence,
        };
        self.ports.observer.record(
            &context,
            operation.map(|operation| operation.id.as_str()),
            AgentLogLevel::Warn,
            &format!("Loop paused ({}): {detail}", reason.as_str()),
        )?;
        if let Some(operation) = operation {
            let _ = self.ports.observer.fail(operation, detail);
        }
        self.ports
            .observer
            .signal_canonical_loop(run_id, CanonicalLoopSignal::Paused)
    }

    pub(super) fn run_view(&self, id: &str) -> Result<LoopRunView, AgentRuntimeApplicationError> {
        self.ports
            .loops
            .find_run_view(id)?
            .ok_or_else(|| missing("Loop run"))
    }
    pub(super) fn run(
        &self,
        id: &str,
    ) -> Result<crate::contexts::agent_runtime::domain::LoopRun, AgentRuntimeApplicationError> {
        self.ports
            .loops
            .find_run(id)?
            .ok_or_else(|| missing("Loop run"))
    }
    pub(super) fn snapshot(
        &self,
        id: &str,
    ) -> Result<crate::contexts::agent_runtime::domain::LoopDefinition, AgentRuntimeApplicationError>
    {
        self.ports
            .loops
            .find_run_definition_snapshot(id)?
            .ok_or_else(|| missing("Loop definition snapshot"))
    }
    pub(super) fn save_run(
        &self,
        run: &crate::contexts::agent_runtime::domain::LoopRun,
        expected: LoopRunStatus,
        completed: Option<&str>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.ports
            .loops
            .save_run_transition(run, expected, &self.ports.clock.now(), completed)
    }
}

pub(super) fn scope_ref(binding: &LoopScopeBinding, role: LoopGuardRole) -> LoopScopeRef {
    LoopScopeRef {
        scope_digest: binding.scope_digest.clone(),
        requested_mode: binding.mode().unwrap_or(
            crate::contexts::agent_runtime::domain::LoopRequestedMode::PreventiveRequired,
        ),
        role,
    }
}

/// Phase name of the manifest verification commands run against.
pub(super) const VERIFICATION_INPUT_PHASE: &str = "verification-input";
/// How many times a verification pass may rewrite the tree before the run pauses.
pub(super) const MAX_VERIFICATION_ROUNDS: usize = 2;
/// How many verification passes one iteration may take, counting re-verification after the
/// Verifier phase found a changed tree, before the run pauses as unverifiable.
pub(super) const MAX_VERIFICATION_PASSES: usize = 6;

pub(super) fn current_iteration(
    view: &LoopRunView,
) -> Result<&super::LoopIterationView, AgentRuntimeApplicationError> {
    view.iterations
        .iter()
        .find(|item| item.sequence == view.current_iteration)
        .ok_or_else(|| missing("Current Loop iteration"))
}
pub(super) fn required(
    value: &Option<String>,
    label: &str,
) -> Result<String, AgentRuntimeApplicationError> {
    value.clone().ok_or_else(|| missing(label))
}
pub(super) fn elapsed_seconds(created_at: &str) -> u64 {
    chrono::DateTime::parse_from_rfc3339(created_at)
        .ok()
        .and_then(|created| {
            u64::try_from(
                chrono::Utc::now()
                    .timestamp()
                    .saturating_sub(created.timestamp()),
            )
            .ok()
        })
        .unwrap_or(0)
}
pub(super) fn missing(label: &str) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Loop(format!("{label} is unavailable."))
}
