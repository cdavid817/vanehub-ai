use super::*;
use crate::contexts::agent_runtime::domain::{
    LoopDefinition, LoopDefinitionInput, LoopLimits, LoopRun, LoopRunPhase, LoopRunStatus,
    LoopTerminalReason, LoopVerificationCommand,
};
use std::sync::{Arc, Mutex};

struct ControlWorld {
    run: Mutex<LoopRun>,
    snapshot: LoopDefinition,
    feedback: Mutex<Option<String>>,
    cancellation_requests: Mutex<u16>,
    operations: Mutex<Vec<LoopOperationContext>>,
    logs: Mutex<Vec<LoopLog>>,
}

impl ControlWorld {
    fn new(run: LoopRun, max_iterations: u16) -> Arc<Self> {
        Arc::new(Self {
            run: Mutex::new(run),
            snapshot: definition(max_iterations),
            feedback: Mutex::new(None),
            cancellation_requests: Mutex::new(0),
            operations: Mutex::new(Vec::new()),
            logs: Mutex::new(Vec::new()),
        })
    }
    fn service(self: &Arc<Self>) -> LoopControlApplicationService {
        LoopControlApplicationService::new(LoopControlApplicationPorts {
            loops: self.clone(),
            execution: self.clone(),
            observer: LoopOperationObserver::new(self.clone(), self.clone(), self.clone()),
            clock: self.clone(),
        })
    }
}

impl LoopRepository for ControlWorld {
    fn list_definitions(&self) -> Result<Vec<LoopDefinition>, AgentRuntimeApplicationError> {
        Ok(vec![self.snapshot.clone()])
    }
    fn find_definition(
        &self,
        definition_id: &str,
    ) -> Result<Option<LoopDefinition>, AgentRuntimeApplicationError> {
        Ok((definition_id == self.snapshot.values().id).then(|| self.snapshot.clone()))
    }
    fn create_definition(&self, _: &LoopDefinition) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn update_definition(
        &self,
        _: &LoopDefinition,
        _: u64,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn delete_definition(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn create_run(
        &self,
        _: &LoopRun,
        _: &LoopDefinition,
        _: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn has_active_run(&self, _: &str) -> Result<bool, AgentRuntimeApplicationError> {
        Ok(self.run.lock().expect("run").status().is_active())
    }
    fn find_run(&self, run_id: &str) -> Result<Option<LoopRun>, AgentRuntimeApplicationError> {
        let run = self.run.lock().expect("run");
        Ok((run.id() == run_id).then(|| run.clone()))
    }
    fn attach_run_operation(
        &self,
        _: &str,
        _: &str,
        _: LoopRunStatus,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn attach_run_worktree(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: LoopRunStatus,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn save_run_transition(
        &self,
        run: &LoopRun,
        expected_status: LoopRunStatus,
        _: &str,
        _: Option<&str>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut stored = self.run.lock().expect("run");
        if stored.status() != expected_status {
            return Err(loop_conflict());
        }
        *stored = run.clone();
        Ok(())
    }

    fn save_pause_request(
        &self,
        run: &LoopRun,
        expected_status: LoopRunStatus,
        expected_pause_requested: bool,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut stored = self.run.lock().expect("run");
        if stored.status() != expected_status
            || stored.pause_requested() != expected_pause_requested
        {
            return Err(loop_conflict());
        }
        *stored = run.clone();
        Ok(())
    }

    fn find_run_definition_snapshot(
        &self,
        run_id: &str,
    ) -> Result<Option<LoopDefinition>, AgentRuntimeApplicationError> {
        Ok((self.run.lock().expect("run").id() == run_id).then(|| self.snapshot.clone()))
    }

    fn save_continue_transition(
        &self,
        run: &LoopRun,
        expected_status: LoopRunStatus,
        feedback: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut stored = self.run.lock().expect("run");
        let mut saved_feedback = self.feedback.lock().expect("feedback");
        if stored.status() != expected_status || saved_feedback.is_some() {
            return Err(loop_conflict());
        }
        *saved_feedback = Some(feedback.to_string());
        *stored = run.clone();
        Ok(())
    }
}

impl LoopExecutionControlPort for ControlWorld {
    fn request_cancellation(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        *self
            .cancellation_requests
            .lock()
            .expect("cancellation requests") += 1;
        Ok(())
    }
}

impl AgentTaskPort for ControlWorld {
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
        self.operations
            .lock()
            .expect("operations")
            .push(context.clone());
        Ok(AgentOperation {
            id: "cancellation-operation".to_string(),
            related_agent_id: Some(context.run_id.clone()),
            message: Some(message.to_string()),
        })
    }

    fn append_log(&self, _: &str, _: String) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn complete(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn fail(&self, _: &str, _: String) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn cancel(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
}

impl LoopLoggingPort for ControlWorld {
    fn record_loop(&self, log: LoopLog) -> Result<(), AgentRuntimeApplicationError> {
        self.logs.lock().expect("logs").push(log);
        Ok(())
    }
}

impl AgentClockPort for ControlWorld {
    fn now(&self) -> String {
        "2026-07-22T12:00:00Z".to_string()
    }
}

fn running_run() -> LoopRun {
    let mut run = LoopRun::new("run-1".to_string(), "loop-1".to_string()).expect("run");
    run.begin().expect("begin");
    run
}

fn acceptance_run() -> LoopRun {
    let mut run = running_run();
    run.move_to(LoopRunPhase::Verifying).expect("verifying");
    run.move_to(LoopRunPhase::Deciding).expect("deciding");
    run.await_acceptance(true).expect("acceptance");
    run
}

fn definition(max_iterations: u16) -> LoopDefinition {
    LoopDefinition::new(LoopDefinitionInput {
        id: "loop-1".to_string(),
        name: "Loop".to_string(),
        enabled: true,
        project_path: "D:/project".to_string(),
        base_branch: "main".to_string(),
        goal: "Implement the goal".to_string(),
        acceptance_criteria: vec!["Tests pass".to_string()],
        allowed_paths: vec!["src".to_string()],
        protected_paths: vec![".git".to_string()],
        worker_agent_id: "worker".to_string(),
        verifier_agent_id: "verifier".to_string(),
        verification_commands: vec![LoopVerificationCommand::new(
            "tests".to_string(),
            "npm".to_string(),
            vec!["test".to_string()],
            None,
            60,
            true,
        )
        .expect("command")],
        limits: LoopLimits::new(max_iterations, 60, 600, 2, 2).expect("limits"),
        version: 1,
        created_at: "2026-07-22T11:00:00Z".to_string(),
        updated_at: "2026-07-22T11:00:00Z".to_string(),
    })
    .expect("definition")
}

fn loop_conflict() -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Loop("concurrent action".to_string())
}

#[test]
fn pause_is_requested_once_and_applied_only_at_a_boundary() {
    let world = ControlWorld::new(running_run(), 3);
    let service = world.service();

    let requested = service.request_pause("run-1").expect("request pause");
    assert_eq!(requested.status(), LoopRunStatus::Running);
    assert!(requested.pause_requested());
    assert!(service.request_pause("run-1").is_err());

    let paused = service.pause_at_boundary("run-1").expect("pause boundary");
    assert_eq!(paused.status(), LoopRunStatus::Paused);
    let resumed = service.resume("run-1").expect("resume");
    assert_eq!(resumed.status(), LoopRunStatus::Running);
    assert!(service.resume("run-1").is_err());
}

#[test]
fn cancellation_requests_child_stop_and_rejects_duplicates() {
    let world = ControlWorld::new(running_run(), 3);
    let service = world.service();

    let cancelled = service.cancel("run-1").expect("cancel");
    assert_eq!(cancelled.status(), LoopRunStatus::Cancelled);
    assert_eq!(
        cancelled.terminal_reason(),
        Some(LoopTerminalReason::UserStopped)
    );
    assert!(service.cancel("run-1").is_err());
    assert_eq!(*world.cancellation_requests.lock().expect("requests"), 1);
    let operations = world.operations.lock().expect("operations");
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].kind, LoopOperationKind::Cancellation);
    let logs = world.logs.lock().expect("logs");
    assert_eq!(logs.len(), 2);
    assert!(logs.iter().all(|log| log.context.run_id == "run-1"));
    assert!(logs
        .iter()
        .all(|log| log.operation_id.as_deref() == Some("cancellation-operation")));
}

#[test]
fn human_accept_and_reject_are_terminal_one_time_actions() {
    let accepted_world = ControlWorld::new(acceptance_run(), 3);
    let accepted = accepted_world.service().accept("run-1").expect("accept");
    assert_eq!(accepted.status(), LoopRunStatus::Succeeded);
    assert!(accepted_world.service().accept("run-1").is_err());

    let rejected_world = ControlWorld::new(acceptance_run(), 3);
    let rejected = rejected_world.service().reject("run-1").expect("reject");
    assert_eq!(rejected.status(), LoopRunStatus::Cancelled);
    assert_eq!(
        rejected.terminal_reason(),
        Some(LoopTerminalReason::UserRejected)
    );
    assert!(rejected_world.service().reject("run-1").is_err());
}

#[test]
fn continuation_requires_feedback_and_atomically_advances_once() {
    let world = ControlWorld::new(acceptance_run(), 3);
    let service = world.service();
    assert!(service
        .continue_with_feedback(ContinueLoopRequest {
            run_id: "run-1".to_string(),
            feedback: "   ".to_string(),
        })
        .is_err());

    let continued = service
        .continue_with_feedback(ContinueLoopRequest {
            run_id: "run-1".to_string(),
            feedback: " Add the missing regression test. ".to_string(),
        })
        .expect("continue");
    assert_eq!(continued.current_iteration(), 2);
    assert_eq!(continued.phase(), LoopRunPhase::Acting);
    assert_eq!(
        world.feedback.lock().expect("feedback").as_deref(),
        Some("Add the missing regression test.")
    );
    assert!(service
        .continue_with_feedback(ContinueLoopRequest {
            run_id: "run-1".to_string(),
            feedback: "Again".to_string(),
        })
        .is_err());

    let limited = ControlWorld::new(acceptance_run(), 1);
    assert!(limited
        .service()
        .continue_with_feedback(ContinueLoopRequest {
            run_id: "run-1".to_string(),
            feedback: "More".to_string(),
        })
        .is_err());
}
