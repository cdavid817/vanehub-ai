use super::*;
use crate::contexts::tooling::mcp::application::McpLimits;
use std::io::{self, BufRead, Cursor, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn listener_url() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let url = format!("http://{}/events", listener.local_addr().expect("address"));
    (listener, url)
}

fn read_request(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut byte = [0_u8; 1];
    while !bytes.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).expect("request header");
        bytes.push(byte[0]);
    }
    let headers = String::from_utf8_lossy(&bytes);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    let mut body = vec![0_u8; content_length];
    stream.read_exact(&mut body).expect("request body");
    bytes.extend(body);
    String::from_utf8(bytes).expect("request")
}

fn open_event_stream(stream: &mut TcpStream, endpoint: &str) {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: keep-alive\r\n\r\nevent: endpoint\ndata: {endpoint}\r\n\r\n"
    )
    .expect("event stream");
    stream.flush().expect("endpoint");
}

fn accepted(stream: &mut TcpStream) {
    stream
        .write_all(b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n\r\n")
        .expect("accepted");
}

fn status_response(stream: &mut TcpStream, status: &str, body: &[u8]) {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .expect("status headers");
    stream.write_all(body).expect("status body");
    stream.flush().expect("status response");
}

#[test]
fn relay_negotiates_endpoint_posts_and_forwards_matching_sse_response() {
    let (listener, url) = listener_url();
    let fixture = thread::spawn(move || {
        let (mut events, _) = listener.accept().expect("event stream");
        let get = read_request(&mut events);
        open_event_stream(&mut events, "/messages?session=one");
        let (mut post, _) = listener.accept().expect("POST");
        let post_request = read_request(&mut post);
        accepted(&mut post);
        events
            .write_all(b"event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n")
            .expect("response event");
        events.flush().expect("response flush");
        (get, post_request)
    });
    let mut output = Vec::new();

    run_stream(
        &url,
        &BTreeMap::from([("x-test-token".to_string(), "fixture-secret".to_string())]),
        "traceparent",
        Duration::from_secs(2),
        McpCancellation::default(),
        None,
        Cursor::new(
            br#"{"jsonrpc":"2.0","id":1,"method":"ping"}
"#,
        ),
        &mut output,
    )
    .expect("legacy relay");

    let (get, post) = fixture.join().expect("fixture");
    assert!(get.starts_with("GET /events HTTP/1.1"), "{get}");
    assert!(
        post.starts_with("POST /messages?session=one HTTP/1.1"),
        "{post}"
    );
    assert!(get
        .to_ascii_lowercase()
        .contains("x-test-token: fixture-secret"));
    assert!(post
        .to_ascii_lowercase()
        .contains("x-test-token: fixture-secret"));
    assert_eq!(output, b"{\"id\":1,\"jsonrpc\":\"2.0\",\"result\":{}}\n");
}

struct ResponseAfterOutput {
    ready: Arc<(Mutex<bool>, Condvar)>,
    bytes: Vec<u8>,
    position: usize,
}

impl Read for ResponseAfterOutput {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let available = self.fill_buf()?;
        let count = available.len().min(output.len());
        output[..count].copy_from_slice(&available[..count]);
        self.consume(count);
        Ok(count)
    }
}

impl BufRead for ResponseAfterOutput {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.position == 0 {
            let (ready, changed) = &*self.ready;
            let mut ready = ready.lock().expect("ready");
            while !*ready {
                ready = changed.wait(ready).expect("signal");
            }
        }
        Ok(&self.bytes[self.position..])
    }

    fn consume(&mut self, amount: usize) {
        self.position = (self.position + amount).min(self.bytes.len());
    }
}

struct SignallingOutput {
    ready: Arc<(Mutex<bool>, Condvar)>,
    bytes: Vec<u8>,
}

impl Write for SignallingOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        if self.bytes.contains(&b'\n') {
            let (ready, changed) = &*self.ready;
            *ready.lock().expect("ready") = true;
            changed.notify_all();
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn server_request_is_forwarded_and_parent_response_posts_to_negotiated_endpoint() {
    let (listener, url) = listener_url();
    let fixture = thread::spawn(move || {
        let (mut events, _) = listener.accept().expect("event stream");
        let _ = read_request(&mut events);
        open_event_stream(&mut events, "/messages");
        events
            .write_all(
                b"event: message\ndata: {\"jsonrpc\":\"2.0\",\"id\":\"server-1\",\"method\":\"roots/list\"}\n\n",
            )
            .expect("server request");
        events.flush().expect("request flush");
        let (mut post, _) = listener.accept().expect("response POST");
        let response = read_request(&mut post);
        accepted(&mut post);
        response
    });
    let ready = Arc::new((Mutex::new(false), Condvar::new()));
    let input = ResponseAfterOutput {
        ready: Arc::clone(&ready),
        bytes: br#"{"jsonrpc":"2.0","id":"server-1","result":{"roots":[]}}
"#
        .to_vec(),
        position: 0,
    };
    let mut output = SignallingOutput {
        ready,
        bytes: Vec::new(),
    };

    run_stream(
        &url,
        &BTreeMap::new(),
        "traceparent",
        Duration::from_secs(2),
        McpCancellation::default(),
        None,
        input,
        &mut output,
    )
    .expect("bidirectional relay");

    let post = fixture.join().expect("fixture");
    assert!(post.contains("\"id\":\"server-1\""));
    assert!(post.contains("\"roots\":[]"));
    assert_eq!(
        output.bytes,
        b"{\"id\":\"server-1\",\"jsonrpc\":\"2.0\",\"method\":\"roots/list\"}\n"
    );
}

#[path = "relay_legacy_sse_failure_tests.rs"]
mod failure_tests;
