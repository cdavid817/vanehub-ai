use super::json_rpc_actor::{spawn_json_rpc_actor, JsonRpcActorLimits};
use super::lsp_server_requests::{
    LspClientRequestLimits, LspServerRequestHandler, LspServerRequestSnapshot,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::time::{timeout, Duration};

fn actor_limits() -> JsonRpcActorLimits {
    JsonRpcActorLimits::new(8, 8, 8, 8, 8, 4).expect("actor limits")
}

fn handler_limits() -> LspClientRequestLimits {
    LspClientRequestLimits::new(4, 4, 2, 1024).expect("handler limits")
}

fn handler() -> Arc<LspServerRequestHandler> {
    Arc::new(
        LspServerRequestHandler::new(
            BTreeMap::from([
                ("rust-analyzer".into(), json!({"checkOnSave": true})),
                ("typescript".into(), json!({"preferences": {}})),
            ]),
            handler_limits(),
        )
        .expect("handler"),
    )
}

async fn request(
    transport: &mut super::json_rpc_actor::JsonRpcTransport,
    id: Value,
    method: &str,
    params: Value,
) -> Value {
    transport
        .send_inbound(
            serde_json::to_vec(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": method,
                "params": params,
            }))
            .expect("request JSON"),
        )
        .await
        .expect("send request");
    let bytes = timeout(Duration::from_secs(1), transport.recv_outbound())
        .await
        .expect("response deadline")
        .expect("response");
    serde_json::from_slice(&bytes).expect("response JSON")
}

#[tokio::test]
async fn workspace_configuration_returns_bounded_values_in_item_order() {
    let handler = handler();
    let (_client, mut transport) = spawn_json_rpc_actor(actor_limits(), handler);
    let response = request(
        &mut transport,
        json!(1),
        "workspace/configuration",
        json!({"items": [
            {"section": "typescript"},
            {"section": "missing"},
            {"section": "rust-analyzer"}
        ]}),
    )
    .await;

    assert_eq!(response["result"][0], json!({"preferences": {}}));
    assert_eq!(response["result"][1], Value::Null);
    assert_eq!(response["result"][2], json!({"checkOnSave": true}));
}

#[tokio::test]
async fn dynamic_registrations_can_be_added_and_removed_atomically() {
    let handler = handler();
    let (_client, mut transport) = spawn_json_rpc_actor(actor_limits(), handler.clone());
    let registered = request(
        &mut transport,
        json!(2),
        "client/registerCapability",
        json!({"registrations": [
            {"id": "watch", "method": "workspace/didChangeWatchedFiles"},
            {"id": "config", "method": "workspace/didChangeConfiguration"}
        ]}),
    )
    .await;
    assert_eq!(registered["result"], Value::Null);
    assert_eq!(handler.snapshot().expect("snapshot").registration_count, 2);

    let unregistered = request(
        &mut transport,
        json!(3),
        "client/unregisterCapability",
        json!({"unregisterations": [{"id": "watch", "method": "workspace/didChangeWatchedFiles"}]}),
    )
    .await;
    assert_eq!(unregistered["result"], Value::Null);
    assert_eq!(
        handler.snapshot().expect("snapshot"),
        LspServerRequestSnapshot {
            registration_count: 1,
            progress_token_count: 0,
        }
    );
}

#[tokio::test]
async fn work_done_progress_tokens_are_idempotent_and_bounded() {
    let handler = handler();
    let (_client, mut transport) = spawn_json_rpc_actor(actor_limits(), handler.clone());
    for (id, token) in [(4, json!("index")), (5, json!("index")), (6, json!(2))] {
        let response = request(
            &mut transport,
            json!(id),
            "window/workDoneProgress/create",
            json!({"token": token}),
        )
        .await;
        assert_eq!(response["result"], Value::Null);
    }
    let overflow = request(
        &mut transport,
        json!(7),
        "window/workDoneProgress/create",
        json!({"token": "third"}),
    )
    .await;

    assert_eq!(overflow["error"]["code"], -32001);
    assert_eq!(
        handler.snapshot().expect("snapshot").progress_token_count,
        2
    );
}

#[tokio::test]
async fn show_message_is_non_interactive_and_workspace_edits_are_rejected() {
    let handler = handler();
    let (_client, mut transport) = spawn_json_rpc_actor(actor_limits(), handler);
    let shown = request(
        &mut transport,
        json!(8),
        "window/showMessageRequest",
        json!({"type": 3, "message": "choose", "actions": [{"title": "Yes"}]}),
    )
    .await;
    assert_eq!(shown["result"], Value::Null);

    let edit = request(
        &mut transport,
        json!(9),
        "workspace/applyEdit",
        json!({"edit": {"changes": {"file:///workspace/main.rs": []}}}),
    )
    .await;
    assert_eq!(edit["result"]["applied"], false);
    assert_eq!(edit["result"]["failureReason"], "read_only_client");
}

#[tokio::test]
async fn invalid_parameters_and_unknown_methods_return_safe_standard_errors() {
    let handler = handler();
    let (_client, mut transport) = spawn_json_rpc_actor(actor_limits(), handler);
    let invalid = request(
        &mut transport,
        json!(10),
        "workspace/configuration",
        json!({"items": "not-an-array"}),
    )
    .await;
    assert_eq!(invalid["error"]["code"], -32602);
    assert!(invalid["error"].get("data").is_none());

    let unknown = request(
        &mut transport,
        json!(11),
        "custom/privateMethod",
        json!({"source": "private"}),
    )
    .await;
    assert_eq!(unknown["error"]["code"], -32601);
    assert!(unknown["error"].get("data").is_none());
}
