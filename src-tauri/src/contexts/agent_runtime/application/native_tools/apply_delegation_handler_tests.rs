use super::*;
use crate::contexts::agent_runtime::application::{
    NativeToolExecutionMode, NativeToolRegistry, NativeToolResultStatus, ToolEligibility,
};
use serde_json::json;

struct Port;

impl ChangeSetApplyPort for Port {
    fn execute_change_set_apply(&self, _: NativeToolPortRequest) -> NativeToolResultEnvelope {
        NativeToolResultEnvelope {
            contract_version: NATIVE_TOOL_CONTRACT_VERSION,
            status: NativeToolResultStatus::Succeeded,
            output: None,
            error_code: None,
            safe_error: None,
            truncated: false,
            metadata: BTreeMap::new(),
        }
    }
}

fn input() -> Value {
    json!({
        "artifact_id": "artifact-1",
        "content_hash": format!("sha256:{}", "a".repeat(64)),
        "diff_hash": format!("sha256:{}", "b".repeat(64)),
        "target_repository_identity": "git:C:/repo:base",
        "base_commit": "c".repeat(40),
        "acknowledged": true
    })
}

fn context(agent_id: &str) -> ToolEligibilityContext {
    ToolEligibilityContext {
        agent_id: agent_id.into(),
        session_id: "session-1".into(),
        generation_id: "generation-1".into(),
        canonical_workspace: None,
        execution_mode: NativeToolExecutionMode::Execute,
        readiness: BTreeMap::from([("apply_delegation_changes".into(), None)]),
    }
}

#[test]
fn registers_as_a_separate_onepiece_only_tool_with_exact_approval_binding() {
    let handler = Arc::new(ApplyDelegationChangesNativeToolHandler::new(Arc::new(Port)));
    let registry = NativeToolRegistry::try_new(vec![handler.clone()]).expect("registry");
    assert_eq!(registry.len(), 1);
    assert_eq!(
        registry.eligibility("apply_delegation_changes", &context("onepiece")),
        ToolEligibility::Eligible
    );
    assert!(matches!(
        registry.eligibility("apply_delegation_changes", &context("custom-api")),
        ToolEligibility::Ineligible { .. }
    ));

    let validated = handler.validate(&input()).expect("validated");
    let permission = handler.permission_request(&validated, &context("onepiece"));
    assert_eq!(permission.action.as_str(), "delegation.apply");
    assert!(permission.resource.as_str().contains("artifact-1"));
    assert!(permission.resource.as_str().contains(&"a".repeat(64)));
    assert!(permission.resource.as_str().contains("git:C:/repo:base"));
    assert!(permission.resource.as_str().contains(&"c".repeat(40)));
    assert_eq!(permission.input_hash, validated.input_hash);
}

#[test]
fn rejects_unacknowledged_malformed_or_extended_requests() {
    let handler = ApplyDelegationChangesNativeToolHandler::new(Arc::new(Port));
    let mut unacknowledged = input();
    unacknowledged["acknowledged"] = Value::Bool(false);
    assert!(handler.validate(&unacknowledged).is_err());

    let mut malformed = input();
    malformed["diff_hash"] = Value::String("sha256:short".into());
    assert!(handler.validate(&malformed).is_err());

    let mut extended = input();
    extended["partial_files"] = json!(["src/lib.rs"]);
    assert!(handler.validate(&extended).is_err());
}
