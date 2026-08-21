use super::*;
use crate::contexts::browser_automation::application::BrowserSidecarTransport;
use std::io::Cursor;

#[test]
fn bounded_reader_accepts_one_json_line_and_rejects_overflow() {
    let mut reader = Cursor::new(b"{\"ok\":true}\n".to_vec());
    assert_eq!(
        read_bounded_line(&mut reader, 64),
        Ok(Some(b"{\"ok\":true}".to_vec()))
    );
    let mut oversized = Cursor::new(b"12345\n".to_vec());
    assert_eq!(
        read_bounded_line(&mut oversized, 4),
        Err(BrowserSidecarError::MessageTooLarge)
    );
}

#[test]
fn real_node_fixture_completes_handshake_health_and_owned_shutdown() {
    if !crate::platform::process::command_exists("node", Duration::from_secs(2)) {
        return;
    }
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("scripts")
        .join("onepiece-playwright-sidecar.mjs");
    let mut process = PlaywrightSidecarProcess::spawn("node", &script, 1024 * 1024)
        .expect("Node fixture should start");
    let limits = BrowserSidecarLimits::default();
    let handshake = BrowserSidecarRequest {
        protocol_version: BROWSER_SIDECAR_PROTOCOL_VERSION,
        request_id: "handshake-1".to_string(),
        method: "handshake".to_string(),
        params: serde_json::json!({"protocol_version": BROWSER_SIDECAR_PROTOCOL_VERSION}),
    };
    let response = process
        .request(&handshake, limits)
        .expect("handshake response");
    assert_eq!(response.request_id, "handshake-1");
    assert!(response.ok);
    process
        .shutdown(Duration::from_secs(5))
        .expect("owned process tree should stop");
}

#[test]
fn real_playwright_worker_bounds_page_operations_handoff_and_artifact_bytes() {
    if !crate::platform::process::command_exists("node", Duration::from_secs(2)) {
        return;
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    if !root.join("node_modules/playwright/package.json").is_file() {
        return;
    }
    let script = root.join("scripts/onepiece-playwright-sidecar.mjs");
    let environment = BTreeMap::from([("VANEHUB_BROWSER_HEADLESS".to_owned(), "1".to_owned())]);
    let mut worker = PlaywrightSidecarProcess::spawn_with_environment(
        "node",
        &script,
        1024 * 1024,
        &environment,
    )
    .expect("worker");
    let limits = BrowserSidecarLimits {
        request_timeout: Duration::from_secs(30),
        ..BrowserSidecarLimits::default()
    };
    let context = worker_request(
        &mut worker,
        limits,
        "create",
        "context.create",
        serde_json::json!({
            "owner": {"session_id": "session-1", "generation_id": "generation-1"},
            "policy": {
                "incognito": true,
                "persistent": false,
                "import_cookies": false,
                "extensions": false,
                "http_credentials": false,
                "max_lifetime_ms": 60000,
                "max_pages": 1,
                "max_download_bytes": 1048576,
                "max_event_count": 100
            }
        }),
    );
    if !context.ok {
        let _ = worker.shutdown(Duration::from_secs(5));
        return;
    }
    let context_id = context
        .result
        .as_ref()
        .and_then(|value| value.get("context_id"))
        .and_then(serde_json::Value::as_str)
        .expect("context id");
    let evaluated = page_request(
        &mut worker,
        limits,
        "evaluate",
        context_id,
        "page.evaluate",
        None,
        serde_json::json!({
            "expression": "(()=>{document.body.innerHTML='<button>Continue</button><p id=content>visible text</p>';return true})()"
        }),
    );
    assert!(evaluated.ok);
    let page_id = evaluated
        .result
        .as_ref()
        .and_then(|value| value.get("page_id"))
        .and_then(serde_json::Value::as_str)
        .expect("page id");
    let inspected = page_request(
        &mut worker,
        limits,
        "inspect",
        context_id,
        "page.inspect",
        Some(page_id),
        serde_json::json!({}),
    );
    assert!(inspected.ok);
    let extracted = page_request(
        &mut worker,
        limits,
        "extract",
        context_id,
        "page.extract",
        Some(page_id),
        serde_json::json!({"selector": "#content"}),
    );
    assert!(extracted.ok);
    assert!(extracted
        .result
        .as_ref()
        .and_then(|value| value.pointer("/payload/text"))
        .is_some_and(|value| value == "visible text"));
    let screenshot = page_request(
        &mut worker,
        limits,
        "screenshot",
        context_id,
        "page.screenshot",
        Some(page_id),
        serde_json::json!({"full_page": false}),
    );
    assert!(screenshot.ok);
    assert!(screenshot
        .result
        .as_ref()
        .and_then(|value| value.pointer("/payload/bytes_base64"))
        .and_then(serde_json::Value::as_str)
        .is_some_and(|value| !value.is_empty()));
    assert!(
        page_request(
            &mut worker,
            limits,
            "handoff",
            context_id,
            "page.handoff",
            Some(page_id),
            serde_json::json!({}),
        )
        .ok
    );
    assert!(
        !page_request(
            &mut worker,
            limits,
            "paused",
            context_id,
            "page.inspect",
            Some(page_id),
            serde_json::json!({}),
        )
        .ok
    );
    assert!(
        page_request(
            &mut worker,
            limits,
            "resume",
            context_id,
            "page.resume",
            Some(page_id),
            serde_json::json!({}),
        )
        .ok
    );
    worker
        // A real Chromium context can need more than five seconds to release its Windows Job
        // Object when the full native suite is running many process fixtures concurrently.
        .shutdown(Duration::from_secs(15))
        .expect("worker shutdown");
}

fn worker_request(
    worker: &mut PlaywrightSidecarProcess,
    limits: BrowserSidecarLimits,
    request_id: &str,
    method: &str,
    params: serde_json::Value,
) -> BrowserSidecarResponse {
    worker
        .request(
            &BrowserSidecarRequest {
                protocol_version: BROWSER_SIDECAR_PROTOCOL_VERSION,
                request_id: request_id.to_owned(),
                method: method.to_owned(),
                params,
            },
            limits,
        )
        .expect("worker response")
}

fn page_request(
    worker: &mut PlaywrightSidecarProcess,
    limits: BrowserSidecarLimits,
    request_id: &str,
    context_id: &str,
    method: &str,
    page_id: Option<&str>,
    input: serde_json::Value,
) -> BrowserSidecarResponse {
    worker_request(
        worker,
        limits,
        request_id,
        method,
        serde_json::json!({
            "context_id": context_id,
            "input": {"page_id": page_id, "input": input}
        }),
    )
}
