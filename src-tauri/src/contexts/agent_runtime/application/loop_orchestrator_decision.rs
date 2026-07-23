use super::loop_orchestrator::{current_iteration, missing, required};
use super::{
    AgentRuntimeApplicationError, LoopOperationContext, LoopOperationKind,
    LoopOrchestratorApplicationService, LoopRunView, LoopVerificationCancellation,
    RecordLoopRevisionProgressRequest, StartLoopVerifierRequest,
};
use crate::contexts::agent_runtime::domain::{
    decide_loop_iteration, LoopDecision, LoopDecisionInput, LoopDecisionOutcome, LoopLimits,
    LoopObjectiveFingerprints, LoopRun, LoopRunStatus, LoopVerifierRecommendation,
};

impl LoopOrchestratorApplicationService {
    pub(super) fn decide(
        &self,
        view: &LoopRunView,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let iteration = current_iteration(view)?;
        if iteration.verifier_recommendation.is_none() {
            let started = self.ports.verifier.start(StartLoopVerifierRequest {
                run_id: view.id.clone(),
                iteration_id: iteration.id.clone(),
                definition_snapshot: view.definition_snapshot.clone(),
                project_path: view.project_path.clone(),
                worktree_path: required(&view.worktree_path, "Loop worktree path")?,
                worktree_name: required(&view.worktree_name, "Loop worktree name")?,
                worktree_branch: required(&view.worktree_branch, "Loop worktree branch")?,
                check_evidence: iteration.evidence.clone(),
            })?;
            let operation = match self.ports.observer.start(
                LoopOperationContext {
                    run_id: view.id.clone(),
                    iteration_id: Some(iteration.id.clone()),
                    kind: LoopOperationKind::RoleGeneration,
                },
                "Running the Loop Verifier role",
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
                self.ports.verifier.complete(terminal)
            })();
            match result {
                Ok(_) => self
                    .ports
                    .observer
                    .complete(&operation, "The Loop Verifier role completed.")?,
                Err(error) => {
                    self.finish_role_error(&operation, &error, cancellation);
                    return Err(error);
                }
            }
        }

        // Verifier completion persists ownership and findings. Decisions and fingerprints must
        // use that durable boundary instead of the pre-generation projection.
        let refreshed_view = self.run_view(&view.id)?;
        let iteration = current_iteration(&refreshed_view)?;
        let recommendation = iteration
            .verifier_recommendation
            .as_deref()
            .map(recommendation)
            .transpose()?
            .ok_or_else(|| missing("Verifier recommendation"))?;
        let checks_passed = iteration
            .evidence
            .iter()
            .filter(|item| {
                item.kind == "verification-command"
                    && item
                        .details
                        .as_ref()
                        .and_then(|value| value.get("required"))
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false)
            })
            .all(|item| item.status == "passed");
        let decision = decide_loop_iteration(&LoopDecisionInput {
            required_checks_passed: checks_passed,
            verifier_recommendation: recommendation,
            user_feedback: iteration.user_feedback.clone(),
            hard_terminal_reason: None,
        });
        let operation = self.ports.observer.start(
            LoopOperationContext {
                run_id: refreshed_view.id.clone(),
                iteration_id: Some(iteration.id.clone()),
                kind: LoopOperationKind::Decision,
            },
            "Applying native Loop decision policy",
        )?;
        match self.apply_decision(&refreshed_view, iteration, decision, checks_passed) {
            Ok(()) => self
                .ports
                .observer
                .complete(&operation, "The native Loop decision was persisted."),
            Err(error) => {
                let _ = self.ports.observer.fail(&operation, &error.to_string());
                Err(error)
            }
        }
    }

    fn apply_decision(
        &self,
        view: &LoopRunView,
        iteration: &super::LoopIterationView,
        decision: LoopDecision,
        checks_passed: bool,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut run = self.run(&view.id)?;
        let snapshot = self.snapshot(&view.id)?;
        let limits = &snapshot.values().limits;
        run.record_runtime_outcome(false, limits)?;
        let now = self.ports.clock.now();
        let (iteration_status, completed) = match decision.outcome {
            LoopDecisionOutcome::AwaitingAcceptance => {
                run.await_acceptance(checks_passed)?;
                (LoopRunStatus::AwaitingAcceptance, false)
            }
            LoopDecisionOutcome::Failed(reason) => {
                run.fail(reason)?;
                (LoopRunStatus::Failed, true)
            }
            LoopDecisionOutcome::Cancelled(reason) => {
                run.cancel(reason)?;
                (LoopRunStatus::Cancelled, true)
            }
            LoopDecisionOutcome::NextIteration => {
                self.record_progress(view, iteration, &mut run, limits)?;
                if run.status() != LoopRunStatus::Failed {
                    run.try_begin_revision(limits)?;
                }
                (LoopRunStatus::Failed, run.status() == LoopRunStatus::Failed)
            }
        };
        self.ports.iterations.complete_iteration(
            &view.id,
            &iteration.id,
            iteration_status,
            &decision.reason,
            &now,
        )?;
        self.save_run(
            &run,
            LoopRunStatus::Running,
            completed.then_some(now.as_str()),
        )
    }

    fn record_progress(
        &self,
        view: &LoopRunView,
        iteration: &super::LoopIterationView,
        run: &mut LoopRun,
        limits: &LoopLimits,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let session_id = required(&iteration.verifier_session_id, "Verifier session")?;
        let previous = view
            .iterations
            .iter()
            .rev()
            .find(|item| item.sequence < iteration.sequence)
            .and_then(|item| {
                Some(LoopObjectiveFingerprints::rehydrate(
                    item.diff_fingerprint.clone()?,
                    item.check_failure_fingerprint.clone()?,
                ))
            });
        self.ports.progress.record_revision(
            run,
            limits,
            RecordLoopRevisionProgressRequest {
                run_id: view.id.clone(),
                iteration_id: iteration.id.clone(),
                diff: self.ports.verifier_context.bounded_diff(&session_id)?,
                evidence: iteration.evidence.clone(),
                previous,
            },
        )?;
        Ok(())
    }
}

fn recommendation(value: &str) -> Result<LoopVerifierRecommendation, AgentRuntimeApplicationError> {
    match value {
        "pass" => Ok(LoopVerifierRecommendation::Pass),
        "revise" => Ok(LoopVerifierRecommendation::Revise),
        "blocked" => Ok(LoopVerifierRecommendation::Blocked),
        _ => Err(missing("Verifier recommendation")),
    }
}
