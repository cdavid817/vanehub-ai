use super::*;
use crate::contexts::agent_runtime::application::{
    NativeToolExecutionMode, NativeToolProgress, NativeToolProgressSink, NativeToolResultStatus,
};
use serde_json::json;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct NoopProgress;

impl NativeToolProgressSink for NoopProgress {
    fn publish(&self, _progress: NativeToolProgress) {}
}

#[derive(Default)]
struct Port {
    operations: Mutex<Vec<NativeToolOperation>>,
}

impl ArtifactPort for Port {
    fn execute_artifact(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        self.operations
            .lock()
            .expect("operations")
            .push(request.input.operation);
        NativeToolResultEnvelope {
            contract_version: NATIVE_TOOL_CONTRACT_VERSION,
            status: NativeToolResultStatus::Succeeded,
            output: Some(json!({"ok": true})),
            error_code: None,
            safe_error: None,
            truncated: false,
            metadata: BTreeMap::new(),
        }
    }
}

fn eligibility(mode: NativeToolExecutionMode) -> ToolEligibilityContext {
    ToolEligibilityContext {
        agent_id: "onepiece".to_owned(),
        session_id: "session-1".to_owned(),
        generation_id: "generation-1".to_owned(),
        canonical_workspace: None,
        execution_mode: mode,
        readiness: BTreeMap::from([("artifact".to_owned(), None)]),
    }
}

fn execution() -> NativeToolExecutionContext {
    NativeToolExecutionContext {
        call_id: "call-1".to_owned(),
        session_id: "session-1".to_owned(),
        generation_id: "generation-1".to_owned(),
        agent_id: "onepiece".to_owned(),
        canonical_workspace: None,
        deadline: Instant::now() + Duration::from_secs(10),
        cancelled: Arc::new(AtomicBool::new(false)),
        progress: Arc::new(NoopProgress),
    }
}

#[test]
fn artifact_handler_has_fixed_schema_and_classifies_permissions_by_operation() {
    let port = Arc::new(Port::default());
    let handler = ArtifactNativeToolHandler::new(port);
    assert_eq!(handler.definition().name, "artifact");
    assert_eq!(
        handler.eligibility(&eligibility(NativeToolExecutionMode::Plan)),
        ToolEligibility::Eligible
    );

    let read = handler
        .validate(&json!({"operation": "read_text", "artifact_id": "artifact-123", "limit": 100}))
        .expect("read");
    let publish = handler
        .validate(&json!({"operation": "publish", "artifact_id": "artifact-123", "visibility": "session"}))
        .expect("publish");
    assert_eq!(
        handler
            .permission_request(&read, &eligibility(NativeToolExecutionMode::Execute))
            .action,
        Action::new("artifact.read")
    );
    assert_eq!(
        handler
            .permission_request(&publish, &eligibility(NativeToolExecutionMode::Execute))
            .action,
        Action::new("artifact.publish")
    );
    assert!(handler
        .validate(&json!({"operation": "metadata", "artifact_id": "../secret"}))
        .is_err());
}

#[test]
fn artifact_handler_delegates_only_validated_requests_to_the_port() {
    let port = Arc::new(Port::default());
    let handler = ArtifactNativeToolHandler::new(port.clone());
    let input = handler
        .validate(&json!({"operation": "list", "limit": 10}))
        .expect("input");
    let result = handler.execute(input, execution());

    assert_eq!(result.status, NativeToolResultStatus::Succeeded);
    assert_eq!(
        port.operations.lock().expect("operations").as_slice(),
        &[NativeToolOperation::ArtifactRead]
    );
}
