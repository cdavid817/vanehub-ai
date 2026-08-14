use super::*;
use crate::contexts::agent_runtime::application::native_tools::{
    NativeToolErrorCode, NativeToolResultStatus,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Default)]
struct CapturingPort(Mutex<Vec<Value>>);

impl BrowserAutomationPort for CapturingPort {
    fn execute_browser(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        self.0.lock().expect("requests").push(request.input.value);
        NativeToolResultEnvelope {
            contract_version: NATIVE_TOOL_CONTRACT_VERSION,
            status: NativeToolResultStatus::Succeeded,
            output: Some(json!({"page_id": "page-1"})),
            error_code: None,
            safe_error: None,
            truncated: false,
            metadata: BTreeMap::new(),
        }
    }
}

fn context(agent_id: &str, ready: bool) -> ToolEligibilityContext {
    ToolEligibilityContext {
        agent_id: agent_id.to_owned(),
        session_id: "session-1".to_owned(),
        generation_id: "generation-1".to_owned(),
        canonical_workspace: None,
        execution_mode: NativeToolExecutionMode::Execute,
        readiness: BTreeMap::from([(
            "browser".to_owned(),
            (!ready).then_some(NativeToolReadinessReasonCode::MissingDependency),
        )]),
    }
}

#[test]
fn fixed_schema_classifies_passive_and_effectful_operations() {
    let handler = BrowserNativeToolHandler::new(Arc::new(CapturingPort::default()));
    let navigate = handler
        .validate(&json!({"operation": "navigate", "url": "https://Example.com/a"}))
        .expect("navigate");
    assert_eq!(navigate.operation, NativeToolOperation::BrowserRead);
    assert_eq!(navigate.resource.canonical_id, "https://example.com");

    let fill = handler
        .validate(&json!({
            "operation": "type",
            "page_origin": "https://example.com/login",
            "selector": "#password",
            "text": "secret"
        }))
        .expect("type");
    assert_eq!(fill.operation, NativeToolOperation::BrowserInteract);
    let permission = handler.permission_request(&fill, &context("onepiece", true));
    assert!(permission.resource.as_str().contains("session-1"));
    assert!(permission.resource.as_str().contains("https://example.com"));
    assert!(!permission.resource.as_str().contains("password"));
    assert!(!permission.resource.as_str().contains("secret"));
}

#[test]
fn malformed_operations_and_forged_origins_fail_validation() {
    let handler = BrowserNativeToolHandler::new(Arc::new(CapturingPort::default()));
    for input in [
        json!({"operation": "evaluate", "page_origin": "https://example.com"}),
        json!({"operation": "click", "page_origin": "file:///tmp/a", "selector": "#x"}),
        json!({"operation": "handoff", "page_origin": "https://example.com", "handoff_seconds": 901}),
        json!({"operation": "inspect", "page_origin": "https://example.com", "extra": true}),
    ] {
        assert_eq!(
            handler.validate(&input).expect_err("invalid").code,
            NativeToolErrorCode::InvalidInput
        );
    }
}

#[test]
fn readiness_is_independent_and_plan_mode_is_excluded() {
    let handler = BrowserNativeToolHandler::new(Arc::new(CapturingPort::default()));
    assert_eq!(
        handler.eligibility(&context("onepiece", true)),
        ToolEligibility::Eligible
    );
    assert_eq!(
        handler.eligibility(&context("onepiece", false)),
        ineligible(NativeToolReadinessReasonCode::MissingDependency)
    );
    let mut plan = context("onepiece", true);
    plan.execution_mode = NativeToolExecutionMode::Plan;
    assert_eq!(
        handler.eligibility(&plan),
        ineligible(NativeToolReadinessReasonCode::PolicyUnavailable)
    );
}
