use super::managed_session::{resolve_result, ManagedMcpSession};
use super::runtime_logging::McpRuntimeLogContext;
use crate::contexts::tooling::mcp::application::{McpExecutionControl, McpLimits, McpRuntimeError};
use crate::contexts::tooling::mcp::domain::{
    McpFailureCode, Scope, ServerConfiguration, ServerConfigurationDraft, TransportType,
};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
#[cfg(windows)]
use tokio::io::{AsyncBufReadExt, BufReader};
#[cfg(windows)]
use windows::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
};

fn stdio_server(command: String, args: Vec<String>) -> ServerConfiguration {
    ServerConfiguration::create(ServerConfigurationDraft {
        name: "managed-session-fixture".to_string(),
        transport_type: TransportType::Stdio,
        command: Some(command),
        args: Some(args),
        env: None,
        url: None,
        headers: None,
        description: None,
        active: true,
        scope: Scope::User,
        project_path: None,
    })
    .expect("server")
}

#[test]
fn cleanup_failure_replaces_remote_success_but_not_the_primary_failure() {
    let cleanup = || Err(McpRuntimeError::new(McpFailureCode::Cleanup));
    let success = resolve_result(Ok("remote success"), cleanup()).expect_err("cleanup");
    assert_eq!(success.code(), McpFailureCode::Cleanup);

    let primary = resolve_result(
        Err::<(), _>(McpRuntimeError::new(McpFailureCode::Timeout)),
        cleanup(),
    )
    .expect_err("primary");
    assert_eq!(primary.code(), McpFailureCode::Timeout);
}

#[test]
fn stdio_session_startup_failure_creates_no_owned_task_or_child() {
    let server = stdio_server(
        "vanehub-definitely-missing-mcp-executable".to_string(),
        Vec::new(),
    );
    let result = ManagedMcpSession::<()>::spawn_stdio(
        "vanehub-definitely-missing-mcp-executable",
        &[],
        &BTreeMap::new(),
        McpLimits::DEFAULT.protocol_message_bytes,
        McpLimits::DEFAULT.stderr_bytes,
        McpRuntimeLogContext::for_server(&server, Some("managed-session-spawn-failure")),
        |_stdout, _stdin| async { Ok(()) },
    );

    let error = match result {
        Ok(_) => panic!("missing executable unexpectedly started"),
        Err(error) => error,
    };
    assert_eq!(error.code(), McpFailureCode::Spawn);
}

#[tokio::test]
async fn managed_http_cleanup_finishes_before_success_is_returned() {
    let session = ManagedMcpSession::spawn_http(
        || async { Ok("remote success") },
        |_deadline| async { Err(McpRuntimeError::new(McpFailureCode::Cleanup)) },
    );

    let error = session
        .run(&McpExecutionControl::with_timeout(Duration::from_secs(1)))
        .await
        .expect_err("cleanup changes success");

    assert_eq!(error.code(), McpFailureCode::Cleanup);
}

#[tokio::test]
async fn managed_cancellation_drops_and_joins_the_owned_operation_task() {
    struct ActiveGuard(Arc<AtomicUsize>);
    impl Drop for ActiveGuard {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }

    let active = Arc::new(AtomicUsize::new(0));
    let active_for_operation = active.clone();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let session = ManagedMcpSession::<()>::spawn_http(
        move || async move {
            active_for_operation.fetch_add(1, Ordering::SeqCst);
            let _guard = ActiveGuard(active_for_operation);
            let _ = started_tx.send(());
            std::future::pending::<Result<(), McpRuntimeError>>().await
        },
        |_| async { Ok(()) },
    );
    let control = McpExecutionControl::with_timeout(Duration::from_secs(2));
    let cancellation = control.cancellation();
    let running = tokio::spawn(async move { session.run(&control).await });

    started_rx.await.expect("operation started");
    cancellation.cancel();
    let error = running.await.expect("session join").expect_err("cancelled");

    assert_eq!(error.code(), McpFailureCode::Cancelled);
    assert_eq!(active.load(Ordering::SeqCst), 0);
}

#[cfg(windows)]
#[tokio::test]
async fn managed_stdio_cancellation_terminates_the_owned_descendant_tree() {
    let executable = std::env::current_exe().expect("test executable");
    let args = vec![
        "--ignored".to_string(),
        "--exact".to_string(),
        "platform::process::managed_child::tests::managed_child_descendant_launcher_fixture"
            .to_string(),
        "--nocapture".to_string(),
    ];
    let server = stdio_server(executable.to_string_lossy().to_string(), args.clone());
    let (pid_tx, pid_rx) = tokio::sync::oneshot::channel();
    let session = ManagedMcpSession::<()>::spawn_stdio(
        &executable.to_string_lossy(),
        &args,
        &BTreeMap::new(),
        McpLimits::DEFAULT.protocol_message_bytes,
        McpLimits::DEFAULT.stderr_bytes,
        McpRuntimeLogContext::for_server(&server, Some("managed-session-descendant-test")),
        move |stdout, _stdin| async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Some(line) = lines.next_line().await.map_err(|error| {
                McpRuntimeError::with_diagnostic(McpFailureCode::Transport, error.to_string())
            })? {
                if let Some(value) = line.trim().strip_prefix("DESCENDANT_PID ") {
                    let pid = value.parse::<u32>().map_err(|error| {
                        McpRuntimeError::with_diagnostic(
                            McpFailureCode::Protocol,
                            error.to_string(),
                        )
                    })?;
                    let _ = pid_tx.send(pid);
                    return std::future::pending::<Result<(), McpRuntimeError>>().await;
                }
            }
            Err(McpRuntimeError::new(McpFailureCode::Protocol))
        },
    )
    .expect("session");
    let control = McpExecutionControl::with_timeout(Duration::from_secs(5));
    let cancellation = control.cancellation();
    let running = tokio::spawn(async move { session.run(&control).await });

    let descendant_pid = pid_rx.await.expect("descendant pid");
    assert!(process_is_running(descendant_pid));
    cancellation.cancel();
    let error = running.await.expect("session join").expect_err("cancelled");

    assert_eq!(error.code(), McpFailureCode::Cancelled);
    assert!(!process_is_running(descendant_pid));
}

#[cfg(windows)]
fn process_is_running(pid: u32) -> bool {
    let Ok(handle) = (unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }) else {
        return false;
    };
    let mut exit_code = 0_u32;
    let active = unsafe { GetExitCodeProcess(handle, &mut exit_code) }.is_ok()
        && exit_code == STILL_ACTIVE.0 as u32;
    let _ = unsafe { CloseHandle(handle) };
    active
}

#[tokio::test]
async fn managed_stdio_timeout_finishes_owned_child_and_drain_cleanup() {
    let executable = std::env::current_exe().expect("test executable");
    let args = vec![
        "--ignored".to_string(),
        "--exact".to_string(),
        "platform::process::managed_child::tests::managed_child_hang_fixture".to_string(),
        "--nocapture".to_string(),
    ];
    let server = stdio_server(executable.to_string_lossy().to_string(), args.clone());
    let session = ManagedMcpSession::<()>::spawn_stdio(
        &executable.to_string_lossy(),
        &args,
        &BTreeMap::new(),
        McpLimits::DEFAULT.protocol_message_bytes,
        McpLimits::DEFAULT.stderr_bytes,
        McpRuntimeLogContext::for_server(&server, Some("managed-session-test")),
        |_stdout, _stdin| async { std::future::pending::<Result<(), McpRuntimeError>>().await },
    )
    .expect("session");
    let started = Instant::now();

    let error = session
        .run(&McpExecutionControl::with_timeout(Duration::from_millis(
            200,
        )))
        .await
        .expect_err("deadline");

    assert_eq!(error.code(), McpFailureCode::Timeout);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[tokio::test]
async fn managed_stdio_maps_limit_plus_one_to_the_typed_limit_failure() {
    let maximum = McpLimits::DEFAULT.protocol_message_bytes;
    let script = format!("process.stdout.write('x'.repeat({}));", maximum + 1);
    let args = vec!["-e".to_string(), script];
    let server = stdio_server("node".to_string(), args.clone());
    let session = ManagedMcpSession::<()>::spawn_stdio(
        "node",
        &args,
        &BTreeMap::new(),
        maximum,
        McpLimits::DEFAULT.stderr_bytes,
        McpRuntimeLogContext::for_server(&server, Some("managed-session-limit-test")),
        |mut stdout, _stdin| async move {
            let mut output = Vec::new();
            stdout.read_to_end(&mut output).await.map_err(|error| {
                McpRuntimeError::with_diagnostic(McpFailureCode::Transport, error.to_string())
            })?;
            Ok(())
        },
    )
    .expect("session");

    let error = session
        .run(&McpExecutionControl::with_timeout(Duration::from_secs(5)))
        .await
        .expect_err("limit plus one");

    assert_eq!(error.code(), McpFailureCode::LimitExceeded);
}
