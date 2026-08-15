use super::*;
use crate::contexts::agent_runtime::application::native_tools::NativeToolResultStatus;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Default)]
struct RecordingPort {
    calls: std::sync::Mutex<Vec<String>>,
}

impl SubagentPort for RecordingPort {
    fn execute_subagent(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        self.calls
            .lock()
            .expect("calls")
            .push(request.input.input_hash.clone());
        NativeToolResultEnvelope {
            contract_version: NATIVE_TOOL_CONTRACT_VERSION,
            status: NativeToolResultStatus::Succeeded,
            output: Some(json!({ "summary": "done" })),
            error_code: None,
            safe_error: None,
            truncated: false,
            metadata: BTreeMap::new(),
        }
    }
}

fn handler() -> SubagentNativeToolHandler {
    SubagentNativeToolHandler::new(Arc::new(RecordingPort::default()))
}

fn context(mode: NativeToolExecutionMode, workspace: Option<&str>) -> ToolEligibilityContext {
    ToolEligibilityContext {
        agent_id: "onepiece".to_owned(),
        session_id: "session-1".to_owned(),
        generation_id: "generation-1".to_owned(),
        canonical_workspace: workspace.map(PathBuf::from),
        execution_mode: mode,
        readiness: BTreeMap::from([("delegate_subagent".to_owned(), None)]),
    }
}

#[test]
fn an_eligible_execute_mode_session_with_a_workspace_receives_the_tool() {
    assert_eq!(
        handler().eligibility(&context(NativeToolExecutionMode::Execute, Some("C:/work"))),
        ToolEligibility::Eligible
    );
}

/// Plan mode is read-only exploration the parent already performs; a child there would move the
/// same work behind a boundary the user cannot watch.
#[test]
fn plan_mode_is_ineligible() {
    assert_eq!(
        handler().eligibility(&context(NativeToolExecutionMode::Plan, Some("C:/work"))),
        ToolEligibility::Ineligible {
            reason: NativeToolReadinessReasonCode::PolicyUnavailable
        }
    );
}

/// A child explores a workspace. Without one, every read-only tool it has would fail on the first
/// call, so the tool is withheld rather than offered and then failing.
#[test]
fn a_session_without_a_workspace_is_ineligible() {
    assert_eq!(
        handler().eligibility(&context(NativeToolExecutionMode::Execute, None)),
        ToolEligibility::Ineligible {
            reason: NativeToolReadinessReasonCode::PolicyUnavailable
        }
    );
}

#[test]
fn an_unready_backend_is_ineligible() {
    let mut unready = context(NativeToolExecutionMode::Execute, Some("C:/work"));
    unready.readiness.insert(
        "delegate_subagent".to_owned(),
        Some(NativeToolReadinessReasonCode::Disabled),
    );
    assert_eq!(
        handler().eligibility(&unready),
        ToolEligibility::Ineligible {
            reason: NativeToolReadinessReasonCode::Disabled
        }
    );

    let mut absent = context(NativeToolExecutionMode::Execute, Some("C:/work"));
    absent.readiness.remove("delegate_subagent");
    assert_eq!(
        handler().eligibility(&absent),
        ToolEligibility::Ineligible {
            reason: NativeToolReadinessReasonCode::BackendUnavailable
        }
    );
}

#[test]
fn a_valid_task_validates_and_binds_to_its_hash() {
    let validated = handler()
        .validate(&json!({ "task": "Find every caller of resolve_tool_catalog." }))
        .expect("valid task");

    assert_eq!(validated.operation, NativeToolOperation::SubagentDelegate);
    assert_eq!(validated.resource.kind, ToolResourceKind::Subagent);
    assert!(validated
        .resource
        .canonical_id
        .starts_with("subagent/task/"));
    assert!(validated.resource.attributes.contains_key("task_hash"));
    assert_eq!(validated.input_hash.len(), 64);
}

#[test]
fn malformed_input_is_rejected() {
    let handler = handler();
    let cases = vec![
        json!("not an object"),
        json!({}),
        json!({ "task": "" }),
        json!({ "task": "   " }),
        json!({ "task": 7 }),
        // A scope argument would let the caller aim the child somewhere the runtime did not pick.
        json!({ "task": "ok", "workspace": "C:/elsewhere" }),
        json!({ "task": "ok", "tools": ["shell"] }),
        json!({ "task": "ok", "change_files": "yes" }),
        json!({ "task": "x".repeat(MAX_SUBAGENT_TASK_CHARS + 1) }),
    ];
    for input in cases {
        assert!(
            handler.validate(&input).is_err(),
            "expected rejection for {input}"
        );
    }

    handler
        .validate(&json!({ "task": "x".repeat(MAX_SUBAGENT_TASK_CHARS) }))
        .expect("a task exactly at the bound is accepted");
}

#[test]
fn the_schema_accepts_only_a_task_and_the_change_flag() {
    let definition = handler().definition().clone();
    assert_eq!(definition.name, "delegate_subagent");
    assert_eq!(definition.input_schema["required"], json!(["task"]));
    assert_eq!(definition.input_schema["additionalProperties"], false);
    let properties = definition.input_schema["properties"]
        .as_object()
        .expect("properties");
    assert_eq!(properties.len(), 2);
    assert_eq!(properties["change_files"]["type"], "boolean");
    // Only `task` is required: a caller that says nothing about writing gets a child that cannot.
    assert_eq!(definition.input_schema["required"], json!(["task"]));
    assert!(!definition.plan_mode_compatible);
    for forbidden in [
        "workspace",
        "tools",
        "model",
        "provider",
        "session_id",
        "depth",
    ] {
        assert!(!properties.contains_key(forbidden), "{forbidden}");
    }
}

#[test]
fn starting_a_child_requests_its_own_delegation_permission() {
    let handler = handler();
    let validated = handler
        .validate(&json!({ "task": "Survey the error handling." }))
        .expect("valid");
    let request = handler.permission_request(
        &validated,
        &context(NativeToolExecutionMode::Execute, Some("C:/w")),
    );

    assert_eq!(request.action, Action::new("subagent:delegate"));
    assert_eq!(request.resource, Resource::new("subagent"));
    assert_eq!(request.operation, NativeToolOperation::SubagentDelegate);
    assert_eq!(request.input_hash, validated.input_hash);
}

#[test]
fn execution_delegates_to_the_port_with_the_validated_input() {
    let port = Arc::new(RecordingPort::default());
    let handler = SubagentNativeToolHandler::new(port.clone());
    let validated = handler
        .validate(&json!({ "task": "Trace the approval path." }))
        .expect("valid");
    let expected_hash = validated.input_hash.clone();

    let envelope = handler.execute(
        validated,
        NativeToolExecutionContext {
            call_id: "call-1".to_owned(),
            session_id: "session-1".to_owned(),
            generation_id: "generation-1".to_owned(),
            agent_id: "onepiece".to_owned(),
            canonical_workspace: Some(PathBuf::from("C:/work")),
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(60),
            cancelled: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            progress: Arc::new(NoopProgress),
        },
    );

    assert_eq!(envelope.status, NativeToolResultStatus::Succeeded);
    assert_eq!(*port.calls.lock().expect("calls"), vec![expected_hash]);
}

#[derive(Debug)]
struct NoopProgress;

impl super::super::NativeToolProgressSink for NoopProgress {
    fn publish(&self, _progress: super::super::NativeToolProgress) {}
}
