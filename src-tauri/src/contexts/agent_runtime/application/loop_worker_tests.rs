use super::*;
use crate::contexts::agent_runtime::domain::LoopRunStatus;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct WorkerWorld {
    calls: Mutex<Vec<String>>,
    iterations: Mutex<Vec<LoopIterationView>>,
    attached_sessions: Mutex<Vec<(String, String)>>,
    prompts: Mutex<Vec<String>>,
    summaries: Mutex<Vec<(String, String, String, String)>>,
    session_sequence: Mutex<u16>,
}

impl WorkerWorld {
    fn service(self: &Arc<Self>) -> LoopWorkerApplicationService {
        LoopWorkerApplicationService::new(LoopWorkerApplicationPorts {
            iterations: self.clone(),
            roles: self.clone(),
            git: self.clone(),
            generations: self.clone(),
            clock: self.clone(),
        })
    }

    fn record(&self, call: &str) {
        self.calls.lock().expect("calls").push(call.to_string());
    }
}

impl LoopIterationRepository for WorkerWorld {
    fn insert_iteration(
        &self,
        iteration: &LoopIterationView,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.record("insert");
        self.iterations
            .lock()
            .expect("iterations")
            .push(iteration.clone());
        Ok(())
    }

    fn attach_worker_session(
        &self,
        iteration_id: &str,
        session_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.record("attach");
        self.attached_sessions
            .lock()
            .expect("sessions")
            .push((iteration_id.to_string(), session_id.to_string()));
        Ok(())
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

    fn save_worker_summary(
        &self,
        run_id: &str,
        iteration_id: &str,
        session_id: &str,
        summary: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.summaries.lock().expect("summaries").push((
            run_id.to_string(),
            iteration_id.to_string(),
            session_id.to_string(),
            summary.to_string(),
        ));
        Ok(())
    }

    fn save_iteration_fingerprints(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn append_evidence(&self, _: &LoopEvidenceView) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
}

impl LoopRoleSessionPort for WorkerWorld {
    fn create_worker_session(
        &self,
        request: LoopRoleSessionRequest,
    ) -> Result<String, AgentRuntimeApplicationError> {
        self.record("role");
        assert_eq!(request.agent_id, "worker-agent");
        assert_eq!(request.worktree_path, "C:/work/project-loop");
        let mut sequence = self.session_sequence.lock().expect("sequence");
        *sequence += 1;
        Ok(format!("worker-session-{sequence}"))
    }

    fn create_verifier_session(
        &self,
        _: LoopRoleSessionRequest,
    ) -> Result<String, AgentRuntimeApplicationError> {
        unreachable!()
    }
}

impl LoopGitStatePort for WorkerWorld {
    fn snapshot(&self, session_id: &str) -> Result<LoopGitStateView, AgentRuntimeApplicationError> {
        self.record("git");
        assert!(session_id.starts_with("worker-session-"));
        Ok(LoopGitStateView {
            branch: Some("vanehub/loop-1".to_string()),
            entries: vec![LoopGitStateEntryView {
                path: "src/worker.rs".to_string(),
                index_status: "modified".to_string(),
                worktree_status: "modified".to_string(),
            }],
            truncated: false,
        })
    }
}

impl LoopWorkerGenerationPort for WorkerWorld {
    fn start_worker_generation(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        self.record("generate");
        self.prompts
            .lock()
            .expect("prompts")
            .push(prompt.to_string());
        Ok(format!("message-{session_id}"))
    }
}

impl AgentClockPort for WorkerWorld {
    fn now(&self) -> String {
        "2026-07-22T08:00:00Z".to_string()
    }
}

fn definition() -> LoopDefinitionView {
    LoopDefinitionView {
        id: "loop-1".to_string(),
        name: "Worker loop".to_string(),
        enabled: true,
        project_path: "C:/work/project".to_string(),
        base_branch: "main".to_string(),
        goal: "Implement bounded Worker orchestration".to_string(),
        acceptance_criteria: vec!["Worker context is durable".to_string()],
        allowed_paths: vec!["src".to_string()],
        protected_paths: vec![".git".to_string()],
        worker_agent_id: "worker-agent".to_string(),
        verifier_agent_id: "verifier-agent".to_string(),
        verification_commands: Vec::new(),
        limits: LoopLimitsView {
            max_iterations: 3,
            step_timeout_seconds: 60,
            total_timeout_seconds: 600,
            max_consecutive_runtime_errors: 2,
            max_consecutive_no_progress: 2,
        },
        version: 1,
        created_at: "2026-07-22T07:00:00Z".to_string(),
        updated_at: "2026-07-22T07:00:00Z".to_string(),
    }
}

fn request(sequence: u16) -> StartLoopWorkerRequest {
    StartLoopWorkerRequest {
        run_id: "run-1".to_string(),
        sequence,
        definition_snapshot: definition(),
        project_path: "C:/work/project".to_string(),
        worktree_path: "C:/work/project-loop".to_string(),
        worktree_name: "loop-1".to_string(),
        worktree_branch: "vanehub/loop-1".to_string(),
        prior_evidence: vec![LoopEvidenceView {
            id: "evidence-1".to_string(),
            run_id: "run-1".to_string(),
            iteration_id: Some("iteration-previous".to_string()),
            kind: "verification-command".to_string(),
            status: "failed".to_string(),
            summary: "Previous tests failed".to_string(),
            operation_id: None,
            command_id: Some("tests".to_string()),
            exit_code: Some(1),
            duration_ms: Some(10),
            details: None,
            created_at: "2026-07-22T07:30:00Z".to_string(),
        }],
        user_feedback: Some("Keep the public API stable".to_string()),
        elapsed_seconds: 120,
    }
}

#[test]
fn worker_iteration_persists_before_side_effects_and_builds_complete_context() {
    let world = Arc::new(WorkerWorld::default());
    let started = world.service().start_iteration(request(2)).expect("worker");

    assert_eq!(
        *world.calls.lock().expect("calls"),
        ["insert", "role", "attach", "git", "generate"]
    );
    let iteration = &world.iterations.lock().expect("iterations")[0];
    assert_eq!(iteration.status, LoopRunStatus::Running);
    assert_eq!(iteration.sequence, 2);
    assert_eq!(iteration.worker_session_id, None);
    assert_eq!(started.session_id, "worker-session-1");
    let prompt = &world.prompts.lock().expect("prompts")[0];
    for expected in [
        "Implement bounded Worker orchestration",
        "Worker context is durable",
        "src/worker.rs",
        "Previous tests failed",
        "Keep the public API stable",
        "remaining total seconds: 480",
    ] {
        assert!(prompt.contains(expected), "missing {expected}");
    }
    assert!(started.context_bytes <= 32 * 1024);
}

#[test]
fn each_iteration_uses_a_fresh_session_and_bounds_unicode_context() {
    let world = Arc::new(WorkerWorld::default());
    let first = world.service().start_iteration(request(1)).expect("first");
    let mut next = request(2);
    next.definition_snapshot.goal = "目标".repeat(20_000);
    next.prior_evidence[0].summary = "证据".repeat(20_000);
    next.user_feedback = Some("反馈".repeat(20_000));
    let second = world.service().start_iteration(next).expect("second");

    assert_ne!(first.session_id, second.session_id);
    assert!(second.context_bytes <= 32 * 1024);
    assert!(world.prompts.lock().expect("prompts")[1].is_char_boundary(second.context_bytes));
}

#[test]
fn worker_context_rejects_cross_run_evidence_before_persistence() {
    let world = Arc::new(WorkerWorld::default());
    let mut invalid = request(1);
    invalid.prior_evidence[0].run_id = "other-run".to_string();

    assert!(world.service().start_iteration(invalid).is_err());
    assert!(world.calls.lock().expect("calls").is_empty());
}

#[test]
fn completed_worker_terminal_is_owned_bounded_and_persisted() {
    let world = Arc::new(WorkerWorld::default());
    let started = world.service().start_iteration(request(1)).expect("worker");
    let summary = "结果".repeat(10_000);
    let completed = world
        .service()
        .complete(LoopRoleGenerationTerminal {
            run_id: "run-1".to_string(),
            iteration_id: started.iteration_id.clone(),
            role: "worker".to_string(),
            session_id: started.session_id.clone(),
            message_id: started.message_id,
            outcome: LoopRoleGenerationOutcome::Completed,
            content: Some(summary),
            error: None,
        })
        .expect("complete");

    assert!(completed.len() <= 8 * 1024);
    let saved = world.summaries.lock().expect("summaries");
    assert_eq!(saved[0].0, "run-1");
    assert_eq!(saved[0].1, started.iteration_id);
    assert_eq!(saved[0].2, started.session_id);
    assert_eq!(saved[0].3, completed);
}
