use super::initialize_negotiation::{
    build_initialize_params, initialize_and_notify, negotiate_initialize_result, supports_method,
    IndexingProgress, RuntimeReadiness,
};
use super::json_rpc_actor::{
    spawn_json_rpc_actor, JsonRpcActorLimits, JsonRpcErrorObject, ServerRequestHandler,
};
use crate::contexts::code_intelligence::domain::models::{
    DocumentSyncMode, PositionEncoding, SemanticMethod,
};
use serde_json::json;
use serde_json::Value;
use std::sync::Arc;

struct NoopServerRequestHandler;

impl ServerRequestHandler for NoopServerRequestHandler {
    fn handle(&self, _method: &str, _params: Value) -> Result<Value, JsonRpcErrorObject> {
        Err(JsonRpcErrorObject::method_not_found())
    }
}

#[test]
fn initialize_declares_only_the_client_capabilities_we_implement() {
    let params =
        build_initialize_params("file:///workspace", json!({"checkOnSave": true}), Some(42));

    assert_eq!(params["processId"], 42);
    assert_eq!(params["rootUri"], "file:///workspace");
    assert_eq!(
        params["capabilities"]["general"]["positionEncodings"],
        json!(["utf-8", "utf-16"])
    );
    assert_eq!(params["capabilities"]["workspace"]["configuration"], true);
    assert_eq!(params["capabilities"]["window"]["workDoneProgress"], true);
    assert_eq!(
        params["capabilities"]["textDocument"]["definition"]["dynamicRegistration"],
        true
    );
    assert_eq!(params["initializationOptions"]["checkOnSave"], true);
    assert!(params["capabilities"].get("workspaceEdit").is_none());
}

#[test]
fn malicious_initialization_options_cannot_override_client_workspace_scope() {
    let malicious = json!({
        "rootUri": "file:///outside",
        "workspaceFolders": [{"uri": "file:///outside", "name": "outside"}],
        "processId": 0,
        "capabilities": {"workspace": {"applyEdit": true}}
    });
    let params = build_initialize_params("file:///trusted", malicious.clone(), Some(42));

    assert_eq!(params["rootUri"], "file:///trusted");
    assert_eq!(params["workspaceFolders"][0]["uri"], "file:///trusted");
    assert_eq!(params["processId"], 42);
    assert_eq!(params["initializationOptions"], malicious);
    assert!(params["capabilities"].get("workspaceEdit").is_none());
}

#[test]
fn omitted_position_encoding_falls_back_to_utf16() {
    let negotiated =
        negotiate_initialize_result(json!({"capabilities": {}})).expect("initialize result");

    assert_eq!(negotiated.position_encoding, PositionEncoding::Utf16);
}

#[test]
fn synchronization_kind_and_options_are_normalized() {
    for (value, expected) in [
        (json!(0), DocumentSyncMode::None),
        (json!(1), DocumentSyncMode::Full),
        (json!(2), DocumentSyncMode::Incremental),
        (
            json!({"openClose": true, "change": 2}),
            DocumentSyncMode::Incremental,
        ),
    ] {
        let negotiated = negotiate_initialize_result(json!({
            "capabilities": {"textDocumentSync": value}
        }))
        .expect("sync capability");
        assert_eq!(negotiated.document_sync, expected);
    }
}

#[test]
fn unsupported_semantic_methods_are_recorded_before_any_request_is_sent() {
    let negotiated = negotiate_initialize_result(json!({
        "capabilities": {
            "positionEncoding": "utf-8",
            "definitionProvider": true,
            "referencesProvider": false,
            "hoverProvider": {"workDoneProgress": true}
        }
    }))
    .expect("capabilities");

    assert_eq!(negotiated.position_encoding, PositionEncoding::Utf8);
    assert!(supports_method(&negotiated, SemanticMethod::Definition));
    assert!(!supports_method(&negotiated, SemanticMethod::References));
    assert!(supports_method(&negotiated, SemanticMethod::Hover));
}

#[test]
fn protocol_ready_remains_separate_from_background_indexing_progress() {
    let mut readiness = RuntimeReadiness::protocol_ready();
    assert!(readiness.is_protocol_ready());
    assert_eq!(readiness.indexing_progress(), IndexingProgress::Idle);

    readiness.observe_indexing(true);
    assert!(readiness.is_protocol_ready());
    assert_eq!(readiness.indexing_progress(), IndexingProgress::Running);

    readiness.observe_indexing(false);
    assert!(readiness.is_protocol_ready());
    assert_eq!(readiness.indexing_progress(), IndexingProgress::Idle);
}

#[test]
fn malformed_initialize_results_fail_closed() {
    for value in [
        json!(null),
        json!({}),
        json!({"capabilities": "invalid"}),
        json!({"capabilities": {"textDocumentSync": 99}}),
        json!({"capabilities": {"positionEncoding": "utf-32"}}),
    ] {
        assert!(negotiate_initialize_result(value).is_err());
    }
}

#[tokio::test]
async fn successful_initialize_is_followed_by_the_initialized_notification() {
    let limits = JsonRpcActorLimits::new(8, 8, 8, 8, 8, 8).expect("actor limits");
    let (client, mut transport) = spawn_json_rpc_actor(limits, Arc::new(NoopServerRequestHandler));
    let initialize = tokio::spawn(async move {
        initialize_and_notify(
            &client,
            build_initialize_params("file:///workspace", json!({}), Some(7)),
        )
        .await
    });

    let request = transport.recv_outbound().await.expect("initialize request");
    let request: Value = serde_json::from_slice(&request).expect("request JSON");
    assert_eq!(request["method"], "initialize");
    let response = serde_json::to_vec(&json!({
        "jsonrpc": "2.0",
        "id": request["id"],
        "result": {"capabilities": {"definitionProvider": true}}
    }))
    .expect("response JSON");
    transport
        .send_inbound(response)
        .await
        .expect("initialize response");

    let initialized = transport
        .recv_outbound()
        .await
        .expect("initialized notification");
    let initialized: Value = serde_json::from_slice(&initialized).expect("notification JSON");
    assert_eq!(initialized["method"], "initialized");
    assert!(initialized.get("id").is_none());
    assert!(initialize.await.expect("initialize task").is_ok());
}
