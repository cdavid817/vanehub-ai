use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn run_fixture_probe(probe: &str) {
    let mut failures = Vec::new();
    for attempt in 1..=2 {
        match run_fixture_probe_once(probe) {
            Ok(stdout) => {
                assert!(stdout.contains(&format!("OK {probe}")), "{stdout}");
                return;
            }
            Err(failure) => failures.push(format!("attempt {attempt}: {failure}")),
        }
    }
    panic!("MCP {probe} fixture probe failed\n{}", failures.join("\n"));
}

fn run_fixture_probe_once(probe: &str) -> Result<String, String> {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let script = manifest_dir
        .join("tests")
        .join("fixtures")
        .join("mcp_fixture_probe.cjs");
    let mut child = Command::new("node")
        .arg(script)
        .arg(probe)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start MCP fixture contract probe");
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll fixture probe") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err("exceeded its 20 second wall-clock guard".to_string());
        }
        thread::sleep(Duration::from_millis(20));
    };

    let mut stdout = String::new();
    let mut stderr = String::new();
    child
        .stdout
        .take()
        .expect("probe stdout")
        .read_to_string(&mut stdout)
        .expect("read probe stdout");
    child
        .stderr
        .take()
        .expect("probe stderr")
        .read_to_string(&mut stderr)
        .expect("read probe stderr");
    if !status.success() {
        return Err(format!("stdout:\n{stdout}\nstderr:\n{stderr}"));
    }
    Ok(stdout)
}

#[test]
fn stdio_fixture_contract_is_complete() {
    run_fixture_probe("stdio");
}

#[test]
fn streamable_http_fixture_contract_is_complete() {
    run_fixture_probe("streamable-http");
}

#[test]
fn legacy_sse_fixture_contract_is_complete() {
    run_fixture_probe("legacy-sse");
}
