use super::*;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct VerificationWorld {
    commands: Mutex<Vec<String>>,
    evidence: Mutex<Vec<LoopEvidenceView>>,
    operation_events: Mutex<Vec<String>>,
    logs: Mutex<Vec<LoopLog>>,
}

impl VerificationWorld {
    fn service(self: &Arc<Self>) -> LoopVerificationApplicationService {
        LoopVerificationApplicationService::new(LoopVerificationApplicationPorts {
            iterations: self.clone(),
            processes: self.clone(),
            observer: LoopOperationObserver::new(self.clone(), self.clone(), self.clone()),
            clock: self.clone(),
        })
    }
}

impl LoopIterationRepository for VerificationWorld {
    fn insert_iteration(&self, _: &LoopIterationView) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn attach_worker_session(&self, _: &str, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn attach_verifier_session(
        &self,
        _: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn save_verifier_result(
        &self,
        _: &SaveLoopVerifierResultRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn save_iteration_fingerprints(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn append_evidence(
        &self,
        evidence: &LoopEvidenceView,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.evidence
            .lock()
            .expect("evidence")
            .push(evidence.clone());
        Ok(())
    }
}

impl LoopVerificationProcessPort for VerificationWorld {
    fn execute(
        &self,
        request: LoopVerificationProcessRequest,
    ) -> Result<LoopVerificationProcessResult, AgentRuntimeApplicationError> {
        let command_id = request.command.id.clone();
        self.commands
            .lock()
            .expect("commands")
            .push(command_id.clone());
        if command_id == "process-error" {
            return Err(AgentRuntimeApplicationError::VerificationProcess(
                "verification process rejected command".to_string(),
            ));
        }
        let status = match command_id.as_str() {
            "required-timeout" => LoopVerificationProcessStatus::TimedOut,
            "optional-fail" => LoopVerificationProcessStatus::Failed,
            "cancelled" => LoopVerificationProcessStatus::Cancelled,
            _ => LoopVerificationProcessStatus::Passed,
        };
        Ok(LoopVerificationProcessResult {
            status,
            exit_code: (status != LoopVerificationProcessStatus::TimedOut).then_some(
                if status == LoopVerificationProcessStatus::Passed {
                    0
                } else {
                    1
                },
            ),
            duration_ms: 25,
            stdout: format!("output for {command_id}"),
            stderr: String::new(),
            output_truncated: false,
        })
    }
}

impl AgentTaskPort for VerificationWorld {
    fn start_agent_launch(
        &self,
        _: &str,
        _: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn start_agent_generation(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn start_loop_operation(
        &self,
        context: &LoopOperationContext,
        message: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        assert_eq!(context.kind, LoopOperationKind::Verification);
        let id = format!(
            "operation-{}",
            self.operation_events.lock().expect("events").len()
        );
        self.operation_events
            .lock()
            .expect("events")
            .push(format!("start:{message}"));
        Ok(AgentOperation {
            id,
            related_agent_id: Some(context.run_id.clone()),
            message: Some(message.to_string()),
        })
    }

    fn append_log(
        &self,
        operation_id: &str,
        _: String,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.operation_events
            .lock()
            .expect("events")
            .push(format!("log:{operation_id}"));
        Ok(())
    }

    fn complete(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.operation_events
            .lock()
            .expect("events")
            .push(format!("complete:{operation_id}"));
        Ok(())
    }

    fn fail(&self, operation_id: &str, _: String) -> Result<(), AgentRuntimeApplicationError> {
        self.operation_events
            .lock()
            .expect("events")
            .push(format!("fail:{operation_id}"));
        Ok(())
    }

    fn cancel(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.operation_events
            .lock()
            .expect("events")
            .push(format!("cancel:{operation_id}"));
        Ok(())
    }
}

impl AgentClockPort for VerificationWorld {
    fn now(&self) -> String {
        "2026-07-22T09:00:00Z".to_string()
    }
}

impl LoopLoggingPort for VerificationWorld {
    fn record_loop(&self, log: LoopLog) -> Result<(), AgentRuntimeApplicationError> {
        self.logs.lock().expect("logs").push(log);
        Ok(())
    }
}

fn command(id: &str, required: bool) -> LoopVerificationCommandView {
    LoopVerificationCommandView {
        id: id.to_string(),
        program: "npm".to_string(),
        args: vec!["test".to_string()],
        working_directory: None,
        timeout_seconds: 60,
        required,
    }
}

fn request(commands: Vec<LoopVerificationCommandView>) -> RunLoopVerificationRequest {
    RunLoopVerificationRequest {
        run_id: "run-1".to_string(),
        iteration_id: "iteration-1".to_string(),
        worktree_root: "C:/work/project-loop".to_string(),
        commands,
        cancellation: LoopVerificationCancellation::default(),
    }
}

#[test]
fn commands_run_in_definition_order_and_required_timeout_blocks_acceptance() {
    let world = Arc::new(VerificationWorld::default());
    let result = world
        .service()
        .run_commands(request(vec![
            command("required-pass", true),
            command("optional-fail", false),
            command("required-timeout", true),
        ]))
        .expect("verification");

    assert_eq!(
        *world.commands.lock().expect("commands"),
        ["required-pass", "optional-fail", "required-timeout"]
    );
    assert_eq!(result.evidence.len(), 3);
    assert!(!result.required_checks_passed);
    assert_eq!(result.evidence[2].status, "timed-out");
    assert_eq!(world.evidence.lock().expect("evidence").len(), 3);
}

#[test]
fn optional_failure_does_not_block_acceptance_readiness() {
    let world = Arc::new(VerificationWorld::default());
    let result = world
        .service()
        .run_commands(request(vec![
            command("required-pass", true),
            command("optional-fail", false),
        ]))
        .expect("verification");

    assert!(result.required_checks_passed);
    assert!(!result.cancelled);
}

#[test]
fn cancellation_records_evidence_and_stops_scheduling_commands() {
    let world = Arc::new(VerificationWorld::default());
    let result = world
        .service()
        .run_commands(request(vec![
            command("cancelled", true),
            command("never-runs", true),
        ]))
        .expect("verification");

    assert!(result.cancelled);
    assert!(!result.required_checks_passed);
    assert_eq!(*world.commands.lock().expect("commands"), ["cancelled"]);
    assert_eq!(result.evidence[0].status, "cancelled");
}

#[test]
fn optional_process_error_records_evidence_without_blocking_acceptance() {
    let world = Arc::new(VerificationWorld::default());
    let result = world
        .service()
        .run_commands(request(vec![command("process-error", false)]))
        .expect("verification result");

    assert!(result.required_checks_passed);
    assert_eq!(result.evidence[0].status, "error");
    assert_eq!(world.evidence.lock().expect("evidence").len(), 1);
    assert!(world
        .operation_events
        .lock()
        .expect("events")
        .iter()
        .any(|event| event.starts_with("fail:")));
    let logs = world.logs.lock().expect("logs");
    assert!(logs.iter().all(|log| log.context.run_id == "run-1"));
    assert!(logs
        .iter()
        .all(|log| { log.context.iteration_id.as_deref() == Some("iteration-1") }));
    assert!(logs
        .iter()
        .all(|log| log.context.kind == LoopOperationKind::Verification));
    assert!(logs.iter().all(|log| log.operation_id.is_some()));
}
