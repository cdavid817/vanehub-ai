use super::*;
use crate::contexts::tooling::skill_tools::domain::{
    SkillToolId, SkillToolKey, SkillToolOwnerId, SkillToolRevision, SkillToolSourceScope,
};
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Call {
    agent: String,
    action: String,
    resource: String,
    session: String,
    generation: String,
    project: String,
}

struct Evaluator {
    effect: Effect,
    calls: Mutex<Vec<Call>>,
    approvals: Mutex<Vec<SkillApprovalProvenance>>,
}

impl SkillPolicyEvaluation for Evaluator {
    fn evaluate(
        &self,
        agent_id: &str,
        action: Action,
        resource: Resource,
        session_id: &str,
        generation_id: &str,
        project_key: &str,
    ) -> Effect {
        self.calls.lock().expect("calls").push(Call {
            agent: agent_id.to_string(),
            action: action.as_str().to_string(),
            resource: resource.as_str().to_string(),
            session: session_id.to_string(),
            generation: generation_id.to_string(),
            project: project_key.to_string(),
        });
        self.effect
    }

    fn create_skill_pending(
        &self,
        provenance: SkillApprovalProvenance,
        _action: Action,
        _resource: Resource,
        _session_id: &str,
        _generation_id: &str,
        _call_id: &str,
        _project_key: &str,
    ) -> Result<String, String> {
        self.approvals.lock().expect("approvals").push(provenance);
        Ok("approval-1".to_string())
    }
}

fn principal() -> SkillToolPrincipal {
    let key = SkillToolKey::new(
        SkillToolOwnerId::parse("review").expect("owner"),
        SkillToolSourceScope::global(),
        SkillToolId::parse("check").expect("tool"),
        SkillToolRevision::parse(&"a".repeat(64)).expect("revision"),
    );
    SkillToolPrincipal::new(
        "codex",
        key,
        Some("/workspace"),
        "session-1",
        "generation-1",
        vec![],
    )
    .expect("principal")
}

#[test]
fn concrete_operation_is_independently_mapped_with_full_context() {
    let evaluator = Arc::new(Evaluator {
        effect: Effect::Allow,
        calls: Mutex::new(Vec::new()),
        approvals: Mutex::new(Vec::new()),
    });
    let adapter = SkillToolPermissionAdapter::with_evaluator(evaluator.clone(), true);
    let capability = SkillToolCapability::parse("tool:read_file").expect("capability");

    assert_eq!(
        adapter.evaluate(
            &principal(),
            &capability,
            &serde_json::json!({"path":"src/lib.rs"})
        ),
        SkillToolPermissionDecision::Allow
    );
    assert_eq!(
        evaluator.calls.lock().expect("calls").as_slice(),
        &[Call {
            agent: "codex".to_string(),
            action: "file.read".to_string(),
            resource: "src/lib.rs".to_string(),
            session: "session-1".to_string(),
            generation: "generation-1".to_string(),
            project: "/workspace".to_string(),
        }]
    );
}

#[test]
fn all_policy_states_are_preserved_and_unknown_operations_fail_to_ask() {
    for (effect, expected) in [
        (Effect::Allow, SkillToolPermissionDecision::Allow),
        (Effect::Ask, SkillToolPermissionDecision::Ask),
        (Effect::Deny, SkillToolPermissionDecision::Deny),
    ] {
        let adapter = SkillToolPermissionAdapter::with_evaluator(
            Arc::new(Evaluator {
                effect,
                calls: Mutex::new(Vec::new()),
                approvals: Mutex::new(Vec::new()),
            }),
            true,
        );
        let capability = SkillToolCapability::parse("tool:future_operation").expect("capability");
        assert_eq!(
            adapter.evaluate(&principal(), &capability, &Value::Null),
            expected
        );
    }
}

#[test]
fn absent_session_context_denies_without_policy_evaluation() {
    let evaluator = Arc::new(Evaluator {
        effect: Effect::Allow,
        calls: Mutex::new(Vec::new()),
        approvals: Mutex::new(Vec::new()),
    });
    let adapter = SkillToolPermissionAdapter::with_evaluator(evaluator.clone(), true);
    let mut missing = principal();
    missing.session_id = None;
    let capability = SkillToolCapability::parse("tool:read_file").expect("capability");

    assert_eq!(
        adapter.evaluate(&missing, &capability, &Value::Null),
        SkillToolPermissionDecision::Deny
    );
    assert!(evaluator.calls.lock().expect("calls").is_empty());
}

#[test]
fn approval_is_revision_bound_redacted_and_not_a_policy_grant() {
    let evaluator = Arc::new(Evaluator {
        effect: Effect::Ask,
        calls: Mutex::new(Vec::new()),
        approvals: Mutex::new(Vec::new()),
    });
    let adapter = SkillToolPermissionAdapter::with_evaluator(evaluator.clone(), true);
    let capability = SkillToolCapability::parse("tool:write_file").expect("capability");
    let id = adapter
        .create_pending(
            &principal(),
            &capability,
            &serde_json::json!({"path":"secret.txt","content":"do-not-leak"}),
            "call-1",
        )
        .expect("approval");

    assert_eq!(id, "approval-1");
    let approvals = evaluator.approvals.lock().expect("approvals");
    let provenance = approvals.first().expect("provenance");
    assert_eq!(provenance.parent_agent_id, "codex");
    assert_eq!(provenance.skill_id, "review");
    assert_eq!(provenance.tool_id, "check");
    assert_eq!(provenance.requested_capability, "tool:write_file");
    assert_eq!(provenance.delegated_operation, "write_file");
    assert!(provenance.immutable_witness.starts_with("sha256:"));
    assert!(!provenance.redacted_input_summary.contains("do-not-leak"));
    assert!(!provenance.redacted_input_summary.contains("secret.txt"));
}

#[test]
fn ask_without_a_supported_approval_channel_fails_closed() {
    let evaluator = Arc::new(Evaluator {
        effect: Effect::Ask,
        calls: Mutex::new(Vec::new()),
        approvals: Mutex::new(Vec::new()),
    });
    let adapter = SkillToolPermissionAdapter::with_evaluator(evaluator.clone(), false);
    let capability = SkillToolCapability::parse("tool:write_file").expect("capability");

    let error = adapter
        .create_pending(
            &principal(),
            &capability,
            &serde_json::json!({"path":"src/lib.rs"}),
            "call-1",
        )
        .expect_err("unavailable channel must deny");
    assert_eq!(error.code(), "host-denied");
    assert!(evaluator.approvals.lock().expect("approvals").is_empty());
}
