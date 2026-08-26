use super::lsp_diagnostics::{
    LspCrashReason, LspDiagnosticEvent, LspDiagnosticIdentity, LspDiagnosticKind,
    LspDiagnosticLogger, LspMethodCategory, LspPrivateDiagnosticData,
};
use crate::contexts::code_intelligence::domain::models::ProcessState;
use crate::contexts::code_intelligence::domain::registry;
use crate::contexts::operations::api::{
    DiagnosticLog, DiagnosticLogPort, LogSeverity, OperationsError,
};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::platform::logging::{self, LogEntry};
use crate::test_support::TempDirectory;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Default)]
struct CapturingLogs(Mutex<Vec<DiagnosticLog>>);

impl DiagnosticLogPort for CapturingLogs {
    fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), OperationsError> {
        self.0.lock().expect("logs").push(log);
        Ok(())
    }
}

fn private_fixture() -> LspPrivateDiagnosticData {
    LspPrivateDiagnosticData {
        raw_protocol_payload: Some("raw-protocol-payload-secret".to_string()),
        diagnostic_message: Some("diagnostic-message-secret".to_string()),
        hover_content: Some("hover-content-secret".to_string()),
        source_content: Some("source-content-secret".to_string()),
        stderr: Some("stderr-secret".to_string()),
        environment: BTreeMap::from([("API_TOKEN".to_string(), "environment-secret".to_string())]),
        arguments: vec!["--token=argument-secret".to_string()],
        credential: Some("credential-secret".to_string()),
        absolute_path: Some("C:\\Users\\private-user\\secret-workspace".to_string()),
    }
}

fn event(kind: LspDiagnosticKind) -> LspDiagnosticEvent {
    LspDiagnosticEvent {
        identity: LspDiagnosticIdentity {
            language: registry::rust(),
            workspace_id: Some("workspace-safe-7".to_string()),
            correlation_id: Some("execution-safe-9".to_string()),
        },
        kind,
        private: private_fixture(),
    }
}

fn all_events() -> Vec<LspDiagnosticEvent> {
    vec![
        event(LspDiagnosticKind::Lifecycle {
            from: ProcessState::Starting,
            to: ProcessState::Initializing,
        }),
        event(LspDiagnosticKind::ProtocolLimit {
            method: LspMethodCategory::Initialize,
            duration_ms: 12,
            observed_bytes: 1_048_577,
        }),
        event(LspDiagnosticKind::Timeout {
            method: LspMethodCategory::SemanticQuery,
            duration_ms: 10_000,
            server_state: ProcessState::Ready,
        }),
        event(LspDiagnosticKind::Cancellation {
            method: LspMethodCategory::SemanticQuery,
            duration_ms: 7,
            server_state: ProcessState::Ready,
        }),
        event(LspDiagnosticKind::Crash {
            exit_code: Some(17),
            restart_attempt: 2,
            reason: LspCrashReason::UnexpectedExit,
        }),
        event(LspDiagnosticKind::Restart { restart_attempt: 3 }),
        event(LspDiagnosticKind::DiagnosticsCount { count: 11 }),
        event(LspDiagnosticKind::Shutdown {
            forced: true,
            process_count: 2,
            duration_ms: 900,
        }),
    ]
}

#[test]
fn every_lsp_event_keeps_bounded_safe_metadata() {
    let capture = Arc::new(CapturingLogs::default());
    let logger = LspDiagnosticLogger::new(capture.clone());
    for event in all_events() {
        logger.record(event);
    }

    let logs = capture.0.lock().expect("logs");
    assert_eq!(logs.len(), 8);
    assert_eq!(
        logs.iter().map(|log| log.severity).collect::<Vec<_>>(),
        vec![
            LogSeverity::Info,
            LogSeverity::Error,
            LogSeverity::Warn,
            LogSeverity::Info,
            LogSeverity::Error,
            LogSeverity::Warn,
            LogSeverity::Info,
            LogSeverity::Info,
        ]
    );
    let rendered = format!("{logs:?}");
    for safe_value in [
        "rust",
        "rust_analyzer",
        "workspace-safe-7",
        "execution-safe-9",
        "starting",
        "initializing",
        "protocol_limit",
        "timeout",
        "cancelled",
        "crash",
        "unexpected_exit",
        "restart",
        "diagnostics_count",
        "shutdown",
        "1048577",
        "10000",
        "17",
        "900",
    ] {
        assert!(
            rendered.contains(safe_value),
            "missing safe metadata {safe_value}"
        );
    }
}

#[test]
fn private_lsp_material_never_reaches_the_unified_log() {
    let directory = TempDirectory::new("lsp-unified-log-redaction");
    let unified = Arc::new(UnifiedLoggingAdapter::new(directory.path().to_path_buf()));
    let logger = LspDiagnosticLogger::new(unified);
    for event in all_events() {
        logger.record(event);
    }

    let raw = std::fs::read_to_string(directory.path().join(logging::LOG_FILE_NAME))
        .expect("unified LSP log");
    let entries = raw
        .lines()
        .map(|line| serde_json::from_str::<LogEntry>(line).expect("LSP log entry"))
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 8);
    for secret in [
        "raw-protocol-payload-secret",
        "diagnostic-message-secret",
        "hover-content-secret",
        "source-content-secret",
        "stderr-secret",
        "environment-secret",
        "argument-secret",
        "credential-secret",
        "private-user",
        "secret-workspace",
    ] {
        assert!(!raw.contains(secret), "unified LSP log leaked {secret}");
    }
    let allowed_keys = [
        "server",
        "language",
        "workspaceId",
        "correlationId",
        "event",
        "fromState",
        "toState",
        "methodCategory",
        "durationMs",
        "serverState",
        "observedBytes",
        "exitCode",
        "restartAttempt",
        "reasonCategory",
        "diagnosticCount",
        "forced",
        "processCount",
        "suppressedCount",
    ];
    assert!(entries
        .iter()
        .flat_map(|entry| entry.context.keys())
        .all(|key| { allowed_keys.contains(&key.as_str()) }));
}

#[test]
fn repeated_failures_are_rate_limited_and_aggregated() {
    let capture = Arc::new(CapturingLogs::default());
    let logger = LspDiagnosticLogger::with_rate_limit(capture.clone(), 2, Duration::from_secs(60));
    let timeout = || {
        event(LspDiagnosticKind::Timeout {
            method: LspMethodCategory::SemanticQuery,
            duration_ms: 10_000,
            server_state: ProcessState::Ready,
        })
    };

    for _ in 0..5 {
        logger.record_at(timeout(), Duration::ZERO);
    }
    logger.record_at(timeout(), Duration::from_secs(61));
    for _ in 0..4 {
        logger.record_at(
            event(LspDiagnosticKind::Crash {
                exit_code: Some(17),
                restart_attempt: 3,
                reason: LspCrashReason::ProtocolFailure,
            }),
            Duration::from_secs(61),
        );
    }
    logger.record_at(
        event(LspDiagnosticKind::Lifecycle {
            from: ProcessState::Backoff,
            to: ProcessState::Failed,
        }),
        Duration::from_secs(61),
    );

    let logs = capture.0.lock().expect("logs");
    assert_eq!(logs.len(), 6);
    assert!(logs.iter().any(|log| {
        log.context.get("event").map(String::as_str) == Some("timeout")
            && log.context.get("suppressedCount").map(String::as_str) == Some("3")
    }));
    let exhausted = logs.last().expect("restart-exhausted log");
    assert_eq!(
        exhausted.context.get("toState").map(String::as_str),
        Some("failed")
    );
    assert_eq!(
        exhausted.context.get("suppressedCount").map(String::as_str),
        Some("2")
    );
}
