use super::json_rpc_actor::{
    spawn_json_rpc_actor, JsonRpcActorLimits, JsonRpcError, JsonRpcErrorObject,
    JsonRpcProtocolEvent, JsonRpcRequestControl, ServerRequestHandler,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

fn limits() -> JsonRpcActorLimits {
    JsonRpcActorLimits::new(8, 8, 8, 8, 8, 4).expect("valid actor limits")
}

async fn outbound(transport: &mut super::json_rpc_actor::JsonRpcTransport) -> Value {
    let bytes = timeout(Duration::from_secs(1), transport.recv_outbound())
        .await
        .expect("outbound deadline")
        .expect("outbound message");
    serde_json::from_slice(&bytes).expect("outbound JSON")
}

async fn inbound(transport: &super::json_rpc_actor::JsonRpcTransport, message: Value) {
    transport
        .send_inbound(serde_json::to_vec(&message).expect("inbound JSON"))
        .await
        .expect("send inbound");
}

#[tokio::test]
async fn request_ids_are_monotonic_and_responses_match_out_of_order() {
    let (client, mut transport) = spawn_json_rpc_actor(limits(), Arc::new(TestHandler));
    let first = tokio::spawn({
        let client = client.clone();
        async move {
            client
                .request::<_, Value>("first", json!({"value": 1}))
                .await
        }
    });
    let first_outbound = outbound(&mut transport).await;
    let second = tokio::spawn({
        let client = client.clone();
        async move {
            client
                .request::<_, Value>("second", json!({"value": 2}))
                .await
        }
    });
    let second_outbound = outbound(&mut transport).await;

    assert_eq!(first_outbound["id"], 1);
    assert_eq!(second_outbound["id"], 2);
    inbound(
        &transport,
        json!({"jsonrpc": "2.0", "id": 2, "result": "two"}),
    )
    .await;
    inbound(
        &transport,
        json!({"jsonrpc": "2.0", "id": 1, "result": "one"}),
    )
    .await;

    assert_eq!(
        first.await.expect("first join").expect("first response"),
        "one"
    );
    assert_eq!(
        second.await.expect("second join").expect("second response"),
        "two"
    );
}

#[tokio::test]
async fn spoofed_response_ids_cannot_complete_an_unrelated_pending_request() {
    let (client, mut transport) = spawn_json_rpc_actor(limits(), Arc::new(TestHandler));
    let request = tokio::spawn({
        let client = client.clone();
        async move { client.request::<_, Value>("protected", Value::Null).await }
    });
    let original = outbound(&mut transport).await;

    inbound(
        &transport,
        json!({"jsonrpc": "2.0", "id": "1", "result": "string-spoof"}),
    )
    .await;
    inbound(
        &transport,
        json!({"jsonrpc": "2.0", "id": 999, "result": "numeric-spoof"}),
    )
    .await;

    assert_eq!(
        timeout(Duration::from_secs(1), transport.recv_protocol_event())
            .await
            .expect("malformed event deadline"),
        Some(JsonRpcProtocolEvent::MalformedMessage)
    );
    assert_eq!(
        timeout(Duration::from_secs(1), transport.recv_protocol_event())
            .await
            .expect("unknown event deadline"),
        Some(JsonRpcProtocolEvent::UnknownResponse)
    );
    assert_eq!(client.pending_count().await.expect("pending count"), 1);

    inbound(
        &transport,
        json!({"jsonrpc": "2.0", "id": original["id"], "result": "legitimate"}),
    )
    .await;
    assert_eq!(
        request.await.expect("join").expect("response"),
        "legitimate"
    );
}

#[tokio::test]
async fn duplicate_response_id_is_reported_without_affecting_later_requests() {
    let (client, mut transport) = spawn_json_rpc_actor(limits(), Arc::new(TestHandler));
    let first = tokio::spawn({
        let client = client.clone();
        async move { client.request::<_, Value>("first", Value::Null).await }
    });
    let first_outbound = outbound(&mut transport).await;
    let response = json!({"jsonrpc": "2.0", "id": first_outbound["id"], "result": "first"});
    inbound(&transport, response.clone()).await;
    assert_eq!(first.await.expect("join").expect("first response"), "first");

    inbound(&transport, response).await;
    assert_eq!(
        timeout(Duration::from_secs(1), transport.recv_protocol_event())
            .await
            .expect("duplicate event deadline"),
        Some(JsonRpcProtocolEvent::UnknownResponse)
    );

    let second = tokio::spawn({
        let client = client.clone();
        async move { client.request::<_, Value>("second", Value::Null).await }
    });
    let second_outbound = outbound(&mut transport).await;
    inbound(
        &transport,
        json!({"jsonrpc": "2.0", "id": second_outbound["id"], "result": "second"}),
    )
    .await;
    assert_eq!(
        second.await.expect("join").expect("second response"),
        "second"
    );
}

#[tokio::test]
async fn typed_request_and_response_payloads_are_converted_at_the_boundary() {
    #[derive(Serialize)]
    struct Params {
        path: String,
    }
    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Response {
        found: bool,
    }

    let (client, mut transport) = spawn_json_rpc_actor(limits(), Arc::new(TestHandler));
    let request = tokio::spawn(async move {
        client
            .request::<_, Response>(
                "typed",
                Params {
                    path: "src/lib.rs".into(),
                },
            )
            .await
    });
    let message = outbound(&mut transport).await;
    assert_eq!(message["params"]["path"], "src/lib.rs");
    inbound(
        &transport,
        json!({"jsonrpc": "2.0", "id": message["id"], "result": {"found": true}}),
    )
    .await;

    assert_eq!(
        request.await.expect("join").expect("typed response"),
        Response { found: true }
    );
}

#[tokio::test]
async fn remote_failures_and_malformed_messages_expose_only_safe_categories() {
    let (client, mut transport) = spawn_json_rpc_actor(limits(), Arc::new(TestHandler));
    let request =
        tokio::spawn(async move { client.request::<_, Value>("fails", Value::Null).await });
    let message = outbound(&mut transport).await;
    inbound(
        &transport,
        json!({
            "jsonrpc": "2.0",
            "id": message["id"],
            "error": {"code": -32001, "message": "private server detail"}
        }),
    )
    .await;
    assert_eq!(
        request.await.expect("join"),
        Err(JsonRpcError::RemoteError { code: -32001 })
    );

    transport
        .send_inbound(b"not-json private source".to_vec())
        .await
        .expect("malformed inbound");
    assert_eq!(
        timeout(Duration::from_secs(1), transport.recv_protocol_event())
            .await
            .expect("event deadline"),
        Some(JsonRpcProtocolEvent::MalformedMessage)
    );
}

#[tokio::test]
async fn inbound_notifications_are_delivered_without_blocking_requests() {
    let (_client, mut transport) = spawn_json_rpc_actor(limits(), Arc::new(TestHandler));
    inbound(
        &transport,
        json!({"jsonrpc": "2.0", "method": "window/logMessage", "params": {"type": 3}}),
    )
    .await;

    let notification = timeout(Duration::from_secs(1), transport.recv_notification())
        .await
        .expect("notification deadline")
        .expect("notification");
    assert_eq!(notification.method, "window/logMessage");
    assert_eq!(notification.params["type"], 3);
}

#[tokio::test]
async fn server_requests_use_the_bounded_handler_and_unknown_methods_return_method_not_found() {
    let (_client, mut transport) = spawn_json_rpc_actor(limits(), Arc::new(TestHandler));
    inbound(
        &transport,
        json!({"jsonrpc": "2.0", "id": "server-1", "method": "test/echo", "params": {"ok": true}}),
    )
    .await;
    let handled = outbound(&mut transport).await;
    assert_eq!(handled["id"], "server-1");
    assert_eq!(handled["result"]["ok"], true);

    inbound(
        &transport,
        json!({"jsonrpc": "2.0", "id": 99, "method": "unknown/method", "params": null}),
    )
    .await;
    let unknown = outbound(&mut transport).await;
    assert_eq!(unknown["id"], 99);
    assert_eq!(unknown["error"]["code"], -32601);
    assert!(unknown["error"].get("data").is_none());
}

#[tokio::test]
async fn pending_and_notification_queues_enforce_hard_bounds() {
    let bounded = JsonRpcActorLimits::new(8, 8, 8, 1, 8, 1).expect("limits");
    let (client, mut transport) = spawn_json_rpc_actor(bounded, Arc::new(TestHandler));
    let pending = tokio::spawn({
        let client = client.clone();
        async move { client.request::<_, Value>("pending", Value::Null).await }
    });
    let _message = outbound(&mut transport).await;

    assert_eq!(
        client.request::<_, Value>("overflow", Value::Null).await,
        Err(JsonRpcError::QueueFull)
    );
    inbound(
        &transport,
        json!({"jsonrpc": "2.0", "method": "first", "params": null}),
    )
    .await;
    inbound(
        &transport,
        json!({"jsonrpc": "2.0", "method": "second", "params": null}),
    )
    .await;
    let notification = timeout(Duration::from_secs(1), transport.recv_notification())
        .await
        .expect("notification deadline")
        .expect("first notification");
    assert_eq!(notification.method, "first");
    let event = timeout(Duration::from_secs(1), transport.recv_protocol_event())
        .await
        .expect("event deadline")
        .expect("drop event");
    assert_eq!(event, JsonRpcProtocolEvent::NotificationDropped);
    pending.abort();
}

#[tokio::test]
async fn notification_flood_is_bounded_and_does_not_starve_server_requests() {
    let bounded = JsonRpcActorLimits::new(8, 64, 8, 2, 64, 4).expect("limits");
    let (_client, mut transport) = spawn_json_rpc_actor(bounded, Arc::new(TestHandler));
    for index in 0..32 {
        inbound(
            &transport,
            json!({"jsonrpc": "2.0", "method": format!("noise/{index}"), "params": null}),
        )
        .await;
    }
    inbound(
        &transport,
        json!({"jsonrpc": "2.0", "id": "marker", "method": "test/echo", "params": {"ok": true}}),
    )
    .await;

    let marker = outbound(&mut transport).await;
    assert_eq!(marker["id"], "marker");
    assert_eq!(marker["result"]["ok"], true);
    for expected in ["noise/0", "noise/1"] {
        let notification = timeout(Duration::from_secs(1), transport.recv_notification())
            .await
            .expect("notification deadline")
            .expect("bounded notification");
        assert_eq!(notification.method, expected);
    }
    assert_eq!(
        timeout(Duration::from_secs(1), transport.recv_protocol_event())
            .await
            .expect("drop event deadline"),
        Some(JsonRpcProtocolEvent::NotificationDropped)
    );
}

#[tokio::test]
async fn abandoned_callers_are_removed_from_the_pending_map() {
    let (client, mut transport) = spawn_json_rpc_actor(limits(), Arc::new(TestHandler));
    let request = tokio::spawn({
        let client = client.clone();
        async move { client.request::<_, Value>("abandoned", Value::Null).await }
    });
    let _message = outbound(&mut transport).await;
    assert_eq!(client.pending_count().await.expect("pending count"), 1);

    request.abort();
    assert_eq!(client.pending_count().await.expect("cleaned count"), 0);
}

#[tokio::test]
async fn deadline_sends_cancel_for_the_real_request_id_and_cleans_pending_state() {
    let (client, mut transport) = spawn_json_rpc_actor(limits(), Arc::new(TestHandler));
    let control = JsonRpcRequestControl::new(
        Duration::from_millis(80),
        Duration::from_millis(20),
        Arc::new(AtomicBool::new(false)),
    )
    .expect("request control");
    let request = tokio::spawn({
        let client = client.clone();
        async move {
            client
                .request_with_control::<_, Value>("slow", Value::Null, control)
                .await
        }
    });
    let original = outbound(&mut transport).await;
    let cancellation = outbound(&mut transport).await;

    assert_eq!(original["id"], 1);
    assert_eq!(cancellation["method"], "$/cancelRequest");
    assert_eq!(cancellation["params"]["id"], 1);
    assert_eq!(request.await.expect("join"), Err(JsonRpcError::Timeout));
    assert_eq!(client.pending_count().await.expect("pending count"), 0);

    inbound(
        &transport,
        json!({"jsonrpc": "2.0", "id": 1, "result": "late"}),
    )
    .await;
    assert_eq!(
        timeout(Duration::from_secs(1), transport.recv_protocol_event())
            .await
            .expect("late response event deadline"),
        Some(JsonRpcProtocolEvent::UnknownResponse)
    );
}

#[tokio::test]
async fn generation_cancellation_stops_waiting_and_uses_the_registered_id() {
    let (client, mut transport) = spawn_json_rpc_actor(limits(), Arc::new(TestHandler));
    let cancelled = Arc::new(AtomicBool::new(false));
    let control = JsonRpcRequestControl::new(
        Duration::from_secs(1),
        Duration::from_millis(50),
        cancelled.clone(),
    )
    .expect("request control");
    let request = tokio::spawn(async move {
        client
            .request_with_control::<_, Value>("cancelled", Value::Null, control)
            .await
    });
    let original = outbound(&mut transport).await;
    cancelled.store(true, Ordering::Release);
    let cancellation = outbound(&mut transport).await;

    assert_eq!(cancellation["method"], "$/cancelRequest");
    assert_eq!(cancellation["params"]["id"], original["id"]);
    assert_eq!(request.await.expect("join"), Err(JsonRpcError::Cancelled));
}

struct TestHandler;

impl ServerRequestHandler for TestHandler {
    fn handle(&self, method: &str, params: Value) -> Result<Value, JsonRpcErrorObject> {
        if method == "test/echo" {
            Ok(params)
        } else {
            Err(JsonRpcErrorObject::method_not_found())
        }
    }
}
