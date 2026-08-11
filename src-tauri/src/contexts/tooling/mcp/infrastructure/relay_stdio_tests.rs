use super::*;
use crate::contexts::tooling::mcp::application::McpCancellation;
use crate::platform::process::ManagedChild;
use std::collections::BTreeMap;
use std::io::{self, BufRead, Read, Write};
use std::sync::{Arc, Mutex};

struct ControlledReader {
    bytes: Vec<u8>,
    position: usize,
    stop: PumpStop,
}

impl ControlledReader {
    fn open(bytes: impl Into<Vec<u8>>, stop: PumpStop) -> Self {
        Self {
            bytes: bytes.into(),
            position: 0,
            stop,
        }
    }
}

impl BufRead for ControlledReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.position < self.bytes.len() {
            return Ok(&self.bytes[self.position..]);
        }
        while !self.stop.is_stopped() {
            std::thread::sleep(Duration::from_millis(2));
        }
        Ok(&[])
    }

    fn consume(&mut self, amount: usize) {
        self.position = (self.position + amount).min(self.bytes.len());
    }
}

impl Read for ControlledReader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let available = self.fill_buf()?;
        let count = available.len().min(output.len());
        output[..count].copy_from_slice(&available[..count]);
        self.consume(count);
        Ok(count)
    }
}

#[derive(Clone, Default)]
struct SharedOutput {
    bytes: Arc<Mutex<Vec<u8>>>,
}

impl SharedOutput {
    fn bytes(&self) -> Vec<u8> {
        self.bytes.lock().expect("output").clone()
    }
}

impl Write for SharedOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.lock().expect("output").extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn node_child(script: &str) -> ManagedChild {
    ManagedChild::spawn(
        "node",
        &["-e".to_string(), script.to_string()],
        &BTreeMap::new(),
    )
    .expect("node child")
}

fn log_context() -> McpRuntimeLogContext {
    McpRuntimeLogContext::for_relay("stdio", None, None, None)
}

#[test]
fn child_exit_does_not_wait_for_open_parent_input() {
    let stop = PumpStop::default();
    let started = Instant::now();

    let result = supervise(
        node_child(""),
        ControlledReader::open(Vec::new(), stop.clone()),
        SharedOutput::default(),
        Duration::from_secs(5),
        McpCancellation::default(),
        stop.clone(),
        None,
        &log_context(),
    );

    assert!(result.is_ok(), "{result:?}");
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(stop.is_stopped());
}

#[test]
fn child_disconnect_fails_pending_request_without_waiting_for_parent_eof() {
    let stop = PumpStop::default();
    let output = SharedOutput::default();
    let request = br#"{"jsonrpc":"2.0","id":"disconnect-1","method":"ping"}
"#;
    let started = Instant::now();

    let _result = supervise(
        node_child("process.stdin.once('data', () => process.exit(7))"),
        ControlledReader::open(request, stop.clone()),
        output.clone(),
        Duration::from_secs(10),
        McpCancellation::default(),
        stop,
        None,
        &log_context(),
    );

    assert!(started.elapsed() < Duration::from_secs(6));
    let response: serde_json::Value =
        serde_json::from_slice(&output.bytes()).expect("disconnect response");
    assert_eq!(response["id"], "disconnect-1");
    assert_eq!(response["error"]["code"], -32000);
}

#[test]
fn parent_eof_forces_a_hanging_child_through_bounded_shutdown() {
    let started = Instant::now();

    let result = supervise(
        node_child("setInterval(() => {}, 1000)"),
        std::io::Cursor::new(Vec::<u8>::new()),
        SharedOutput::default(),
        Duration::from_secs(5),
        McpCancellation::default(),
        PumpStop::default(),
        None,
        &log_context(),
    );

    assert!(result.is_ok(), "{result:?}");
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn oldest_request_timeout_returns_json_rpc_error_with_parent_input_open() {
    let stop = PumpStop::default();
    let output = SharedOutput::default();
    let started = Instant::now();
    let request = br#"{"jsonrpc":"2.0","id":41,"method":"tools/call"}
"#;

    let result = supervise(
        node_child("process.stdin.resume(); setInterval(() => {}, 1000)"),
        ControlledReader::open(request, stop.clone()),
        output.clone(),
        Duration::from_millis(50),
        McpCancellation::default(),
        stop,
        None,
        &log_context(),
    );

    assert!(result.expect_err("timeout").contains("timed out"));
    assert!(started.elapsed() < Duration::from_secs(2));
    let response: serde_json::Value =
        serde_json::from_slice(&output.bytes()).expect("timeout response");
    assert_eq!(response["id"], 41);
    assert_eq!(response["error"]["code"], -32001);
}

#[test]
fn cancellation_and_pump_failure_each_terminate_a_hanging_child() {
    let cancellation = McpCancellation::default();
    cancellation.cancel();
    let cancellation_stop = PumpStop::default();
    let cancelled = supervise(
        node_child("setInterval(() => {}, 1000)"),
        ControlledReader::open(Vec::new(), cancellation_stop.clone()),
        SharedOutput::default(),
        Duration::from_secs(5),
        cancellation,
        cancellation_stop,
        None,
        &log_context(),
    );
    assert!(cancelled.expect_err("cancelled").contains("cancelled"));

    let stop = PumpStop::default();
    let failed = supervise(
        node_child("process.stdin.resume(); setInterval(() => {}, 1000)"),
        ControlledReader::open(b"invalid-json\n", stop.clone()),
        SharedOutput::default(),
        Duration::from_secs(5),
        McpCancellation::default(),
        stop,
        None,
        &log_context(),
    );
    assert!(failed
        .expect_err("pump failure")
        .contains("invalid JSON-RPC"));
}
