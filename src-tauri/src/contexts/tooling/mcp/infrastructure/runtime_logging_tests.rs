use super::*;
use crate::contexts::tooling::mcp::domain::{
    Scope, ServerConfiguration, ServerConfigurationDraft, TransportType,
};
use crate::test_support::TempDirectory;

fn secret_server() -> ServerConfiguration {
    ServerConfiguration::create(ServerConfigurationDraft {
        name: "secret-fixture".to_string(),
        transport_type: TransportType::Stdio,
        command: Some("C:\\private\\secret-server.exe".to_string()),
        args: Some(vec!["--token=raw-argument-secret".to_string()]),
        env: Some([("API_TOKEN".to_string(), "raw-env-secret".to_string())].into()),
        url: None,
        headers: Some(
            [(
                "authorization".to_string(),
                "Bearer raw-header-secret".to_string(),
            )]
            .into(),
        ),
        description: None,
        active: true,
        scope: Scope::User,
        project_path: None,
    })
    .expect("server")
}

fn assert_only_safe_log_remains(directory: &std::path::Path, secrets: &[&str]) -> String {
    let entries = std::fs::read_dir(directory)
        .expect("log directory")
        .collect::<Result<Vec<_>, _>>()
        .expect("log entries");
    assert_eq!(entries.len(), 1, "unexpected diagnostic artifact remained");
    let entry = &entries[0];
    assert_eq!(entry.file_name(), logging::LOG_FILE_NAME);
    let raw = std::fs::read_to_string(entry.path()).expect("safe log");
    for secret in secrets {
        assert!(!raw.contains(secret), "runtime log leaked {secret}");
        assert!(
            !entry.file_name().to_string_lossy().contains(secret),
            "diagnostic filename leaked {secret}"
        );
    }
    raw
}

#[test]
fn normal_sink_receives_only_safe_command_and_failure_metadata() {
    let directory = TempDirectory::new("mcp-runtime-safe-log");
    let server = secret_server();
    let context = McpRuntimeLogContext::for_server(&server, Some("operation-safe-17"));
    let error = McpRuntimeError::with_diagnostic(
        McpFailureCode::Protocol,
        "body=raw-body-secret schema=raw-schema-secret tool=raw-tool-secret",
    );
    let command = RuntimeDiagnostic {
        context: &context,
        phase: RuntimePhase::Command,
        outcome: RuntimeOutcome::Started,
        error_code: None,
        duration: None,
        stderr_observed_bytes: None,
        stderr_truncated: None,
        exit_code: None,
    };
    persist_to_dirs(&command, directory.path(), directory.path());
    let failure = RuntimeDiagnostic {
        context: &context,
        phase: RuntimePhase::Protocol,
        outcome: RuntimeOutcome::Failed,
        error_code: Some(error.code()),
        duration: Some(Duration::from_millis(12)),
        stderr_observed_bytes: Some(96 * 1024),
        stderr_truncated: Some(true),
        exit_code: Some(1),
    };
    persist_to_dirs(&failure, directory.path(), directory.path());

    let raw = assert_only_safe_log_remains(
        directory.path(),
        &[
            "secret-server.exe",
            "raw-argument-secret",
            "raw-env-secret",
            "raw-header-secret",
            "raw-body-secret",
            "raw-schema-secret",
            "raw-tool-secret",
        ],
    );
    assert!(raw.contains("operation-safe-17"));
    assert!(raw.contains("absolute_path"));
    assert!(raw.contains("\"argumentCount\":\"1\""));
    assert!(raw.contains("\"errorCode\":\"protocol\""));
    assert!(raw.contains("\"stderrTruncated\":\"true\""));
}

#[test]
fn emergency_sink_receives_only_a_fixed_already_safe_classification() {
    let normal = TempDirectory::new("mcp-runtime-broken-normal");
    let broken_normal = normal.write("not-a-directory", "raw-relay-config-secret");
    let emergency = TempDirectory::new("mcp-runtime-emergency");
    let server = secret_server();
    let context = McpRuntimeLogContext::for_server(&server, Some("operation-safe-18"));
    let diagnostic = RuntimeDiagnostic {
        context: &context,
        phase: RuntimePhase::Cleanup,
        outcome: RuntimeOutcome::Failed,
        error_code: Some(McpFailureCode::Cleanup),
        duration: None,
        stderr_observed_bytes: Some(64 * 1024),
        stderr_truncated: Some(true),
        exit_code: None,
    };

    persist_to_dirs(&diagnostic, &broken_normal, emergency.path());

    let raw = assert_only_safe_log_remains(
        emergency.path(),
        &[
            "secret-fixture",
            "operation-safe-18",
            "raw-relay-config-secret",
            "raw-argument-secret",
            "raw-env-secret",
            "raw-header-secret",
        ],
    );
    assert!(raw.contains("mcp_runtime_logging_unavailable"));
}

#[test]
fn relay_correlation_is_preserved_without_private_configuration() {
    let context = McpRuntimeLogContext::for_relay(
        "streamable_http",
        Some("run-safe"),
        Some("trace-safe"),
        Some("span-safe"),
    );
    let diagnostic = RuntimeDiagnostic {
        context: &context,
        phase: RuntimePhase::Relay,
        outcome: RuntimeOutcome::Succeeded,
        error_code: None,
        duration: None,
        stderr_observed_bytes: None,
        stderr_truncated: None,
        exit_code: None,
    };

    let (_, _, rendered) = model::render(&diagnostic);

    assert_eq!(rendered.get("runId").map(String::as_str), Some("run-safe"));
    assert_eq!(
        rendered.get("traceId").map(String::as_str),
        Some("trace-safe")
    );
    assert_eq!(
        rendered.get("spanId").map(String::as_str),
        Some("span-safe")
    );
}
