use super::*;
use crate::contexts::agent_runtime::domain::{
    assess_revision_progress, LoopLimits, LoopRun, LoopRunPhase,
};
use serde_json::json;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct ProgressRepository {
    saved: Mutex<Vec<(String, String, String, String)>>,
}

impl LoopIterationRepository for ProgressRepository {
    fn insert_iteration(&self, _: &LoopIterationView) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn attach_worker_session(&self, _: &str, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn attach_verifier_session(
        &self,
        _: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn save_verifier_result(
        &self,
        _: &SaveLoopVerifierResultRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn save_iteration_fingerprints(
        &self,
        run_id: &str,
        iteration_id: &str,
        diff_fingerprint: &str,
        check_failure_fingerprint: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.saved.lock().expect("saved").push((
            run_id.to_string(),
            iteration_id.to_string(),
            diff_fingerprint.to_string(),
            check_failure_fingerprint.to_string(),
        ));
        Ok(())
    }

    fn append_evidence(&self, _: &LoopEvidenceView) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
}

fn evidence(id: &str, status: &str, required: bool) -> LoopEvidenceView {
    LoopEvidenceView {
        id: format!("evidence-{id}"),
        run_id: "run-1".to_string(),
        iteration_id: Some("iteration-1".to_string()),
        kind: "verification-command".to_string(),
        status: status.to_string(),
        summary: format!("{id} {status}"),
        operation_id: Some(format!("operation-{id}")),
        command_id: Some(id.to_string()),
        exit_code: (status == "failed").then_some(1),
        duration_ms: Some(10),
        details: Some(json!({ "required": required })),
        created_at: "2026-07-22T10:00:00Z".to_string(),
    }
}

#[test]
fn optional_checks_do_not_change_required_failure_fingerprint() {
    let baseline = fingerprint_loop_iteration("diff", &[evidence("required", "failed", true)])
        .expect("baseline");
    let with_optional = fingerprint_loop_iteration(
        "diff",
        &[
            evidence("required", "failed", true),
            evidence("optional", "timed-out", false),
        ],
    )
    .expect("optional");

    assert_eq!(baseline, with_optional);
}

#[test]
fn newly_passing_required_evidence_is_objective_progress() {
    let previous =
        fingerprint_loop_iteration("diff", &[evidence("lint", "failed", true)]).expect("previous");
    let current = fingerprint_loop_iteration(
        "diff",
        &[
            evidence("lint", "failed", true),
            evidence("test", "passed", true),
        ],
    )
    .expect("current");

    let progress = assess_revision_progress(Some(&previous), &current);
    assert!(progress.progressed);
    assert!(progress.has_new_passing_required_evidence);
}

#[test]
fn malformed_required_evidence_is_rejected() {
    let mut missing_command = evidence("lint", "failed", true);
    missing_command.command_id = None;
    assert!(fingerprint_loop_iteration("diff", &[missing_command]).is_err());

    let unknown = evidence("lint", "unknown", true);
    assert!(fingerprint_loop_iteration("diff", &[unknown]).is_err());
}

#[test]
fn progress_service_persists_fingerprints_before_updating_run_counter() {
    let repository = Arc::new(ProgressRepository::default());
    let service = LoopProgressApplicationService::new(repository.clone());
    let limits = LoopLimits::new(3, 60, 600, 2, 2).expect("limits");
    let mut run = LoopRun::new("run-1".to_string(), "loop-1".to_string()).expect("run");
    run.begin().expect("begin");
    run.move_to(LoopRunPhase::Verifying).expect("verify");
    run.move_to(LoopRunPhase::Deciding).expect("decide");
    let previous = fingerprint_loop_iteration("same diff", &[evidence("lint", "failed", true)])
        .expect("previous");

    let recorded = service
        .record_revision(
            &mut run,
            &limits,
            RecordLoopRevisionProgressRequest {
                run_id: "run-1".to_string(),
                iteration_id: "iteration-1".to_string(),
                diff: "same diff".to_string(),
                evidence: vec![evidence("lint", "failed", true)],
                previous: Some(previous),
            },
        )
        .expect("record progress");

    assert!(!recorded.assessment.progressed);
    assert!(!recorded.no_progress_limit_reached);
    assert_eq!(run.consecutive_no_progress(), 1);
    let saved = repository.saved.lock().expect("saved");
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].0, "run-1");
    assert_eq!(saved[0].1, "iteration-1");
    assert_eq!(saved[0].2, recorded.fingerprints.diff);
}

#[test]
fn progress_service_rejects_cross_iteration_evidence_before_persistence() {
    let repository = Arc::new(ProgressRepository::default());
    let service = LoopProgressApplicationService::new(repository.clone());
    let limits = LoopLimits::new(3, 60, 600, 2, 2).expect("limits");
    let mut run = LoopRun::new("run-1".to_string(), "loop-1".to_string()).expect("run");
    run.begin().expect("begin");
    run.move_to(LoopRunPhase::Verifying).expect("verify");
    run.move_to(LoopRunPhase::Deciding).expect("decide");
    let mut foreign = evidence("lint", "failed", true);
    foreign.iteration_id = Some("iteration-2".to_string());

    let result = service.record_revision(
        &mut run,
        &limits,
        RecordLoopRevisionProgressRequest {
            run_id: "run-1".to_string(),
            iteration_id: "iteration-1".to_string(),
            diff: "diff".to_string(),
            evidence: vec![foreign],
            previous: None,
        },
    );

    assert!(result.is_err());
    assert!(repository.saved.lock().expect("saved").is_empty());
    assert_eq!(run.consecutive_no_progress(), 0);
}
