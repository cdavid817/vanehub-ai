use super::NativeToolLogger;
use crate::contexts::agent_runtime::application::{
    NativeToolLogEvent, NativeToolLogEventKind, NativeToolLogIdentity, NativeToolPrivateLogData,
    NativeToolSafeLogMetadata,
};
use crate::contexts::operations::api::{
    LogSeverity, OperationLog, OperationLogPort, OperationsError,
};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::platform::logging;
use crate::test_support::TempDirectory;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct CapturingLogs(Mutex<Vec<OperationLog>>);

impl OperationLogPort for CapturingLogs {
    fn write_operation(&self, log: OperationLog) -> Result<(), OperationsError> {
        self.0.lock().expect("logs").push(log);
        Ok(())
    }
}

fn event(kind: NativeToolLogEventKind) -> NativeToolLogEvent {
    NativeToolLogEvent {
        identity: NativeToolLogIdentity {
            operation_id: "operation-safe-1".to_string(),
            call_id: "call-safe-2".to_string(),
            execution_run_id: "run-safe-3".to_string(),
            agent_id: "onepiece".to_string(),
            session_id: "session-safe-4".to_string(),
            tool_name: "delegate_cli".to_string(),
        },
        kind,
        safe: NativeToolSafeLogMetadata {
            operation: Some("delegate.analyze".to_string()),
            outcome: Some("failed".to_string()),
            reason_code: Some("external_failure token=safe-field-secret".to_string()),
            resource_hash: Some("sha256:safe-resource-hash".to_string()),
            target: Some("codex_cli".to_string()),
            mode: Some("analyze".to_string()),
            duration_ms: Some(250),
            observed_count: Some(7),
            limit_count: Some(10),
        },
        private: private_fixture(),
    }
}

fn private_fixture() -> NativeToolPrivateLogData {
    NativeToolPrivateLogData {
        raw_input: Some("raw-input-secret".to_string()),
        raw_output: Some("raw-output-secret".to_string()),
        path: Some("C:\\Users\\private-user\\workspace-secret".to_string()),
        url: Some("https://private-user:url-secret@example.invalid/private".to_string()),
        prompt: Some("prompt-secret".to_string()),
        credential: Some("credential-secret".to_string()),
        external_process_error: Some("stderr-process-secret".to_string()),
        environment: BTreeMap::from([("API_TOKEN".to_string(), "environment-secret".to_string())]),
        headers: BTreeMap::from([(
            "Authorization".to_string(),
            "Bearer header-secret".to_string(),
        )]),
    }
}

#[test]
fn private_tool_material_is_removed_before_the_logging_port() {
    let capture = Arc::new(CapturingLogs::default());
    NativeToolLogger::new(capture.clone()).record(event(NativeToolLogEventKind::Failed));

    let logs = capture.0.lock().expect("logs");
    let rendered = format!("{:?}", logs.as_slice());
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].severity, LogSeverity::Error);
    for secret in private_secrets() {
        assert!(
            !rendered.contains(secret),
            "pre-persistence log leaked a protected value"
        );
    }
    assert!(!rendered.contains("safe-field-secret"));
    assert!(rendered.contains("[REDACTED]"));
}

#[test]
fn event_definitions_preserve_only_bounded_safe_correlations() {
    let capture = Arc::new(CapturingLogs::default());
    let logger = NativeToolLogger::new(capture.clone());
    for kind in all_kinds() {
        logger.record(event(kind));
    }

    let logs = capture.0.lock().expect("logs");
    assert_eq!(logs.len(), all_kinds().len());
    assert_eq!(
        logs.iter().map(|log| log.severity).collect::<Vec<_>>(),
        vec![
            LogSeverity::Info,
            LogSeverity::Debug,
            LogSeverity::Info,
            LogSeverity::Warn,
            LogSeverity::Info,
            LogSeverity::Info,
            LogSeverity::Warn,
            LogSeverity::Warn,
            LogSeverity::Error,
            LogSeverity::Error,
        ]
    );
    assert!(logs.iter().all(|log| {
        log.operation_id == "operation-safe-1"
            && log.category == "agent_runtime.native_tool"
            && log.context.get("agentId").map(String::as_str) == Some("onepiece")
            && log.context.get("resourceHash").map(String::as_str)
                == Some("sha256:safe-resource-hash")
    }));
    let allowed_keys = [
        "event",
        "callId",
        "executionRunId",
        "agentId",
        "sessionId",
        "toolName",
        "operation",
        "outcome",
        "reasonCode",
        "resourceHash",
        "target",
        "mode",
        "durationMs",
        "observedCount",
        "limitCount",
    ];
    assert!(logs
        .iter()
        .flat_map(|log| log.context.keys())
        .all(|key| allowed_keys.contains(&key.as_str())));
}

#[test]
fn unified_persistence_keeps_the_pre_persistence_privacy_boundary() {
    let directory = TempDirectory::new("native-tool-unified-log");
    let unified = Arc::new(UnifiedLoggingAdapter::new(directory.path().to_path_buf()));
    NativeToolLogger::new(unified).record(event(NativeToolLogEventKind::ExternalProcessFailed));

    let raw = std::fs::read_to_string(directory.path().join(logging::LOG_FILE_NAME))
        .expect("native tool log");
    for secret in private_secrets().into_iter().chain(["safe-field-secret"]) {
        assert!(
            !raw.contains(secret),
            "persisted log leaked a protected value"
        );
    }
    assert!(raw.contains("external_process_failed"));
    assert!(raw.contains("operation-safe-1"));
    assert!(raw.contains("[REDACTED]"));
}

#[test]
fn recognized_ocr_content_never_crosses_the_logging_boundary() {
    let capture = Arc::new(CapturingLogs::default());
    let mut ocr_event = event(NativeToolLogEventKind::Completed);
    ocr_event.identity.tool_name = "ocr".to_string();
    ocr_event.private.raw_input = Some("private scanned invoice".to_string());
    ocr_event.private.raw_output = Some("account 998877 secret total".to_string());
    NativeToolLogger::new(capture.clone()).record(ocr_event);

    let rendered = format!("{:?}", capture.0.lock().expect("logs").as_slice());
    assert!(rendered.contains("ocr"));
    assert!(!rendered.contains("private scanned invoice"));
    assert!(!rendered.contains("account 998877 secret total"));
}

#[test]
fn durable_log_redaction_matrix_excludes_every_private_content_class() {
    let directory = TempDirectory::new("native-tool-redaction-matrix");
    let unified = Arc::new(UnifiedLoggingAdapter::new(directory.path().to_path_buf()));
    let mut sensitive = event(NativeToolLogEventKind::ExternalProcessFailed);
    sensitive.private = NativeToolPrivateLogData {
        raw_input: Some("page-body-secret file-body-secret".to_string()),
        raw_output: Some("ocr-text-secret full-external-transcript-secret".to_string()),
        path: Some("C:\\private\\file-body-secret.txt".to_string()),
        url: Some("https://example.invalid/page-body-secret".to_string()),
        prompt: Some("delegation-prompt-secret".to_string()),
        credential: Some("raw-credential-secret".to_string()),
        external_process_error: Some("hidden-reasoning-secret".to_string()),
        environment: BTreeMap::from([(
            "PROVIDER_TOKEN".to_string(),
            "environment-credential-secret".to_string(),
        )]),
        headers: BTreeMap::from([(
            "Authorization".to_string(),
            "Bearer authorization-header-secret".to_string(),
        )]),
    };
    NativeToolLogger::new(unified).record(sensitive);

    let raw = std::fs::read_to_string(directory.path().join(logging::LOG_FILE_NAME))
        .expect("durable native tool log");
    for secret in [
        "page-body-secret",
        "file-body-secret",
        "ocr-text-secret",
        "full-external-transcript-secret",
        "delegation-prompt-secret",
        "raw-credential-secret",
        "environment-credential-secret",
        "authorization-header-secret",
        "hidden-reasoning-secret",
    ] {
        assert!(
            !raw.contains(secret),
            "durable log leaked a protected value"
        );
    }
    assert!(raw.contains("external_process_failed"));
}

fn all_kinds() -> Vec<NativeToolLogEventKind> {
    vec![
        NativeToolLogEventKind::Started,
        NativeToolLogEventKind::Progress,
        NativeToolLogEventKind::AwaitingApproval,
        NativeToolLogEventKind::Denied,
        NativeToolLogEventKind::Completed,
        NativeToolLogEventKind::Cancelled,
        NativeToolLogEventKind::LimitExceeded,
        NativeToolLogEventKind::ReadinessUnavailable,
        NativeToolLogEventKind::ExternalProcessFailed,
        NativeToolLogEventKind::Failed,
    ]
}

fn private_secrets() -> [&'static str; 11] {
    [
        "raw-input-secret",
        "raw-output-secret",
        "private-user",
        "workspace-secret",
        "url-secret",
        "prompt-secret",
        "credential-secret",
        "stderr-process-secret",
        "environment-secret",
        "header-secret",
        "example.invalid",
    ]
}
