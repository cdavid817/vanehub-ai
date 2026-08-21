//! The Claude Code `PreToolUse` hook wrapper (`claude-code-permission-hook`; design.md D4).
//! Claude Code spawns this as a fresh process on every matched tool call, so it is deliberately
//! minimal: only `serde`/`serde_json`/`dirs` (all already workspace dependencies), a hand-rolled
//! HTTP/1.1 client over `std::net::TcpStream` instead of `reqwest`, and no async runtime at all.
//! It never links `vanehub_ai_lib` — the discovery-file shape and path (`VaneHub` subdir,
//! `permission-hook.json`) are duplicated from
//! `contexts/permissions/infrastructure/hook_bridge_discovery.rs` by hand and must be kept in
//! sync there.
//!
//! In hook mode (no arguments) it always exits 0 with a `hookSpecificOutput` JSON body on
//! stdout — Claude Code's documented contract encodes the actual allow/deny decision in that
//! JSON, not in the process exit code. The human-invoked `--uninstall` mode (`uninstall.rs`)
//! uses conventional exit codes instead.

mod uninstall;

use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Above the native decision pipeline's own 300s pending-approval sweep, below Claude Code's
/// 600s hook ceiling (design.md D8) — a legitimate pending `Ask` always resolves through the
/// sweep well before this fires; this is only a safety net against a request that never got a
/// response through any channel at all.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(320);
const MANAGED_SCOPE_ENV: &str = "VANEHUB_PERMISSION_HOOK_SCOPE";
const MANAGED_SCOPE_VALUE: &str = "managed";

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match arguments.as_slice() {
        [] => print_hook_output(&run(DEFAULT_TIMEOUT)),
        [flag] if flag == "--uninstall" => match uninstall::run() {
            Ok(message) => println!("{message}"),
            Err(message) => {
                eprintln!("vanehub-permission-hook --uninstall failed: {message}");
                std::process::exit(1);
            }
        },
        // Never fall back to hook mode on unrecognized arguments: hook mode blocks on stdin,
        // which for a human typo means a silently hung terminal instead of an error.
        _ => {
            eprintln!("usage: vanehub-permission-hook [--uninstall]");
            std::process::exit(2);
        }
    }
}

enum Decision {
    PassThrough,
    Allow,
    Deny(&'static str),
}

#[derive(Deserialize)]
struct HookRequest {
    tool_name: String,
    #[serde(default)]
    tool_input: serde_json::Value,
    session_id: String,
    cwd: String,
}

#[derive(Serialize)]
struct WrapperRequest<'a> {
    tool_name: &'a str,
    tool_input: &'a serde_json::Value,
    session_id: &'a str,
    cwd: &'a str,
}

#[derive(Deserialize)]
struct Discovery {
    port: u16,
    token: String,
}

struct HttpResponse {
    status: u16,
    body: String,
}

fn run(timeout: Duration) -> Decision {
    let mut input = String::new();
    if io::stdin().read_to_string(&mut input).is_err() {
        return Decision::Deny("could not read the tool-use request from stdin");
    }
    let Some(request) = parse_hook_request(&input) else {
        return Decision::Deny("could not parse the tool-use request");
    };

    let scope = std::env::var(MANAGED_SCOPE_ENV).ok();
    run_request(&request, timeout, scope.as_deref())
}

fn run_request(request: &HookRequest, timeout: Duration, scope: Option<&str>) -> Decision {
    run_request_with_discovery(request, timeout, scope, read_discovery)
}

fn run_request_with_discovery(
    request: &HookRequest,
    timeout: Duration,
    scope: Option<&str>,
    read_discovery: impl FnOnce() -> Option<Discovery>,
) -> Decision {
    if scope != Some(MANAGED_SCOPE_VALUE) {
        return Decision::PassThrough;
    }

    let discovery = read_discovery();
    // Which offline story applies if the request below yields nothing: discovery data that led
    // nowhere means a VaneHub instance registered and then went away; no discovery data means
    // none ever registered here. The deny messages differ so the reader of the denial knows
    // which situation they are recovering from (`add-permission-hook-recovery`'s "Offline
    // denial names its recovery paths").
    let offline = if discovery.is_some() {
        OfflineKind::Unreachable
    } else {
        OfflineKind::NoDiscovery
    };
    let response = discovery.and_then(|discovery| {
        let body = serde_json::to_string(&WrapperRequest {
            tool_name: &request.tool_name,
            tool_input: &request.tool_input,
            session_id: &request.session_id,
            cwd: &request.cwd,
        })
        .ok()?;
        send_request(discovery.port, &discovery.token, &body, timeout)
    });

    decide(response, offline, &request.tool_name)
}

/// How the wrapper ended up without a server response — see the comment in `run`.
#[derive(Clone, Copy)]
enum OfflineKind {
    NoDiscovery,
    Unreachable,
}

fn parse_hook_request(stdin: &str) -> Option<HookRequest> {
    serde_json::from_str(stdin).ok()
}

/// Read-only tools that fail *open* when VaneHub cannot be reached at all — deliberately a
/// short, hardcoded, reviewable list rather than a mirror of the domain's full risk mapping
/// (design.md D5). Does not apply to a malformed-but-reachable response; see `decide`.
fn is_offline_allowlisted(tool_name: &str) -> bool {
    matches!(tool_name, "Read" | "Glob" | "Grep")
}

/// Path and shape must match `hook_bridge_discovery.rs`'s `discovery_file_path`/`DiscoveryFile`
/// exactly — see this file's top doc comment.
fn read_discovery() -> Option<Discovery> {
    let path = dirs::data_local_dir()?
        .join("VaneHub")
        .join("permission-hook.json");
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn send_request(port: u16, token: &str, body: &str, timeout: Duration) -> Option<HttpResponse> {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().ok()?;
    let mut stream = TcpStream::connect_timeout(&addr, timeout).ok()?;
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;

    stream.write_all(&build_http_request(token, body)).ok()?;
    stream.flush().ok()?;

    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).ok()?;

    parse_http_response(&raw)
}

fn build_http_request(token: &str, body: &str) -> Vec<u8> {
    format!(
        "POST /evaluate HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nAuthorization: Bearer {token}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn parse_http_response(raw: &[u8]) -> Option<HttpResponse> {
    let text = String::from_utf8_lossy(raw);
    let (headers, body) = text.split_once("\r\n\r\n")?;
    let status_line = headers.lines().next()?;
    let status = status_line.split_whitespace().nth(1)?.parse().ok()?;
    Some(HttpResponse {
        status,
        body: body.to_string(),
    })
}

/// `response = None` means the server was entirely unreachable (connect/timeout failure, or no
/// discovery file at all) -> risk-tiered allowlist fallback (design.md D5). `response = Some`
/// but not a clean `{"decision": "allow"}` body means the server *was* reached but the response
/// couldn't be trusted (non-200, unparseable JSON, anything other than `"allow"`) -> unconditional
/// deny, no allowlist exception: a reachable-but-corrupt response is a more concerning failure
/// mode than plain absence (design.md D5's clarification; `claude-code-permission-hook`'s
/// "Malformed hook payloads fail closed").
fn decide(response: Option<HttpResponse>, offline: OfflineKind, tool_name: &str) -> Decision {
    match response {
        None if is_offline_allowlisted(tool_name) => Decision::Allow,
        None => Decision::Deny(match offline {
            OfflineKind::Unreachable => {
                "VaneHub is unreachable and this tool is not in the offline allowlist. The \
                 VaneHub instance that installed this hook is not running; start VaneHub to \
                 resume approvals, or run `vanehub-permission-hook --uninstall` to remove the \
                 hook from Claude Code's global settings"
            }
            OfflineKind::NoDiscovery => {
                "no VaneHub instance has registered on this machine and this tool is not in the \
                 offline allowlist; start VaneHub to enable approvals, or run \
                 `vanehub-permission-hook --uninstall` to remove the hook from Claude Code's \
                 global settings"
            }
        }),
        Some(response) if response.status == 200 => {
            match serde_json::from_str::<serde_json::Value>(&response.body) {
                Ok(value) if value.get("decision").and_then(|d| d.as_str()) == Some("allow") => {
                    Decision::Allow
                }
                Ok(_) => Decision::Deny("denied by VaneHub policy"),
                Err(_) => Decision::Deny("VaneHub returned a response that could not be parsed"),
            }
        }
        Some(_) => Decision::Deny("VaneHub rejected the request"),
    }
}

fn print_hook_output(decision: &Decision) {
    let Some(output) = hook_output(decision) else {
        return;
    };
    println!("{output}");
}

fn hook_output(decision: &Decision) -> Option<serde_json::Value> {
    let (permission_decision, reason) = match decision {
        Decision::PassThrough => return None,
        Decision::Allow => ("allow", None),
        Decision::Deny(reason) => ("deny", Some(*reason)),
    };
    let mut output = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": permission_decision,
        }
    });
    if let Some(reason) = reason {
        output["hookSpecificOutput"]["permissionDecisionReason"] =
            serde_json::Value::String(reason.to_string());
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn parse_hook_request_extracts_the_documented_fields() {
        let request = parse_hook_request(
            r#"{"session_id":"s1","hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"ls"},"cwd":"/tmp"}"#,
        )
        .expect("should parse");
        assert_eq!(request.tool_name, "Bash");
        assert_eq!(request.session_id, "s1");
        assert_eq!(request.cwd, "/tmp");
    }

    #[test]
    fn parse_hook_request_rejects_garbage() {
        assert!(parse_hook_request("not json").is_none());
        assert!(
            parse_hook_request("{}").is_none(),
            "missing required fields should fail to parse"
        );
    }

    fn bash_request() -> HookRequest {
        parse_hook_request(
            r#"{"session_id":"s1","hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"ls"},"cwd":"/tmp"}"#,
        )
        .expect("valid hook request")
    }

    #[test]
    fn unscoped_request_passes_through_without_reading_discovery() {
        let decision =
            run_request_with_discovery(&bash_request(), Duration::from_secs(1), None, || {
                panic!("an unmanaged request must not read discovery data")
            });

        assert!(matches!(decision, Decision::PassThrough));
        assert!(hook_output(&decision).is_none());
    }

    #[test]
    fn unknown_scope_also_passes_through_without_a_permission_decision() {
        let decision = run_request_with_discovery(
            &bash_request(),
            Duration::from_secs(1),
            Some("unexpected"),
            || panic!("an unknown scope must not read discovery data"),
        );

        assert!(matches!(decision, Decision::PassThrough));
        assert!(hook_output(&decision).is_none());
    }

    #[test]
    fn managed_scope_preserves_offline_bash_denial() {
        let decision = run_request_with_discovery(
            &bash_request(),
            Duration::from_secs(1),
            Some(MANAGED_SCOPE_VALUE),
            || None,
        );

        assert!(matches!(decision, Decision::Deny(_)));
        assert_eq!(
            hook_output(&decision)
                .and_then(|output| output["hookSpecificOutput"]["permissionDecision"]
                    .as_str()
                    .map(str::to_string))
                .as_deref(),
            Some("deny")
        );
    }

    #[test]
    fn offline_allowlist_covers_exactly_the_read_only_tools() {
        assert!(is_offline_allowlisted("Read"));
        assert!(is_offline_allowlisted("Glob"));
        assert!(is_offline_allowlisted("Grep"));
        assert!(!is_offline_allowlisted("Bash"));
        assert!(!is_offline_allowlisted("Write"));
        assert!(!is_offline_allowlisted("Edit"));
    }

    #[test]
    fn unreachable_server_fails_open_for_allowlisted_tools() {
        assert!(matches!(
            decide(None, OfflineKind::Unreachable, "Read"),
            Decision::Allow
        ));
        assert!(matches!(
            decide(None, OfflineKind::NoDiscovery, "Glob"),
            Decision::Allow
        ));
    }

    #[test]
    fn unreachable_server_fails_closed_for_everything_else() {
        assert!(matches!(
            decide(None, OfflineKind::Unreachable, "Bash"),
            Decision::Deny(_)
        ));
        assert!(matches!(
            decide(None, OfflineKind::NoDiscovery, "SomeFutureTool"),
            Decision::Deny(_)
        ));
    }

    #[test]
    fn offline_deny_reasons_differ_by_state_and_both_name_the_recovery_actions() {
        let Decision::Deny(unreachable) = decide(None, OfflineKind::Unreachable, "Bash") else {
            panic!("unreachable must deny");
        };
        let Decision::Deny(no_discovery) = decide(None, OfflineKind::NoDiscovery, "Bash") else {
            panic!("no-discovery must deny");
        };
        assert_ne!(unreachable, no_discovery);
        for reason in [unreachable, no_discovery] {
            assert!(
                reason.contains("--uninstall"),
                "missing escape hatch: {reason}"
            );
            assert!(
                reason.contains("start VaneHub"),
                "missing restart path: {reason}"
            );
        }
    }

    #[test]
    fn malformed_response_fails_closed_even_for_an_allowlisted_tool() {
        let garbage = HttpResponse {
            status: 200,
            body: "not json".to_string(),
        };
        assert!(matches!(
            decide(Some(garbage), OfflineKind::Unreachable, "Read"),
            Decision::Deny(_)
        ));
    }

    #[test]
    fn non_200_status_fails_closed_even_for_an_allowlisted_tool() {
        let response = HttpResponse {
            status: 500,
            body: r#"{"decision":"allow"}"#.to_string(),
        };
        assert!(matches!(
            decide(Some(response), OfflineKind::Unreachable, "Read"),
            Decision::Deny(_)
        ));
    }

    #[test]
    fn clean_allow_response_allows() {
        let response = HttpResponse {
            status: 200,
            body: r#"{"decision":"allow"}"#.to_string(),
        };
        assert!(matches!(
            decide(Some(response), OfflineKind::Unreachable, "Bash"),
            Decision::Allow
        ));
    }

    #[test]
    fn clean_deny_response_denies() {
        let response = HttpResponse {
            status: 200,
            body: r#"{"decision":"deny"}"#.to_string(),
        };
        assert!(matches!(
            decide(Some(response), OfflineKind::Unreachable, "Bash"),
            Decision::Deny(_)
        ));
    }

    #[test]
    fn build_http_request_has_a_correct_content_length_and_bearer_token() {
        let raw = build_http_request("tok123", r#"{"a":1}"#);
        let text = String::from_utf8(raw).unwrap();
        assert!(text.starts_with("POST /evaluate HTTP/1.1\r\n"));
        assert!(text.contains("Authorization: Bearer tok123\r\n"));
        assert!(text.contains("Content-Length: 7\r\n"));
        assert!(text.ends_with(r#"{"a":1}"#));
    }

    #[test]
    fn parse_http_response_splits_status_and_body() {
        let raw =
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"decision\":\"allow\"}";
        let response = parse_http_response(raw).expect("should parse");
        assert_eq!(response.status, 200);
        assert_eq!(response.body, r#"{"decision":"allow"}"#);
    }

    #[test]
    fn parse_http_response_rejects_a_response_with_no_header_body_separator() {
        assert!(parse_http_response(b"garbage, not http at all").is_none());
    }

    #[test]
    fn send_request_returns_none_on_connection_refused() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let result = send_request(port, "tok", "{}", Duration::from_millis(500));
        assert!(result.is_none());
    }

    #[test]
    fn send_request_times_out_on_a_hung_server_rather_than_blocking_forever() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                thread::sleep(Duration::from_secs(5));
                drop(stream);
            }
        });

        let started = std::time::Instant::now();
        let result = send_request(port, "tok", "{}", Duration::from_millis(300));
        assert!(result.is_none());
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "should time out well before the hung server's own 5s sleep, took {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn send_request_round_trips_against_a_real_minimal_http_server() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let body = r#"{"decision":"allow"}"#;
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        let result = send_request(
            port,
            "tok",
            r#"{"tool_name":"Bash"}"#,
            Duration::from_secs(2),
        )
        .expect("should succeed");
        assert_eq!(result.status, 200);
        assert_eq!(result.body, r#"{"decision":"allow"}"#);
    }
}
