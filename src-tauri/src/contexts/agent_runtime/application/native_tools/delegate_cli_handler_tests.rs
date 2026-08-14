use super::*;
use crate::contexts::agent_runtime::application::{
    NativeToolProgress, NativeToolProgressSink, NativeToolResultStatus,
};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct NoopProgress;

impl NativeToolProgressSink for NoopProgress {
    fn publish(&self, _progress: NativeToolProgress) {}
}

struct Port;

impl CliDelegationPort for Port {
    fn execute_delegation(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        NativeToolResultEnvelope {
            contract_version: NATIVE_TOOL_CONTRACT_VERSION,
            status: NativeToolResultStatus::Succeeded,
            output: Some(json!({"operation": request.input.operation.as_str()})),
            error_code: None,
            safe_error: None,
            truncated: false,
            metadata: BTreeMap::new(),
        }
    }
}

fn eligibility() -> ToolEligibilityContext {
    ToolEligibilityContext {
        agent_id: "onepiece".to_owned(),
        session_id: "session-1".to_owned(),
        generation_id: "generation-1".to_owned(),
        canonical_workspace: Some(PathBuf::from("C:/workspace")),
        execution_mode: NativeToolExecutionMode::Execute,
        readiness: BTreeMap::from([("delegate_cli".to_owned(), None)]),
    }
}

fn execution() -> NativeToolExecutionContext {
    NativeToolExecutionContext {
        call_id: "delegation-1".to_owned(),
        session_id: "session-1".to_owned(),
        generation_id: "generation-1".to_owned(),
        agent_id: "onepiece".to_owned(),
        canonical_workspace: Some(PathBuf::from("C:/workspace")),
        deadline: Instant::now() + Duration::from_secs(5),
        cancelled: Arc::new(AtomicBool::new(false)),
        progress: Arc::new(NoopProgress),
    }
}

#[test]
fn fixed_handler_validates_target_mode_and_artifact_ids() {
    let handler = DelegateCliNativeToolHandler::new(Arc::new(Port));
    assert_eq!(handler.definition().name, "delegate_cli");
    assert_eq!(
        handler.eligibility(&eligibility()),
        ToolEligibility::Eligible
    );
    let analyze = handler
        .validate(&json!({
            "target": "claude_code",
            "mode": "analyze",
            "task": "Review the repository",
            "artifact_ids": ["artifact-123"]
        }))
        .expect("analyze");
    let edit = handler
        .validate(&json!({
            "target": "codex_cli",
            "mode": "edit",
            "task": "Prepare an isolated patch"
        }))
        .expect("edit");
    assert_eq!(analyze.operation, NativeToolOperation::DelegationAnalyze);
    assert_eq!(edit.operation, NativeToolOperation::DelegationEdit);
    assert!(handler
        .validate(&json!({"target": "other", "mode": "analyze", "task": "x"}))
        .is_err());
}

#[test]
fn permission_and_execution_preserve_validated_mode() {
    let handler = DelegateCliNativeToolHandler::new(Arc::new(Port));
    let input = handler
        .validate(&json!({
            "target": "codex_cli",
            "mode": "edit",
            "task": "Prepare an isolated patch"
        }))
        .expect("input");
    let permission = handler.permission_request(&input, &eligibility());
    assert_eq!(permission.action, Action::new("delegation.edit"));
    let result = handler.execute(input, execution());
    assert_eq!(result.status, NativeToolResultStatus::Succeeded);
    assert_eq!(result.output, Some(json!({"operation": "delegation.edit"})));
}
