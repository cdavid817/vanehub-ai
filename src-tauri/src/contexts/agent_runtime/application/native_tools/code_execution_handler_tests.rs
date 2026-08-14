use super::*;
use crate::contexts::agent_runtime::application::native_tools::{
    NativeToolErrorCode, NativeToolResultStatus,
};
use serde_json::json;
use std::sync::Mutex;

#[derive(Default)]
struct Port(Mutex<Vec<Value>>);

impl CodeExecutionPort for Port {
    fn execute_code(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        self.0.lock().expect("requests").push(request.input.value);
        NativeToolResultEnvelope {
            contract_version: NATIVE_TOOL_CONTRACT_VERSION,
            status: NativeToolResultStatus::Succeeded,
            output: Some(json!({"status": "succeeded"})),
            error_code: None,
            safe_error: None,
            truncated: false,
            metadata: BTreeMap::new(),
        }
    }
}

fn context(ready: bool) -> ToolEligibilityContext {
    ToolEligibilityContext {
        agent_id: "onepiece".to_owned(),
        session_id: "session-1".to_owned(),
        generation_id: "generation-1".to_owned(),
        canonical_workspace: None,
        execution_mode: NativeToolExecutionMode::Execute,
        readiness: BTreeMap::from([(
            "code_execution".to_owned(),
            (!ready).then_some(NativeToolReadinessReasonCode::IsolationUnavailable),
        )]),
    }
}

fn valid_input(source: &str) -> Value {
    json!({
        "runtime": "python",
        "source": source,
        "arguments": ["one"],
        "inputs": [{"artifact_id": "artifact-1", "content_hash": "a".repeat(64)}],
        "limits": {"wall_time_ms": 1000, "memory_bytes": 1048576}
    })
}

#[test]
fn permission_binds_runtime_source_artifacts_limits_and_exact_input() {
    let handler = CodeExecutionNativeToolHandler::new(Arc::new(Port::default()));
    let first = handler
        .validate(&valid_input("print('one')"))
        .expect("first");
    let second = handler
        .validate(&valid_input("print('two')"))
        .expect("second");
    assert_eq!(first.operation, NativeToolOperation::CodeExecute);
    assert_ne!(first.input_hash, second.input_hash);
    assert_ne!(first.resource.canonical_id, second.resource.canonical_id);
    let permission = handler.permission_request(&first, &context(true));
    assert!(permission.resource.as_str().contains("code-runtime/python"));
    assert!(permission.resource.as_str().contains("#inputs=sha256:"));
    assert!(permission.resource.as_str().contains("&limits=sha256:"));
    assert!(!permission.resource.as_str().contains("print"));
}

#[test]
fn unsupported_runtime_raised_limits_and_forged_fields_fail() {
    let handler = CodeExecutionNativeToolHandler::new(Arc::new(Port::default()));
    for input in [
        json!({"runtime": "powershell", "source": "echo bad"}),
        json!({"runtime": "python", "source": "ok", "limits": {"process_count": 3}}),
        json!({"runtime": "javascript", "source": "ok", "command": "cmd.exe"}),
    ] {
        assert_eq!(
            handler.validate(&input).expect_err("invalid").code,
            NativeToolErrorCode::InvalidInput
        );
    }
}

#[test]
fn sandbox_readiness_and_plan_policy_are_independently_enforced() {
    let handler = CodeExecutionNativeToolHandler::new(Arc::new(Port::default()));
    assert_eq!(
        handler.eligibility(&context(true)),
        ToolEligibility::Eligible
    );
    assert_eq!(
        handler.eligibility(&context(false)),
        ineligible(NativeToolReadinessReasonCode::IsolationUnavailable)
    );
    let mut plan = context(true);
    plan.execution_mode = NativeToolExecutionMode::Plan;
    assert_eq!(
        handler.eligibility(&plan),
        ineligible(NativeToolReadinessReasonCode::PolicyUnavailable)
    );
}
