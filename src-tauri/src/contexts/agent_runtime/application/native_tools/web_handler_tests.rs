use super::*;
use crate::contexts::agent_runtime::application::{
    NativeToolProgress, NativeToolProgressSink, NativeToolRegistry, NativeToolResultStatus,
};
use serde_json::json;
use std::collections::BTreeSet;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug)]
struct NoopProgress;

impl NativeToolProgressSink for NoopProgress {
    fn publish(&self, _progress: NativeToolProgress) {}
}

#[derive(Default)]
struct RecordingWebPort {
    operations: Mutex<Vec<NativeToolOperation>>,
}

impl WebResearchPort for RecordingWebPort {
    fn execute_web(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        self.operations
            .lock()
            .expect("operations lock")
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

fn eligibility(
    mode: NativeToolExecutionMode,
    readiness: BTreeMap<String, Option<NativeToolReadinessReasonCode>>,
) -> ToolEligibilityContext {
    ToolEligibilityContext {
        agent_id: "onepiece".to_string(),
        session_id: "session-1".to_string(),
        generation_id: "generation-1".to_string(),
        canonical_workspace: None,
        execution_mode: mode,
        readiness,
    }
}

fn execution() -> NativeToolExecutionContext {
    NativeToolExecutionContext {
        call_id: "call-1".to_string(),
        session_id: "session-1".to_string(),
        generation_id: "generation-1".to_string(),
        agent_id: "onepiece".to_string(),
        canonical_workspace: None,
        deadline: Instant::now() + Duration::from_secs(10),
        cancelled: Arc::new(AtomicBool::new(false)),
        progress: Arc::new(NoopProgress),
    }
}

#[test]
fn search_and_fetch_have_independent_readiness_and_are_excluded_in_plan_mode() {
    let port = Arc::new(RecordingWebPort::default());
    let search = WebSearchNativeToolHandler::new(port.clone());
    let fetch = WebFetchNativeToolHandler::new(port);
    let readiness = BTreeMap::from([
        ("web_search".to_string(), None),
        (
            "web_fetch".to_string(),
            Some(NativeToolReadinessReasonCode::BackendUnavailable),
        ),
    ]);

    assert_eq!(
        search.eligibility(&eligibility(
            NativeToolExecutionMode::Execute,
            readiness.clone()
        )),
        ToolEligibility::Eligible
    );
    assert_eq!(
        fetch.eligibility(&eligibility(
            NativeToolExecutionMode::Execute,
            readiness.clone()
        )),
        ToolEligibility::Ineligible {
            reason: NativeToolReadinessReasonCode::BackendUnavailable
        }
    );
    assert_eq!(
        search.eligibility(&eligibility(NativeToolExecutionMode::Plan, readiness)),
        ToolEligibility::Ineligible {
            reason: NativeToolReadinessReasonCode::PolicyUnavailable
        }
    );
}

#[test]
fn fixed_schemas_validate_and_bind_permissions_to_canonical_resources() {
    let port = Arc::new(RecordingWebPort::default());
    let search = WebSearchNativeToolHandler::new(port.clone());
    let fetch = WebFetchNativeToolHandler::new(port);
    assert_eq!(search.definition().name, "web_search");
    assert_eq!(fetch.definition().name, "web_fetch");

    let search_input = search
        .validate(&json!({"query": "rust tauri", "count": 5, "safe_search": "moderate"}))
        .expect("search input");
    assert_eq!(search_input.operation, NativeToolOperation::WebSearch);
    assert!(search_input
        .resource
        .canonical_id
        .starts_with("web-search/sha256:"));
    assert_eq!(
        search
            .permission_request(
                &search_input,
                &eligibility(NativeToolExecutionMode::Execute, BTreeMap::new())
            )
            .action,
        Action::new("web.search")
    );

    let fetch_input = fetch
        .validate(&json!({
            "url": "https://Example.com:443/docs#section",
            "max_text_chars": 12000,
            "persist_binary": false
        }))
        .expect("fetch input");
    assert_eq!(fetch_input.operation, NativeToolOperation::WebFetch);
    assert_eq!(
        fetch_input.resource.canonical_id,
        "https://example.com/docs"
    );
    assert_eq!(
        fetch
            .permission_request(
                &fetch_input,
                &eligibility(NativeToolExecutionMode::Execute, BTreeMap::new())
            )
            .action,
        Action::new("web.fetch")
    );

    assert!(search.validate(&json!({"query": " padded "})).is_err());
    assert!(search
        .validate(&json!({"query": "valid", "unknown": true}))
        .is_err());
    assert!(fetch.validate(&json!({"url": "file:///secret"})).is_err());
    assert!(fetch
        .validate(&json!({"url": "https://user:pass@example.com"}))
        .is_err());
    assert!(fetch
        .validate(&json!({"url": "https://example.com:8443"}))
        .is_err());
}

#[test]
fn handlers_delegate_only_validated_operations_to_the_shared_web_port() {
    let port = Arc::new(RecordingWebPort::default());
    let search = WebSearchNativeToolHandler::new(port.clone());
    let fetch = WebFetchNativeToolHandler::new(port.clone());
    let search_input = search
        .validate(&json!({"query": "onepiece"}))
        .expect("search input");
    let fetch_input = fetch
        .validate(&json!({"url": "https://example.com"}))
        .expect("fetch input");

    assert_eq!(
        search.execute(search_input, execution()).status,
        NativeToolResultStatus::Succeeded
    );
    assert_eq!(
        fetch.execute(fetch_input, execution()).status,
        NativeToolResultStatus::Succeeded
    );
    assert_eq!(
        port.operations.lock().expect("operations lock").as_slice(),
        &[
            NativeToolOperation::WebSearch,
            NativeToolOperation::WebFetch
        ]
    );
}

#[test]
fn fixed_web_handlers_register_together_and_keep_onepiece_and_plan_gates() {
    let registry = NativeToolRegistry::try_new(web_native_tool_handlers(Arc::new(
        RecordingWebPort::default(),
    )))
    .expect("fixed Web handler names should be unique");
    let readiness = BTreeMap::from([
        ("web_search".to_string(), None),
        ("web_fetch".to_string(), None),
    ]);
    let execute = eligibility(NativeToolExecutionMode::Execute, readiness.clone());
    let names = registry
        .eligible_definitions(&execute)
        .into_iter()
        .map(|definition| definition.name)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        names,
        BTreeSet::from(["web_fetch".to_string(), "web_search".to_string()])
    );

    let mut custom = execute.clone();
    custom.agent_id = "custom-api-agent".to_string();
    assert!(registry.eligible_definitions(&custom).is_empty());
    assert!(registry
        .eligible_definitions(&eligibility(NativeToolExecutionMode::Plan, readiness))
        .is_empty());
}
