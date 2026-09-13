//! The sealed-evidence acceptance gate every accept entry point routes through.
//!
//! Acceptance is an asynchronous native operation: it takes the run's write ownership, stops the
//! owned role sessions, copies the actual contents of the tree into immutable objects, checks that
//! the sealed manifest is the one the Verifier phase validated, and only then commits success with
//! a compare-and-set on the run revision. A tree that changed since verification, an evidence
//! store that cannot be established, or a concurrent transition all leave the run unsucceeded.

use super::loop_scope_evidence::{scope_evidence_state, ScopeEvidenceState};
use super::{
    AgentClockPort, AgentLogLevel, AgentRuntimeApplicationError, CanonicalLoopSignal,
    LoopAcceptanceResultView, LoopBackgroundPort, LoopControlOperationClaim, LoopEvidenceView,
    LoopGenerationControlPort, LoopIterationRepository, LoopOperationContext, LoopOperationKind,
    LoopOperationObserver, LoopRepository, LoopRunView, LoopScopePlatformPort,
    LoopVerificationCancellation, RequestLoopAcceptanceRequest,
};
use crate::contexts::agent_runtime::domain::{LoopRunStatus, LoopTerminalReason};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct LoopAcceptanceApplicationPorts {
    pub(crate) loops: Arc<dyn LoopRepository>,
    pub(crate) iterations: Arc<dyn LoopIterationRepository>,
    pub(crate) generations: Arc<dyn LoopGenerationControlPort>,
    pub(crate) scope_platform: Arc<dyn LoopScopePlatformPort>,
    pub(crate) background: Arc<dyn LoopBackgroundPort>,
    pub(crate) observer: LoopOperationObserver,
    pub(crate) clock: Arc<dyn AgentClockPort>,
}

#[derive(Clone)]
pub(crate) struct LoopAcceptanceApplicationService {
    ports: LoopAcceptanceApplicationPorts,
}

impl LoopAcceptanceApplicationService {
    pub(crate) fn new(ports: LoopAcceptanceApplicationPorts) -> Self {
        Self { ports }
    }

    /// Validates the caller's preconditions, claims the run for one operation and schedules the
    /// sealing work. Returns the operation the UI can watch.
    pub(crate) fn request(
        &self,
        request: RequestLoopAcceptanceRequest,
    ) -> Result<LoopAcceptanceResultView, AgentRuntimeApplicationError> {
        if let Some(key) = request.idempotency_key.as_deref() {
            match self.ports.loops.claim_control_operation(
                key,
                "accept",
                &request.run_id,
                &self.ports.clock.now(),
            )? {
                LoopControlOperationClaim::Existing(outcome) => {
                    return decode_outcome(&request.run_id, &outcome)
                }
                LoopControlOperationClaim::InFlight => {
                    return Err(validation(
                        "This acceptance request is already being processed.",
                    ))
                }
                LoopControlOperationClaim::New => {}
            }
        }
        let result = self.request_inner(&request);
        if let Some(key) = request.idempotency_key.as_deref() {
            match &result {
                Ok(result) => self.ports.loops.record_control_operation(
                    key,
                    &serde_json::json!({ "operationId": result.operation_id }).to_string(),
                )?,
                Err(_) => {
                    let _ = self.ports.loops.release_control_operation(key);
                }
            }
        }
        result
    }

    fn request_inner(
        &self,
        request: &RequestLoopAcceptanceRequest,
    ) -> Result<LoopAcceptanceResultView, AgentRuntimeApplicationError> {
        let view = self.view(&request.run_id)?;
        if view.status != LoopRunStatus::AwaitingAcceptance {
            return Err(validation(
                "Only a run awaiting acceptance can be accepted.",
            ));
        }
        let record = self
            .ports
            .loops
            .find_run_scope(&request.run_id)?
            .ok_or_else(|| loop_error("Loop run not found."))?;
        if let Some(expected) = request.expected_revision {
            if expected != record.revision {
                return Err(validation(
                    "The run changed since it was loaded; refresh and review the current evidence.",
                ));
            }
        }
        let binding = record.binding.clone().ok_or_else(|| {
            validation("scope-binding-missing: this run cannot be accepted without a trustworthy scope binding.")
        })?;
        if let Some(expected) = request.expected_scope_digest.as_deref() {
            if expected != binding.scope_digest {
                return Err(validation(
                    "The run scope differs from the one being accepted.",
                ));
            }
        }
        let iteration = current_iteration(&view)?;
        let ScopeEvidenceState::Complete {
            verifier_manifest_digest,
        } = scope_evidence_state(iteration)
        else {
            return Err(validation(
                "Acceptance requires complete, violation-free scope evidence for the current iteration.",
            ));
        };
        if let Some(expected) = request.expected_evidence_id.as_deref() {
            let latest = latest_verifier_evidence_id(iteration);
            if latest.as_deref() != Some(expected) {
                return Err(validation(
                    "The acceptance evidence changed since it was displayed; review the current evidence.",
                ));
            }
        }
        let operation = self.ports.observer.start(
            LoopOperationContext {
                run_id: view.id.clone(),
                iteration_id: Some(iteration.id.clone()),
                kind: LoopOperationKind::Acceptance,
            },
            "Sealing accepted artifacts",
        )?;
        if let Err(error) =
            self.ports
                .loops
                .attach_acceptance_operation(&view.id, &operation.id, record.revision)
        {
            let _ = self.ports.observer.fail(
                &operation,
                "Another acceptance or transition already owns this run.",
            );
            return Err(error);
        }
        let service = self.clone();
        let run_id = view.id.clone();
        let operation_id = operation.id.clone();
        let expected_revision = record.revision;
        let spawned = self.ports.background.spawn(
            &format!("loop-acceptance-{}", view.id),
            Box::new(move || {
                service.perform(
                    &run_id,
                    &operation_id,
                    expected_revision,
                    &verifier_manifest_digest,
                );
            }),
        );
        if let Err(error) = spawned {
            let _ = self
                .ports
                .loops
                .release_acceptance_operation(&view.id, &operation.id);
            let _ = self.ports.observer.fail(&operation, &error.to_string());
            return Err(error);
        }
        Ok(LoopAcceptanceResultView {
            run_id: view.id,
            operation_id: operation.id,
        })
    }

    /// Runs on the background port. Every failure path releases the operation and leaves an
    /// evidence row saying what could not be established.
    pub(crate) fn perform(
        &self,
        run_id: &str,
        operation_id: &str,
        expected_revision: u64,
        verifier_manifest_digest: &str,
    ) {
        let outcome = self.perform_inner(
            run_id,
            operation_id,
            expected_revision,
            verifier_manifest_digest,
        );
        let context = LoopOperationContext {
            run_id: run_id.to_string(),
            iteration_id: None,
            kind: LoopOperationKind::Acceptance,
        };
        let operation = super::ActiveLoopOperation {
            id: operation_id.to_string(),
            context: context.clone(),
        };
        match outcome {
            Ok(summary) => {
                let _ = self.ports.observer.complete(&operation, &summary);
            }
            Err(AcceptanceFailure::Unverifiable(reason)) => {
                let _ = self.record_evidence(run_id, "unverifiable", &reason, None);
                let _ = self
                    .ports
                    .loops
                    .release_acceptance_operation(run_id, operation_id);
                let _ = self.pause_unverifiable(run_id, &reason);
                let _ = self.ports.observer.fail(&operation, &reason);
            }
            Err(AcceptanceFailure::Conflict(reason)) => {
                let _ = self
                    .ports
                    .loops
                    .release_acceptance_operation(run_id, operation_id);
                let _ = self.ports.observer.record(
                    &context,
                    Some(operation_id),
                    AgentLogLevel::Warn,
                    &reason,
                );
                let _ = self.ports.observer.fail(&operation, &reason);
            }
            Err(AcceptanceFailure::Internal(error)) => {
                let _ = self
                    .ports
                    .loops
                    .release_acceptance_operation(run_id, operation_id);
                let _ = self.ports.observer.fail(&operation, &error.to_string());
            }
        }
    }

    fn perform_inner(
        &self,
        run_id: &str,
        operation_id: &str,
        expected_revision: u64,
        verifier_manifest_digest: &str,
    ) -> Result<String, AcceptanceFailure> {
        let view = self.view(run_id).map_err(AcceptanceFailure::Internal)?;
        let iteration = current_iteration(&view).map_err(AcceptanceFailure::Internal)?;
        // Reconcile owned writers before the tree is read: a role session that is still
        // producing output could otherwise change files between copy and validation.
        for session_id in [&iteration.worker_session_id, &iteration.verifier_session_id]
            .into_iter()
            .flatten()
        {
            let _ = self.ports.generations.stop_loop_generation(session_id);
        }
        let record = self
            .ports
            .loops
            .find_run_scope(run_id)
            .map_err(AcceptanceFailure::Internal)?
            .ok_or_else(|| AcceptanceFailure::Conflict("Loop run not found.".to_string()))?;
        if record.revision != expected_revision {
            return Err(AcceptanceFailure::Conflict(
                "The run changed while acceptance was being prepared.".to_string(),
            ));
        }
        let binding = record
            .binding
            .ok_or_else(|| AcceptanceFailure::Unverifiable("scope binding missing".to_string()))?;
        let worktree_path = view
            .worktree_path
            .clone()
            .ok_or_else(|| AcceptanceFailure::Unverifiable("worktree path missing".to_string()))?;
        self.ports
            .scope_platform
            .verify_root(&worktree_path, &binding.root)
            .map_err(|failure| {
                AcceptanceFailure::Unverifiable(format!("{}: {}", failure.code, failure.message))
            })?;
        let evidence_id = format!("sealed-{}", Uuid::new_v4());
        let receipt = self
            .ports
            .scope_platform
            .seal_contents(
                run_id,
                &worktree_path,
                &evidence_id,
                &LoopVerificationCancellation::default(),
            )
            .map_err(|failure| {
                AcceptanceFailure::Unverifiable(format!("{}: {}", failure.code, failure.message))
            })?;
        if receipt.digest != verifier_manifest_digest {
            return Err(AcceptanceFailure::Unverifiable(
                "scope-unverifiable: the sealed contents differ from the Verifier-phase evidence; the tree changed after verification.".to_string(),
            ));
        }
        let mut run = self
            .ports
            .loops
            .find_run(run_id)
            .map_err(AcceptanceFailure::Internal)?
            .ok_or_else(|| AcceptanceFailure::Conflict("Loop run not found.".to_string()))?;
        run.accept()
            .map_err(|error| AcceptanceFailure::Conflict(error.to_string()))?;
        let now = self.ports.clock.now();
        self.ports
            .loops
            .seal_acceptance(&run, expected_revision, operation_id, &evidence_id, &now)
            .map_err(|error| AcceptanceFailure::Conflict(error.to_string()))?;
        self.record_evidence(
            run_id,
            "passed",
            &format!(
                "Sealed {} object(s), {} byte(s); manifest {} matches the verified evidence.",
                receipt.copied_objects, receipt.copied_bytes, receipt.digest
            ),
            Some(serde_json::json!({
                "sealedEvidenceId": evidence_id,
                "manifestId": receipt.manifest_id,
                "manifestDigest": receipt.digest,
                "copiedObjects": receipt.copied_objects,
                "copiedBytes": receipt.copied_bytes,
                "scopeDigest": binding.scope_digest,
                "requestedMode": binding.requested_mode,
            })),
        )
        .map_err(AcceptanceFailure::Internal)?;
        self.ports
            .observer
            .signal_canonical_loop(run_id, CanonicalLoopSignal::Completed)
            .map_err(AcceptanceFailure::Internal)?;
        Ok(format!(
            "Accepted with sealed evidence {evidence_id} ({} object(s)).",
            receipt.copied_objects
        ))
    }

    fn pause_unverifiable(
        &self,
        run_id: &str,
        reason: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut run = self
            .ports
            .loops
            .find_run(run_id)?
            .ok_or_else(|| loop_error("Loop run not found."))?;
        let expected = run.status();
        if expected != LoopRunStatus::AwaitingAcceptance {
            return Ok(());
        }
        run.pause_for_scope(LoopTerminalReason::ScopeUnverifiable)?;
        self.ports
            .loops
            .save_run_transition(&run, expected, &self.ports.clock.now(), None)?;
        self.ports.observer.record(
            &LoopOperationContext {
                run_id: run_id.to_string(),
                iteration_id: None,
                kind: LoopOperationKind::Acceptance,
            },
            None,
            AgentLogLevel::Warn,
            &format!("Loop paused (scope-unverifiable): {reason}"),
        )?;
        self.ports
            .observer
            .signal_canonical_loop(run_id, CanonicalLoopSignal::Paused)
    }

    fn record_evidence(
        &self,
        run_id: &str,
        status: &str,
        summary: &str,
        details: Option<serde_json::Value>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let view = self.view(run_id)?;
        let iteration_id = current_iteration(&view).ok().map(|item| item.id.clone());
        self.ports.iterations.append_evidence(&LoopEvidenceView {
            id: format!("loop-evidence-{}", Uuid::new_v4()),
            run_id: run_id.to_string(),
            iteration_id,
            kind: "acceptance".to_string(),
            status: status.to_string(),
            summary: summary.to_string(),
            operation_id: None,
            command_id: None,
            exit_code: None,
            duration_ms: None,
            details,
            created_at: self.ports.clock.now(),
        })
    }

    fn view(&self, run_id: &str) -> Result<LoopRunView, AgentRuntimeApplicationError> {
        self.ports
            .loops
            .find_run_view(run_id)?
            .ok_or_else(|| loop_error("Loop run not found."))
    }
}

enum AcceptanceFailure {
    Unverifiable(String),
    Conflict(String),
    Internal(AgentRuntimeApplicationError),
}

fn latest_verifier_evidence_id(iteration: &super::LoopIterationView) -> Option<String> {
    iteration
        .evidence
        .iter()
        .rev()
        .find(|item| {
            item.kind == super::SCOPE_EVIDENCE_KIND
                && item.status == "passed"
                && item
                    .details
                    .as_ref()
                    .and_then(|details| details.get("phase"))
                    .and_then(serde_json::Value::as_str)
                    == Some("verifier")
        })
        .map(|item| item.id.clone())
}

fn decode_outcome(
    run_id: &str,
    outcome: &str,
) -> Result<LoopAcceptanceResultView, AgentRuntimeApplicationError> {
    let value: serde_json::Value = serde_json::from_str(outcome)
        .map_err(|_| validation("Recorded acceptance outcome is unreadable."))?;
    let operation_id = value
        .get("operationId")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| validation("Recorded acceptance outcome is incomplete."))?;
    Ok(LoopAcceptanceResultView {
        run_id: run_id.to_string(),
        operation_id: operation_id.to_string(),
    })
}

fn current_iteration(
    view: &LoopRunView,
) -> Result<&super::LoopIterationView, AgentRuntimeApplicationError> {
    view.iterations
        .iter()
        .find(|item| item.sequence == view.current_iteration)
        .ok_or_else(|| loop_error("Current Loop iteration is unavailable."))
}

fn validation(message: &str) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Validation(message.to_string())
}

fn loop_error(message: &str) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Loop(message.to_string())
}
