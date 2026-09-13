use super::loop_scope_evidence::{scope_evidence_state, ScopeEvidenceState};
use super::{
    AgentClockPort, AgentRuntimeApplicationError, CanonicalLoopSignal, ContinueLoopRequest,
    LoopApplicationService, LoopAuditConsumption, LoopControlAction, LoopControlEnvelope,
    LoopExecutionControlPort, LoopOperationContext, LoopOperationKind, LoopOperationObserver,
    LoopRepository, LoopRunScopeRecord, LoopRunView, LoopScopeBinding,
};
use crate::contexts::agent_runtime::domain::{
    LoopRequestedMode, LoopRun, LoopRunPhase, LoopRunStatus, LoopTerminalReason,
};
use std::sync::Arc;

const MAX_FEEDBACK_BYTES: usize = 16 * 1024;

#[derive(Clone)]
pub(crate) struct LoopControlApplicationPorts {
    pub(crate) loops: Arc<dyn LoopRepository>,
    pub(crate) execution: Arc<dyn LoopExecutionControlPort>,
    pub(crate) observer: LoopOperationObserver,
    pub(crate) clock: Arc<dyn AgentClockPort>,
    /// Shared with start so resume and continuation re-prove capability and consume audit
    /// receipts through the same checks.
    pub(crate) admission: LoopApplicationService,
}

#[derive(Clone)]
pub(crate) struct LoopControlApplicationService {
    ports: LoopControlApplicationPorts,
}

/// Everything resume/continue must re-establish before a blocker is cleared.
struct ControlGate {
    binding: LoopScopeBinding,
    record: LoopRunScopeRecord,
    audit: Option<LoopAuditConsumption>,
}

impl LoopControlApplicationService {
    pub(crate) fn new(ports: LoopControlApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn request_pause(
        &self,
        run_id: &str,
    ) -> Result<LoopRun, AgentRuntimeApplicationError> {
        let mut run = self.find_run(run_id)?;
        let expected_status = run.status();
        let expected_pause_requested = run.pause_requested();
        run.request_pause()?;
        self.ports.loops.save_pause_request(
            &run,
            expected_status,
            expected_pause_requested,
            &self.ports.clock.now(),
        )?;
        Ok(run)
    }

    #[cfg(test)]
    pub(crate) fn pause_at_boundary(
        &self,
        run_id: &str,
    ) -> Result<LoopRun, AgentRuntimeApplicationError> {
        let mut run = self.find_run(run_id)?;
        let expected_status = run.status();
        run.pause_at_boundary()?;
        self.save_transition(&run, expected_status, None)?;
        Ok(run)
    }

    pub(crate) fn resume(
        &self,
        run_id: &str,
        envelope: LoopControlEnvelope,
    ) -> Result<LoopRun, AgentRuntimeApplicationError> {
        let mut run = self.find_run(run_id)?;
        let expected_status = run.status();
        if expected_status != LoopRunStatus::Paused {
            run.resume()?;
        }
        let gate = self.gate(&run, LoopControlAction::Resume, &envelope)?;
        // A scope blocker is cleared only after it has been re-proved; a run that still cannot
        // establish its authority stays paused with the same actionable reason.
        match run.terminal_reason() {
            Some(LoopTerminalReason::ScopeBindingMissing) => {
                return Err(validation(
                    "scope-binding-missing: this run has no trustworthy scope binding and cannot resume; cancel it and start a new run from the confirmed definition.",
                ))
            }
            Some(LoopTerminalReason::ScopeUnverifiable) if run.phase() == LoopRunPhase::Finalizing => {
                self.require_stable_finalizing_tree(&run, &gate)?;
            }
            _ => {}
        }
        run.resume()?;
        self.ports.loops.save_run_transition_with_audit(
            &run,
            expected_status,
            &self.ports.clock.now(),
            None,
            gate.audit.as_ref(),
        )?;
        self.ports
            .observer
            .signal_canonical_loop(run_id, CanonicalLoopSignal::Resumed)?;
        Ok(run)
    }

    pub(crate) fn cancel(&self, run_id: &str) -> Result<LoopRun, AgentRuntimeApplicationError> {
        let mut run = self.find_run(run_id)?;
        let expected_status = run.status();
        run.cancel(LoopTerminalReason::UserStopped)?;
        let operation = self.ports.observer.start(
            LoopOperationContext {
                run_id: run_id.to_string(),
                iteration_id: None,
                kind: LoopOperationKind::Cancellation,
            },
            "Requesting immediate Loop cancellation",
        )?;
        let completed_at = self.ports.clock.now();
        if let Err(error) = self.ports.loops.save_run_transition(
            &run,
            expected_status,
            &completed_at,
            Some(&completed_at),
        ) {
            let _ = self.ports.observer.fail(&operation, &error.to_string());
            return Err(error);
        }
        if let Err(error) = self.ports.execution.request_cancellation(run_id) {
            let _ = self.ports.observer.fail(&operation, &error.to_string());
            return Err(error);
        }
        self.ports
            .observer
            .complete(&operation, "Loop cancellation was requested.")?;
        self.ports
            .observer
            .signal_canonical_loop(run_id, CanonicalLoopSignal::Cancelled)?;
        Ok(run)
    }

    pub(crate) fn continue_with_feedback(
        &self,
        request: ContinueLoopRequest,
    ) -> Result<LoopRun, AgentRuntimeApplicationError> {
        let feedback = request.feedback.trim();
        if feedback.is_empty() {
            return Err(validation("Continuation feedback is required."));
        }
        if feedback.len() > MAX_FEEDBACK_BYTES {
            return Err(validation("Continuation feedback is too large."));
        }

        let mut run = self.find_run(&request.run_id)?;
        let snapshot = self
            .ports
            .loops
            .find_run_definition_snapshot(&request.run_id)?
            .ok_or_else(|| loop_error("Loop definition snapshot not found."))?;
        let expected_status = run.status();
        run.continue_iteration(&snapshot.values().limits)?;
        let gate = self.gate(&run, LoopControlAction::Continue, &request.envelope)?;
        self.ports.loops.save_continue_transition_with_audit(
            &run,
            expected_status,
            feedback,
            &self.ports.clock.now(),
            gate.audit.as_ref(),
        )?;
        Ok(run)
    }

    pub(crate) fn reject(&self, run_id: &str) -> Result<LoopRun, AgentRuntimeApplicationError> {
        let mut run = self.find_run(run_id)?;
        let expected_status = run.status();
        run.cancel(LoopTerminalReason::UserRejected)?;
        let completed_at = self.ports.clock.now();
        self.ports.loops.save_run_transition(
            &run,
            expected_status,
            &completed_at,
            Some(&completed_at),
        )?;
        self.ports
            .observer
            .signal_canonical_loop(run_id, CanonicalLoopSignal::Cancelled)?;
        Ok(run)
    }

    /// Re-proves binding, root identity, capability and the audit acknowledgement before a
    /// resume or continuation may schedule work. Legacy runs without a binding are refused.
    fn gate(
        &self,
        run: &LoopRun,
        action: LoopControlAction,
        envelope: &LoopControlEnvelope,
    ) -> Result<ControlGate, AgentRuntimeApplicationError> {
        let record = self
            .ports
            .loops
            .find_run_scope(run.id())?
            .ok_or_else(|| loop_error("Loop run not found."))?;
        if let Some(expected) = envelope.expected_revision {
            if expected != record.revision {
                return Err(validation(
                    "The run changed since it was loaded; refresh and review the current state.",
                ));
            }
        }
        let binding = record.binding.clone().ok_or_else(|| {
            validation(
                "scope-binding-missing: this run has no trustworthy scope binding; cancel it and start a new run from the confirmed definition.",
            )
        })?;
        let snapshot = self
            .ports
            .loops
            .find_run_definition_snapshot(run.id())?
            .ok_or_else(|| loop_error("Loop definition snapshot not found."))?;
        let assessment = self.ports.admission.assessment().assess(&snapshot)?;
        if assessment.witness_digest != binding.witness_digest
            || !assessment.satisfies_requested_mode
        {
            return Err(validation(&format!(
                "scope-capability-changed: execution capability no longer satisfies the frozen {} mode: {}",
                binding.requested_mode,
                assessment.blockers.join("; ")
            )));
        }
        let audit = if binding.mode() == Some(LoopRequestedMode::ArtifactAudited) {
            Some(self.ports.admission.audit_consumption(
                envelope,
                action,
                run.id(),
                record.revision,
                &binding.scope_digest,
                &assessment,
            )?)
        } else {
            None
        };
        Ok(ControlGate {
            binding,
            record,
            audit,
        })
    }

    /// A run paused as unverifiable while awaiting acceptance may only resume when a fresh
    /// complete scan again matches the Verifier-phase evidence.
    fn require_stable_finalizing_tree(
        &self,
        run: &LoopRun,
        gate: &ControlGate,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let view = self
            .ports
            .loops
            .find_run_view(run.id())?
            .ok_or_else(|| loop_error("Loop run not found."))?;
        let iteration = current_iteration(&view)?;
        let ScopeEvidenceState::Complete {
            verifier_manifest_digest,
        } = scope_evidence_state(iteration)
        else {
            return Err(validation(
                "scope-unverifiable: this iteration has no complete scope evidence; cancel the run or start a new one.",
            ));
        };
        let worktree_path = view
            .worktree_path
            .clone()
            .ok_or_else(|| loop_error("Loop worktree path is unavailable."))?;
        let platform = self.ports.admission.assessment().platform();
        platform
            .verify_root(&worktree_path, &gate.binding.root)
            .map_err(|failure| validation(&format!("{}: {}", failure.code, failure.message)))?;
        let fresh = platform
            .capture_manifest(
                run.id(),
                &worktree_path,
                &format!("resume-check-{}", uuid::Uuid::new_v4()),
                &super::LoopVerificationCancellation::default(),
            )
            .map_err(|failure| validation(&format!("{}: {}", failure.code, failure.message)))?;
        if fresh.digest != verifier_manifest_digest {
            return Err(validation(
                "scope-unverifiable: the worktree still differs from the verified evidence; restore it or start a new run.",
            ));
        }
        let _ = &gate.record;
        Ok(())
    }

    fn find_run(&self, run_id: &str) -> Result<LoopRun, AgentRuntimeApplicationError> {
        self.ports
            .loops
            .find_run(run_id)?
            .ok_or_else(|| loop_error("Loop run not found."))
    }

    #[cfg(test)]
    fn save_transition(
        &self,
        run: &LoopRun,
        expected_status: LoopRunStatus,
        completed_at: Option<&str>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.ports.loops.save_run_transition(
            run,
            expected_status,
            &self.ports.clock.now(),
            completed_at,
        )
    }
}

fn current_iteration(
    view: &LoopRunView,
) -> Result<&super::LoopIterationView, AgentRuntimeApplicationError> {
    view.iterations
        .iter()
        .find(|item| item.sequence == view.current_iteration)
        .ok_or_else(|| loop_error("Current Loop iteration is unavailable."))
}

pub(crate) fn validation(message: &str) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Validation(message.to_string())
}

pub(crate) fn loop_error(message: &str) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Loop(message.to_string())
}
