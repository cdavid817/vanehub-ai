use super::*;
use crate::test_support::TempDirectory;
use std::io::{Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

#[test]
fn non_relay_process_arguments_are_ignored() {
    assert!(!try_run_from_process_args());
    assert!(!run_from_process_args(["vanehub".into()]).expect("non-relay arguments"));
}

#[test]
fn relay_process_arguments_require_and_run_the_configuration_path() {
    assert!(run_from_process_args(["vanehub".into(), RELAY_FLAG.into()]).is_err());

    let directory = TempDirectory::new("mcp-relay-process-args");
    let path = write_configuration(
        directory.path(),
        &RelayConfiguration {
            target: RelayTarget::Http {
                url: "http://127.0.0.1:1".to_string(),
                headers: BTreeMap::new(),
            },
            traceparent: "traceparent".to_string(),
            observation: None,
            timeout_ms: 10,
        },
    )
    .expect("write process configuration");
    assert!(run_from_process_args([
        "vanehub".into(),
        RELAY_FLAG.into(),
        path.as_os_str().to_owned(),
    ])
    .expect("run relay configuration"));
    assert!(!path.exists());
}

#[test]
fn relay_configuration_round_trip_preserves_literal_json_rpc_arguments() {
    let literal = r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"arguments":{"quoted":"a b \\\" c"}}}"#;
    let configuration = RelayConfiguration {
        target: RelayTarget::Stdio {
            command: "node".to_string(),
            args: vec!["fixture.cjs".to_string(), literal.to_string()],
            env: BTreeMap::new(),
        },
        traceparent: "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_string(),
        observation: None,
        timeout_ms: 25,
    };
    let encoded = serde_json::to_vec(&configuration).expect("encode");
    let decoded = serde_json::from_slice::<RelayConfiguration>(&encoded).expect("decode");
    let RelayTarget::Stdio { args, .. } = decoded.target else {
        panic!("stdio target expected");
    };
    assert_eq!(args[1], literal);
}

#[test]
fn configuration_never_invents_an_unbounded_timeout() {
    assert_eq!(default_timeout_ms(), DEFAULT_TIMEOUT_MS);
}

#[test]
fn stdio_forwarding_is_byte_transparent_for_json_rpc_and_protocol_errors() {
    let input = br#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"arguments":{"literal":"a b ; & \"quoted\""}}}
not-json-but-still-forwarded
"#;
    let mut output = Vec::new();

    forward_stdio_input(Cursor::new(input), &mut output, None).expect("forward");

    assert_eq!(output, input);
}

#[test]
fn stdio_forwarding_propagates_a_bounded_writer_failure() {
    struct FailingWriter;
    impl Write for FailingWriter {
        fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "cancelled",
            ))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let result = forward_stdio_input(
        Cursor::new(
            br#"{"jsonrpc":"2.0","id":1,"method":"ping"}
"#,
        ),
        &mut FailingWriter,
        None,
    );

    assert!(result.is_err());
}

#[test]
fn configuration_routes_stdio_and_removes_secret_bearing_temporary_files_on_failure() {
    let directory = TempDirectory::new("mcp-relay-config");
    let path = write_configuration(
        directory.path(),
        &RelayConfiguration {
            target: RelayTarget::Stdio {
                command: "definitely-missing-vanehub-relay-command".to_string(),
                args: Vec::new(),
                env: BTreeMap::from([("API_TOKEN".to_string(), "credential-secret".to_string())]),
            },
            traceparent: "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01".to_string(),
            observation: None,
            timeout_ms: 10,
        },
    )
    .expect("write configuration");

    let error = run_configuration(&path).expect_err("missing child must fail");

    assert!(!path.exists());
    assert!(!error.contains("credential-secret"));
}

#[test]
fn malformed_configuration_is_removed_and_rejected() {
    let directory = TempDirectory::new("mcp-relay-malformed");
    let path = directory.write("relay.json", "{not-json");

    assert!(run_configuration(&path).is_err());
    assert!(!path.exists());
}

#[test]
fn configuration_routes_stdio_through_a_real_bounded_child() {
    let directory = TempDirectory::new("mcp-relay-stdio-child");
    let executable = std::env::current_exe().expect("current test executable");
    let path = write_configuration(
        directory.path(),
        &RelayConfiguration {
            target: RelayTarget::Stdio {
                command: executable.to_string_lossy().into_owned(),
                args: vec![
                    "--ignored".to_string(),
                    "--exact".to_string(),
                    "contexts::tooling::mcp::infrastructure::relay::tests::relay_stdio_child_fixture"
                        .to_string(),
                ],
                env: BTreeMap::new(),
            },
            traceparent: "traceparent".to_string(),
            observation: None,
            timeout_ms: 1_000,
        },
    )
    .expect("write configuration");

    run_configuration(&path).expect("stdio child relay");
    assert!(!path.exists());
}

#[test]
fn http_configuration_with_empty_input_uses_the_stream_router() {
    let directory = TempDirectory::new("mcp-relay-http-config");
    let path = write_configuration(
        directory.path(),
        &RelayConfiguration {
            target: RelayTarget::Http {
                url: "http://127.0.0.1:1".to_string(),
                headers: BTreeMap::new(),
            },
            traceparent: "traceparent".to_string(),
            observation: None,
            timeout_ms: 10,
        },
    )
    .expect("write configuration");

    run_configuration(&path).expect("empty input performs no HTTP request");
    assert!(!path.exists());
}

#[test]
fn http_routing_forwards_json_rpc_and_refuses_redirects() {
    let body = br#"{"jsonrpc":"2.0","id":1,"result":{"tools":[]}}"#;
    let (url, server) = one_response_server("200 OK", body, &[], None);
    let mut output = Vec::new();
    relay_http_stream(
        &url,
        &BTreeMap::from([("authorization".to_string(), "Bearer redacted".to_string())]),
        "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        Duration::from_secs(1),
        None,
        Cursor::new(
            br#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}
"#,
        ),
        &mut output,
    )
    .expect("HTTP relay");
    server.join().expect("HTTP fixture");
    assert_eq!(output, [body.as_slice(), b"\n"].concat());

    let (redirect_url, redirect_server) = one_response_server(
        "302 Found",
        b"",
        &[("location", "https://example.invalid/redirect")],
        None,
    );
    let error = relay_http_stream(
        &redirect_url,
        &BTreeMap::new(),
        "traceparent",
        Duration::from_secs(1),
        None,
        Cursor::new(
            br#"{"jsonrpc":"2.0","id":2,"method":"ping"}
"#,
        ),
        &mut Vec::new(),
    )
    .expect_err("redirect refused");
    redirect_server.join().expect("redirect fixture");
    assert_eq!(error, "MCP HTTP relay refused a redirect");
}

#[test]
fn http_routing_has_a_bounded_timeout_and_rejects_protocol_errors() {
    let (url, server) = one_response_server("200 OK", b"{}", &[], Some(Duration::from_millis(100)));
    let timeout = relay_http_stream(
        &url,
        &BTreeMap::new(),
        "traceparent",
        Duration::from_millis(10),
        None,
        Cursor::new(
            br#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}
"#,
        ),
        &mut Vec::new(),
    )
    .expect_err("request timeout");
    server.join().expect("timeout fixture");
    assert!(!timeout.is_empty());

    let invalid = relay_http_stream(
        "http://127.0.0.1:1",
        &BTreeMap::new(),
        "traceparent",
        Duration::from_millis(10),
        None,
        Cursor::new(b"not-json\n"),
        &mut Vec::new(),
    )
    .expect_err("invalid JSON-RPC");
    assert_eq!(invalid, "relay received invalid JSON-RPC");
}

#[test]
fn http_routing_reuses_session_identity_and_propagates_output_failure() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind session fixture");
    let address = listener.local_addr().expect("session fixture address");
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for id in 1..=2 {
            let (mut stream, _) = listener.accept().expect("accept session request");
            let request = read_http_request(&mut stream);
            requests.push(String::from_utf8_lossy(&request).into_owned());
            let body = format!(r#"{{"jsonrpc":"2.0","id":{id},"result":{{}}}}"#);
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\nmcp-session-id: session-reused\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write session response");
        }
        requests
    });
    let input = br#"{"jsonrpc":"2.0","id":1,"method":"initialize"}
{"jsonrpc":"2.0","id":2,"method":"tools/list"}
"#;
    relay_http_stream(
        &format!("http://{address}"),
        &BTreeMap::new(),
        "traceparent",
        Duration::from_secs(1),
        None,
        Cursor::new(input),
        &mut Vec::new(),
    )
    .expect("two HTTP requests");
    let requests = server.join().expect("session fixture");
    assert!(!requests[0].to_ascii_lowercase().contains("mcp-session-id"));
    assert!(requests[1]
        .to_ascii_lowercase()
        .contains("mcp-session-id: session-reused"));

    struct FailingOutput;
    impl Write for FailingOutput {
        fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "output closed",
            ))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    let (url, output_server) = one_response_server("500 Internal Server Error", b"{}", &[], None);
    assert!(relay_http_stream(
        &url,
        &BTreeMap::new(),
        "traceparent",
        Duration::from_secs(1),
        None,
        Cursor::new(
            br#"{"jsonrpc":"2.0","id":3,"method":"ping"}
"#,
        ),
        &mut FailingOutput,
    )
    .is_err());
    output_server.join().expect("output fixture");
}

#[test]
#[ignore = "spawned only by the bounded stdio relay test"]
fn relay_stdio_child_fixture() {
    let mut input = std::io::stdin().lock();
    let mut output = std::io::stdout().lock();
    std::io::copy(&mut input, &mut output).expect("echo relay input");
}

fn one_response_server(
    status: &'static str,
    body: &'static [u8],
    headers: &'static [(&'static str, &'static str)],
    delay: Option<Duration>,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind HTTP fixture");
    let address = listener.local_addr().expect("fixture address");
    let body = body.to_vec();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept HTTP request");
        let _ = read_http_request(&mut stream);
        if let Some(delay) = delay {
            thread::sleep(delay);
        }
        let extra_headers = headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}\r\n"))
            .collect::<String>();
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\n{extra_headers}Connection: close\r\n\r\n",
            body.len()
        );
        let response = [response.as_bytes(), body.as_slice()].concat();
        stream.write_all(&response).expect("write HTTP response");
    });
    (format!("http://{address}"), handle)
}

fn read_http_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        let count = stream.read(&mut buffer).expect("read HTTP request");
        if count == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..count]);
        let Some(header_end) = request.windows(4).position(|value| value == b"\r\n\r\n") else {
            continue;
        };
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        if request.len() >= header_end + 4 + content_length {
            break;
        }
    }
    request
}
