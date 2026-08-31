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

/// Where Playwright keeps the browsers it downloads.
///
/// Checked separately from the package, because the package being installed is what `npm ci` gives
/// you and a browser is what `npx playwright install` gives you — and the difference between them is
/// the difference between this test running and this test returning immediately. The `Rust` job had
/// the first and not the second, so the guard below passed and the test asserted nothing on every
/// run for as long as it has existed.
fn playwright_browser_cache() -> Option<std::path::PathBuf> {
    if let Ok(explicit) = std::env::var("PLAYWRIGHT_BROWSERS_PATH") {
        // `0` means "beside the package" rather than a path. Treated as unknown rather than guessed
        // at: a wrong guess here reports a browser that is not there.
        if explicit == "0" {
            return None;
        }
        return Some(std::path::PathBuf::from(explicit));
    }
    #[cfg(windows)]
    let base = std::env::var("LOCALAPPDATA")
        .ok()
        .map(std::path::PathBuf::from);
    #[cfg(target_os = "macos")]
    let base = std::env::var("HOME")
        .ok()
        .map(|home| std::path::PathBuf::from(home).join("Library/Caches"));
    #[cfg(all(unix, not(target_os = "macos")))]
    let base = std::env::var("HOME")
        .ok()
        .map(|home| std::path::PathBuf::from(home).join(".cache"));
    base.map(|base| base.join("ms-playwright"))
}

/// Whether a browser this test could drive is actually present.
fn a_browser_is_installed() -> bool {
    playwright_browser_cache().is_some_and(|cache| {
        std::fs::read_dir(cache).is_ok_and(|mut entries| {
            entries.any(|entry| {
                entry.is_ok_and(|entry| entry.file_name().to_string_lossy().starts_with("chromium"))
            })
        })
    })
}

/// Says that this test did not run, and what would make it run.
///
/// A bare `return` reports the same thing a pass reports, and the longer it does so the more
/// coverage the suite appears to have. This does not fail the run — a machine with no browser is not
/// a broken machine — but it leaves a sentence behind that names the prerequisite, so a green run
/// can be told apart from a run that checked something.
fn skip(reason: &str) {
    eprintln!(
        "SKIPPED real_playwright_worker_bounds_page_operations_handoff_and_artifact_bytes: {reason}. Run `npx playwright install chromium` to exercise it."
    );
}

#[test]
fn real_playwright_worker_bounds_page_operations_handoff_and_artifact_bytes() {
    if !crate::platform::process::command_exists("node", Duration::from_secs(2)) {
        skip("no `node` on PATH");
        return;
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    if !root.join("node_modules/playwright/package.json").is_file() {
        skip("the `playwright` package is not installed");
        return;
    }
    if !a_browser_is_installed() {
        skip("the `playwright` package is installed but no Chromium browser is");
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
        // A browser is installed and the context still could not be created, so this is a failure
        // rather than an absence. Returning green here is what let the missing-browser case hide:
        // the two were the same early return, and one of them is a defect.
        let _ = worker.shutdown(SHUTDOWN_BUDGET);
        panic!(
            "a browser is installed but context.create failed: {:?}",
            context.error_code
        );
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
    // Reaping a real Chromium tree is not a request, so it does not get the request budget. Closing
    // stdin, waiting out the grace phase, force-killing and then waiting for every child to be
    // reaped is disk- and scheduler-bound, and under the full native suite -- hundreds of tests,
    // each with its own process or database fixture -- thirty seconds is not enough. That is a
    // property of the machine, not of the worker, and the assertion that matters is that the tree
    // does go away rather than that it goes away quickly.
    worker.shutdown(SHUTDOWN_BUDGET).expect("worker shutdown");
}

/// How long a real browser tree gets to disappear. See the call site for why it is not the
/// request timeout.
const SHUTDOWN_BUDGET: Duration = Duration::from_secs(120);

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
