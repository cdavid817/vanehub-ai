use super::{AttemptVerificationDispatch, VerificationEvidenceUpdate};
use crate::contexts::agent_runtime::api::{
    AgentRuntimeApi, AgentRuntimeApplicationError, GuardedValidationCancellation,
    GuardedValidationRequest, GuardedValidationResult, GuardedValidationStatus,
    LoopVerificationCommand,
};
use crate::contexts::task_orchestration::application::PlanApplicationError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttemptVerificationResult {
    pub(crate) evidence: Vec<VerificationEvidenceUpdate>,
    pub(crate) passed: bool,
    pub(crate) summary: String,
}

#[derive(Clone)]
pub(crate) struct OnePieceAttemptVerifier {
    guarded: Arc<dyn GuardedValidationGateway>,
    active: Arc<Mutex<HashMap<String, GuardedValidationCancellation>>>,
}

impl OnePieceAttemptVerifier {
    pub(crate) fn new(agents: AgentRuntimeApi) -> Self {
        Self {
            guarded: Arc::new(agents),
            active: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub(crate) fn cancel(&self, run_id: &str) -> bool {
        self.active
            .lock()
            .ok()
            .and_then(|active| active.get(run_id).cloned())
            .is_some_and(|cancellation| {
                cancellation.cancel();
                true
            })
    }

    pub(crate) fn verify(
        &self,
        dispatch: &AttemptVerificationDispatch,
    ) -> Result<AttemptVerificationResult, PlanApplicationError> {
        let guarded_commands = dispatch
            .commands
            .iter()
            .map(|command| {
                LoopVerificationCommand::new(
                    command.id.clone(),
                    command.program.clone(),
                    command.args.clone(),
                    command.working_directory.clone(),
                    command.timeout_seconds,
                    command.required,
                )
                .map_err(|error| PlanApplicationError::Validation(error.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let cancellation = GuardedValidationCancellation::default();
        self.active
            .lock()
            .map_err(|_| {
                PlanApplicationError::Storage(
                    "verification cancellation registry unavailable".to_string(),
                )
            })?
            .insert(dispatch.plan_run_id.clone(), cancellation.clone());
        let mut evidence = Vec::with_capacity(dispatch.commands.len());
        let mut required_passed = true;
        for (command, guarded) in dispatch.commands.iter().zip(guarded_commands) {
            match self.guarded.run_guarded_validation(
                GuardedValidationRequest {
                    worktree_root: dispatch.worktree_path.clone(),
                    command: guarded,
                },
                cancellation.clone(),
            ) {
                Ok(result) => {
                    if command.required && result.status != GuardedValidationStatus::Passed {
                        required_passed = false;
                    }
                    let output_summary = result.output_summary.map(|summary| {
                        if result.output_truncated {
                            format!("{summary}\n[output truncated]")
                        } else {
                            summary
                        }
                    });
                    evidence.push(VerificationEvidenceUpdate {
                        command_id: command.id.clone(),
                        status: result.status.as_str().to_string(),
                        exit_code: result.exit_code,
                        duration_ms: Some(result.duration_ms),
                        output_summary,
                    });
                }
                Err(_) => {
                    if command.required {
                        required_passed = false;
                    }
                    evidence.push(VerificationEvidenceUpdate {
                        command_id: command.id.clone(),
                        status: "execution_error".to_string(),
                        exit_code: None,
                        duration_ms: None,
                        output_summary: None,
                    });
                }
            }
        }
        self.active
            .lock()
            .map_err(|_| {
                PlanApplicationError::Storage(
                    "verification cancellation registry unavailable".to_string(),
                )
            })?
            .remove(&dispatch.plan_run_id);
        let passed_count = evidence
            .iter()
            .filter(|item| item.status == GuardedValidationStatus::Passed.as_str())
            .count();
        Ok(AttemptVerificationResult {
            summary: format!(
                "{passed_count}/{} validation commands passed; required checks {}.",
                evidence.len(),
                if required_passed { "passed" } else { "failed" }
            ),
            evidence,
            passed: required_passed,
        })
    }
}

trait GuardedValidationGateway: Send + Sync {
    fn run_guarded_validation(
        &self,
        request: GuardedValidationRequest,
        cancellation: GuardedValidationCancellation,
    ) -> Result<GuardedValidationResult, AgentRuntimeApplicationError>;
}

impl GuardedValidationGateway for AgentRuntimeApi {
    fn run_guarded_validation(
        &self,
        request: GuardedValidationRequest,
        cancellation: GuardedValidationCancellation,
    ) -> Result<GuardedValidationResult, AgentRuntimeApplicationError> {
        AgentRuntimeApi::run_guarded_validation_cancellable(self, request, cancellation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::task_orchestration::domain::VerificationCommand;
    use std::collections::VecDeque;
    use std::sync::{Barrier, Mutex};

    struct FakeGuardedValidation {
        results: Mutex<VecDeque<Result<GuardedValidationResult, AgentRuntimeApplicationError>>>,
    }

    struct WaitingGuardedValidation {
        started: Arc<Barrier>,
    }

    impl GuardedValidationGateway for WaitingGuardedValidation {
        fn run_guarded_validation(
            &self,
            _request: GuardedValidationRequest,
            cancellation: GuardedValidationCancellation,
        ) -> Result<GuardedValidationResult, AgentRuntimeApplicationError> {
            self.started.wait();
            while !cancellation.is_cancelled() {
                std::thread::yield_now();
            }
            Ok(result(GuardedValidationStatus::Cancelled))
        }
    }

    impl GuardedValidationGateway for FakeGuardedValidation {
        fn run_guarded_validation(
            &self,
            _request: GuardedValidationRequest,
            _cancellation: GuardedValidationCancellation,
        ) -> Result<GuardedValidationResult, AgentRuntimeApplicationError> {
            self.results
                .lock()
                .expect("results")
                .pop_front()
                .expect("prepared result")
        }
    }

    fn command(id: &str, required: bool) -> VerificationCommand {
        VerificationCommand {
            id: id.into(),
            program: "cargo".into(),
            args: vec!["test".into()],
            working_directory: None,
            timeout_seconds: 60,
            required,
        }
    }

    fn result(status: GuardedValidationStatus) -> GuardedValidationResult {
        GuardedValidationResult {
            status,
            exit_code: (status != GuardedValidationStatus::TimedOut).then_some(1),
            duration_ms: 25,
            output_summary: Some("bounded output".into()),
            output_truncated: false,
        }
    }

    #[test]
    fn multiple_results_preserve_fixed_execution_errors_and_required_failure() {
        let verifier = OnePieceAttemptVerifier {
            guarded: Arc::new(FakeGuardedValidation {
                results: Mutex::new(VecDeque::from([
                    Ok(result(GuardedValidationStatus::Passed)),
                    Ok(result(GuardedValidationStatus::Failed)),
                    Err(AgentRuntimeApplicationError::VerificationProcess(
                        "sensitive process diagnostic".into(),
                    )),
                ])),
            }),
            active: Arc::new(Mutex::new(HashMap::new())),
        };
        let verification = verifier
            .verify(&AttemptVerificationDispatch {
                attempt_id: "attempt-1".into(),
                plan_run_id: "run-1".into(),
                subtask_run_id: "task-run-1".into(),
                worktree_path: "C:/worktree".into(),
                commands: vec![
                    command("pass", true),
                    command("optional-fail", false),
                    command("execution-error", true),
                ],
            })
            .expect("verification");

        assert!(!verification.passed);
        assert_eq!(
            verification
                .evidence
                .iter()
                .map(|item| item.status.as_str())
                .collect::<Vec<_>>(),
            ["passed", "failed", "execution_error"]
        );
        assert_eq!(verification.evidence[2].output_summary, None);
        assert!(!verification.summary.contains("sensitive"));
    }

    #[test]
    fn cancellation_reaches_the_active_guarded_process() {
        let started = Arc::new(Barrier::new(2));
        let verifier = OnePieceAttemptVerifier {
            guarded: Arc::new(WaitingGuardedValidation {
                started: started.clone(),
            }),
            active: Arc::new(Mutex::new(HashMap::new())),
        };
        let worker = verifier.clone();
        let handle = std::thread::spawn(move || {
            worker.verify(&AttemptVerificationDispatch {
                attempt_id: "attempt-1".into(),
                plan_run_id: "run-1".into(),
                subtask_run_id: "task-run-1".into(),
                worktree_path: "C:/worktree".into(),
                commands: vec![command("slow", true)],
            })
        });
        started.wait();
        assert!(verifier.cancel("run-1"));
        let verification = handle
            .join()
            .expect("verification thread")
            .expect("verification");
        assert!(!verification.passed);
        assert_eq!(verification.evidence[0].status, "cancelled");
        assert!(!verifier.cancel("run-1"));
    }
}
