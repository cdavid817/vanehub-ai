use super::*;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct VerifierWorld {
    sessions: Mutex<Vec<LoopRoleSessionRequest>>,
    attachments: Mutex<Vec<(String, String)>>,
    prompts: Mutex<Vec<String>>,
    saved: Mutex<Vec<(String, Vec<String>)>>,
}

impl VerifierWorld {
    fn service(self: &Arc<Self>) -> LoopVerifierApplicationService {
        LoopVerifierApplicationService::new(LoopVerifierApplicationPorts {
            iterations: self.clone(),
            roles: self.clone(),
            context: self.clone(),
            generations: self.clone(),
        })
    }
}

impl LoopIterationRepository for VerifierWorld {
    fn insert_iteration(&self, _: &LoopIterationView) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn attach_worker_session(&self, _: &str, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn attach_verifier_session(
        &self,
        iteration_id: &str,
        session_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.attachments
            .lock()
            .expect("attachments")
            .push((iteration_id.to_string(), session_id.to_string()));
        Ok(())
    }

    fn save_verifier_result(
        &self,
        request: &SaveLoopVerifierResultRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        if request.run_id != "run-1"
            || request.iteration_id != "iteration-1"
            || !request.session_id.starts_with("verifier-session-")
        {
            return Err(AgentRuntimeApplicationError::Loop(
                "invalid Verifier result ownership".to_string(),
            ));
        }
        self.saved.lock().expect("saved").push((
            request.result.recommendation.as_str().to_string(),
            request.result.findings.clone(),
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
        unreachable!()
    }
}

impl LoopRoleSessionPort for VerifierWorld {
    fn create_worker_session(
        &self,
        _: LoopRoleSessionRequest,
    ) -> Result<String, AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn create_verifier_session(
        &self,
        request: LoopRoleSessionRequest,
    ) -> Result<String, AgentRuntimeApplicationError> {
        let mut sessions = self.sessions.lock().expect("sessions");
        sessions.push(request);
        Ok(format!("verifier-session-{}", sessions.len()))
    }
}

impl LoopVerifierContextPort for VerifierWorld {
    fn bounded_diff(&self, _: &str) -> Result<String, AgentRuntimeApplicationError> {
        Ok("界".repeat(20_000))
    }
}

impl LoopVerifierGenerationPort for VerifierWorld {
    fn start_verifier_generation(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        self.prompts
            .lock()
            .expect("prompts")
            .push(prompt.to_string());
        Ok(format!("message-{session_id}"))
    }
}

fn definition() -> LoopDefinitionView {
    LoopDefinitionView {
        id: "loop-1".to_string(),
        name: "Verifier loop".to_string(),
        enabled: true,
        project_path: "C:/work/project".to_string(),
        base_branch: "main".to_string(),
        goal: "Verify the implementation independently".to_string(),
        acceptance_criteria: vec!["All required checks pass".to_string()],
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
        created_at: "2026-07-22T00:00:00Z".to_string(),
        updated_at: "2026-07-22T00:00:00Z".to_string(),
    }
}

fn evidence() -> LoopEvidenceView {
    LoopEvidenceView {
        id: "evidence-1".to_string(),
        run_id: "run-1".to_string(),
        iteration_id: Some("iteration-1".to_string()),
        kind: "verification-command".to_string(),
        status: "passed".to_string(),
        summary: "tests passed".to_string(),
        operation_id: Some("operation-1".to_string()),
        command_id: Some("tests".to_string()),
        exit_code: Some(0),
        duration_ms: Some(20),
        details: None,
        created_at: "2026-07-22T00:01:00Z".to_string(),
    }
}

fn request() -> StartLoopVerifierRequest {
    StartLoopVerifierRequest {
        run_id: "run-1".to_string(),
        iteration_id: "iteration-1".to_string(),
        definition_snapshot: definition(),
        project_path: "C:/work/project".to_string(),
        worktree_path: "C:/work/project-loop".to_string(),
        worktree_name: "loop-worktree".to_string(),
        worktree_branch: "vanehub/loop-1".to_string(),
        check_evidence: vec![evidence()],
    }
}

fn terminal(content: &str) -> LoopRoleGenerationTerminal {
    LoopRoleGenerationTerminal {
        run_id: "run-1".to_string(),
        iteration_id: "iteration-1".to_string(),
        role: "verifier".to_string(),
        session_id: "verifier-session-1".to_string(),
        message_id: "message-1".to_string(),
        outcome: LoopRoleGenerationOutcome::Completed,
        content: Some(content.to_string()),
        error: None,
    }
}

#[test]
fn each_start_uses_a_fresh_session_and_bounded_read_only_context() {
    let world = Arc::new(VerifierWorld::default());
    let first = world.service().start(request()).expect("first verifier");
    let second = world.service().start(request()).expect("second verifier");

    assert_ne!(first.session_id, second.session_id);
    assert!(first.context_bytes <= 32 * 1024);
    let prompts = world.prompts.lock().expect("prompts");
    assert!(prompts[0].contains("read-only Verifier"));
    assert!(prompts[0].contains("tests [passed]: tests passed"));
    assert!(prompts[0].is_char_boundary(prompts[0].len()));
    assert_eq!(world.attachments.lock().expect("attachments").len(), 2);
    assert_eq!(
        world.sessions.lock().expect("sessions")[0].agent_id,
        "verifier-agent"
    );
}

#[test]
fn completed_structured_result_is_validated_and_saved() {
    let world = Arc::new(VerifierWorld::default());
    let result = world
        .service()
        .complete(terminal(
            r#"{"recommendation":"revise","findings":["Fix the timeout handling."]}"#,
        ))
        .expect("result");

    assert_eq!(result.recommendation, LoopVerifierRecommendation::Revise);
    assert_eq!(world.saved.lock().expect("saved")[0].0, "revise");
}

#[test]
fn malformed_or_unactionable_results_are_rejected_without_persistence() {
    let world = Arc::new(VerifierWorld::default());
    for content in [
        "```json\n{}\n```",
        r#"{"recommendation":"approve","findings":[]}"#,
        r#"{"recommendation":"blocked","findings":[]}"#,
        r#"{"recommendation":"pass","findings":[],"extra":true}"#,
    ] {
        assert!(world.service().complete(terminal(content)).is_err());
    }
    assert!(world.saved.lock().expect("saved").is_empty());
}

#[test]
fn terminal_role_and_repository_ownership_are_enforced() {
    let world = Arc::new(VerifierWorld::default());
    let mut wrong_role = terminal(r#"{"recommendation":"pass","findings":[]}"#);
    wrong_role.role = "worker".to_string();
    assert!(world.service().complete(wrong_role).is_err());

    let mut wrong_run = terminal(r#"{"recommendation":"pass","findings":[]}"#);
    wrong_run.run_id = "run-2".to_string();
    assert!(world.service().complete(wrong_run).is_err());
    assert!(world.saved.lock().expect("saved").is_empty());
}
