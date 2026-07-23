use super::{AgentRuntimeApplicationError, LoopEvidenceView, LoopIterationRepository};
use crate::contexts::agent_runtime::domain::{
    assess_revision_progress, fingerprint_objective_state, LoopCheckOutcome, LoopLimits,
    LoopObjectiveFingerprints, LoopRequiredCheckObservation, LoopRevisionProgress, LoopRun,
    LoopRunPhase, LoopRunStatus,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct RecordLoopRevisionProgressRequest {
    pub(crate) run_id: String,
    pub(crate) iteration_id: String,
    pub(crate) diff: String,
    pub(crate) evidence: Vec<LoopEvidenceView>,
    pub(crate) previous: Option<LoopObjectiveFingerprints>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordedLoopRevisionProgress {
    pub(crate) fingerprints: LoopObjectiveFingerprints,
    pub(crate) assessment: LoopRevisionProgress,
    pub(crate) no_progress_limit_reached: bool,
}

#[derive(Clone)]
pub(crate) struct LoopProgressApplicationService {
    iterations: Arc<dyn LoopIterationRepository>,
}

impl LoopProgressApplicationService {
    pub(crate) fn new(iterations: Arc<dyn LoopIterationRepository>) -> Self {
        Self { iterations }
    }

    pub(crate) fn record_revision(
        &self,
        run: &mut LoopRun,
        limits: &LoopLimits,
        request: RecordLoopRevisionProgressRequest,
    ) -> Result<RecordedLoopRevisionProgress, AgentRuntimeApplicationError> {
        validate_record_request(run, &request)?;
        let fingerprints = fingerprint_loop_iteration(&request.diff, &request.evidence)?;
        let assessment = assess_revision_progress(request.previous.as_ref(), &fingerprints);
        self.iterations.save_iteration_fingerprints(
            &request.run_id,
            &request.iteration_id,
            &fingerprints.diff,
            &fingerprints.required_check_failures,
        )?;
        let no_progress_limit_reached =
            run.record_revision_progress(assessment.progressed, limits)?;
        Ok(RecordedLoopRevisionProgress {
            fingerprints,
            assessment,
            no_progress_limit_reached,
        })
    }
}

pub(crate) fn fingerprint_loop_iteration(
    diff: &str,
    evidence: &[LoopEvidenceView],
) -> Result<LoopObjectiveFingerprints, AgentRuntimeApplicationError> {
    let checks = evidence
        .iter()
        .filter(|item| item.kind == "verification-command" && is_required(item))
        .map(required_observation)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(fingerprint_objective_state(diff, &checks))
}

fn validate_record_request(
    run: &LoopRun,
    request: &RecordLoopRevisionProgressRequest,
) -> Result<(), AgentRuntimeApplicationError> {
    if request.run_id != run.id()
        || request.iteration_id.trim().is_empty()
        || run.status() != LoopRunStatus::Running
        || run.phase() != LoopRunPhase::Deciding
    {
        return Err(validation(
            "Loop revision progress is not at a valid deciding boundary.",
        ));
    }
    if request.evidence.iter().any(|item| {
        item.run_id != request.run_id
            || item.iteration_id.as_deref() != Some(request.iteration_id.as_str())
    }) {
        return Err(validation(
            "Loop revision evidence belongs to another run or iteration.",
        ));
    }
    Ok(())
}

fn is_required(evidence: &LoopEvidenceView) -> bool {
    evidence
        .details
        .as_ref()
        .and_then(|details| details.get("required"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

fn required_observation(
    evidence: &LoopEvidenceView,
) -> Result<LoopRequiredCheckObservation, AgentRuntimeApplicationError> {
    let command_id = evidence
        .command_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| validation("Required verification evidence has no command id."))?;
    let outcome = match evidence.status.as_str() {
        "passed" => LoopCheckOutcome::Passed,
        "failed" => LoopCheckOutcome::Failed,
        "timed-out" => LoopCheckOutcome::TimedOut,
        "cancelled" => LoopCheckOutcome::Cancelled,
        "error" => LoopCheckOutcome::Error,
        _ => {
            return Err(validation(
                "Required verification evidence has an unknown status.",
            ))
        }
    };
    Ok(LoopRequiredCheckObservation {
        command_id: command_id.to_string(),
        outcome,
        exit_code: evidence.exit_code,
    })
}

fn validation(message: impl Into<String>) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Validation(message.into())
}
