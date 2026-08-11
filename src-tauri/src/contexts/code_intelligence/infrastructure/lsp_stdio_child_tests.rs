use super::json_rpc_actor::{JsonRpcActorLimits, JsonRpcErrorObject, ServerRequestHandler};
use super::lsp_framing::{FrameLimits, LspFrameError};
use super::lsp_stdio_child::{LspStdioError, ManagedLspStdio};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

fn fixture_args(mode: &str) -> Vec<String> {
    vec![
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/lsp_stdio_server.cjs")
            .to_string_lossy()
            .into_owned(),
        mode.into(),
    ]
}

fn actor_limits() -> JsonRpcActorLimits {
    JsonRpcActorLimits::new(8, 8, 8, 8, 8, 4).expect("actor limits")
}

fn spawn(mode: &str, payload_limit: usize, stderr_limit: usize) -> ManagedSpawn {
    let (client, events, process) = ManagedLspStdio::spawn(
        "node",
        &fixture_args(mode),
        &BTreeMap::new(),
        FrameLimits::new(128, payload_limit).expect("frame limits"),
        stderr_limit,
        actor_limits(),
        Arc::new(RejectServerRequests),
    )
    .expect("managed LSP stdio");
    ManagedSpawn {
        client,
        _events: events,
        process,
    }
}

struct ManagedSpawn {
    client: super::json_rpc_actor::JsonRpcClient,
    _events: super::json_rpc_actor::JsonRpcEvents,
    process: ManagedLspStdio,
}

#[tokio::test]
async fn managed_stdio_round_trip_drains_stderr_and_reaps_the_child() {
    let mut spawned = spawn("echo", 256, 32);
    let response = spawned
        .client
        .request::<_, Value>("test/ping", json!({"ping": true}))
        .await
        .expect("response");
    assert_eq!(response, json!({"pong": true}));

    let exit = spawned
        .process
        .wait_until(Instant::now() + Duration::from_secs(5))
        .await
        .expect("process wait")
        .expect("process exited");
    assert!(exit.status.success());
    assert!(exit.stderr.observed_bytes >= 4096);
    assert!(exit.stderr.truncated);
}

#[tokio::test]
async fn managed_stdio_fixture_accepts_first_definition_after_initialize() {
    let mut spawned = spawn("lsp-semantic", 4 * 1024, 256);
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let root_uri = url::Url::from_directory_path(workspace).expect("workspace URI");
    let document_uri = url::Url::from_file_path(workspace.join("Cargo.toml"))
        .expect("document URI")
        .to_string();

    let initialize = spawned
        .client
        .request::<_, Value>(
            "initialize",
            json!({
                "processId": null,
                "rootUri": root_uri,
                "capabilities": {},
            }),
        )
        .await
        .expect("initialize response");
    assert_eq!(initialize["capabilities"]["definitionProvider"], true);

    spawned
        .client
        .notify("initialized", json!({}))
        .await
        .expect("initialized notification");
    spawned
        .client
        .notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": document_uri,
                    "languageId": "toml",
                    "version": 1,
                    "text": "[package]\n",
                }
            }),
        )
        .await
        .expect("didOpen notification");

    let definitions = spawned
        .client
        .request::<_, Value>(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": document_uri },
                "position": { "line": 0, "character": 0 },
            }),
        )
        .await
        .expect("first semantic response");
    assert_eq!(definitions[0]["uri"], document_uri);

    let shutdown = spawned
        .client
        .request::<_, Value>("shutdown", Value::Null)
        .await
        .expect("shutdown response");
    assert!(shutdown.is_null());
    spawned
        .client
        .notify("exit", Value::Null)
        .await
        .expect("exit notification");
    let exit = spawned
        .process
        .wait_until(Instant::now() + Duration::from_secs(5))
        .await
        .expect("process wait")
        .expect("process exited");
    assert!(exit.status.success());
}

#[tokio::test]
async fn protocol_limit_failure_terminates_and_reaps_the_managed_process() {
    let mut spawned = spawn("oversized", 16, 32);
    let error = spawned
        .process
        .wait_until(Instant::now() + Duration::from_secs(5))
        .await
        .expect_err("oversized frame fails");

    assert_eq!(
        error,
        LspStdioError::Protocol(LspFrameError::PayloadTooLarge)
    );
    assert!(spawned.process.is_reaped());
}

#[tokio::test]
async fn unexpected_child_exit_stops_pending_requests_and_protocol_tasks() {
    let mut spawned = spawn("exit", 256, 32);
    let client = spawned.client.clone();
    let pending = tokio::spawn(async move {
        client
            .request::<_, Value>("test/pending", Value::Null)
            .await
    });
    let exit = spawned
        .process
        .wait_until(Instant::now() + Duration::from_secs(5))
        .await
        .expect("wait")
        .expect("exit");

    assert!(exit.status.success());
    assert!(pending.await.expect("join").is_err());
    assert!(spawned.process.protocol_tasks_finished());
}

struct RejectServerRequests;

impl ServerRequestHandler for RejectServerRequests {
    fn handle(&self, _method: &str, _params: Value) -> Result<Value, JsonRpcErrorObject> {
        Err(JsonRpcErrorObject::method_not_found())
    }
}
