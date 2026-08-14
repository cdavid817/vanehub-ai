use crate::contexts::operations::api::{LogSeverity, OperationLog, OperationLogPort};
use crate::platform::logging::redact_text;
use std::collections::BTreeMap;
use std::sync::Arc;

const MAX_SAFE_VALUE_CHARS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationLogEvent {
    pub(crate) operation_id: String,
    pub(crate) delegation_id: String,
    pub(crate) attempt_id: Option<String>,
    pub(crate) target: String,
    pub(crate) mode: String,
    pub(crate) state: String,
    pub(crate) reason_code: Option<String>,
    pub(crate) executable_sha256: Option<String>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) raw_task: Option<String>,
    pub(crate) raw_context: Option<String>,
    pub(crate) raw_transcript: Option<String>,
    pub(crate) raw_diagnostic: Option<String>,
}

#[derive(Clone)]
pub(crate) struct DelegationLogger {
    logging: Arc<dyn OperationLogPort>,
}

impl DelegationLogger {
    pub(crate) fn new(logging: Arc<dyn OperationLogPort>) -> Self {
        Self { logging }
    }

    pub(crate) fn record(&self, event: DelegationLogEvent) {
        let _ = self.logging.write_operation(build_log(event));
    }
}

fn build_log(event: DelegationLogEvent) -> OperationLog {
    let DelegationLogEvent {
        operation_id,
        delegation_id,
        attempt_id,
        target,
        mode,
        state,
        reason_code,
        executable_sha256,
        duration_ms,
        raw_task,
        raw_context,
        raw_transcript,
        raw_diagnostic,
    } = event;
    drop((raw_task, raw_context, raw_transcript, raw_diagnostic));
    let mut context = BTreeMap::from([
        ("delegationId".to_owned(), sanitize(delegation_id)),
        ("target".to_owned(), sanitize(target)),
        ("mode".to_owned(), sanitize(mode)),
        ("state".to_owned(), sanitize(state.clone())),
    ]);
    insert(&mut context, "attemptId", attempt_id);
    insert(&mut context, "reasonCode", reason_code);
    insert(&mut context, "executableSha256", executable_sha256);
    if let Some(duration_ms) = duration_ms {
        context.insert("durationMs".to_owned(), duration_ms.to_string());
    }
    OperationLog {
        operation_id: sanitize(operation_id),
        severity: severity(&state),
        category: "agent_runtime.cli_delegation".to_owned(),
        message: "CLI delegation observation".to_owned(),
        context,
    }
}

fn insert(context: &mut BTreeMap<String, String>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        context.insert(key.to_owned(), sanitize(value));
    }
}

fn sanitize(value: String) -> String {
    redact_text(&value)
        .chars()
        .take(MAX_SAFE_VALUE_CHARS)
        .collect()
}

fn severity(state: &str) -> LogSeverity {
    match state {
        "failed" | "cleanup_failed" | "interrupted" => LogSeverity::Error,
        "cancelled" | "limit_exceeded" | "circuit_open" => LogSeverity::Warn,
        "progress" => LogSeverity::Debug,
        _ => LogSeverity::Info,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::api::{OperationLogPort, OperationsError};
    use std::sync::Mutex;

    #[derive(Default)]
    struct Capture(Mutex<Vec<OperationLog>>);

    impl OperationLogPort for Capture {
        fn write_operation(&self, log: OperationLog) -> Result<(), OperationsError> {
            self.0.lock().expect("logs").push(log);
            Ok(())
        }
    }

    #[test]
    fn persistence_receives_only_bounded_safe_delegation_metadata() {
        let capture = Arc::new(Capture::default());
        DelegationLogger::new(capture.clone()).record(DelegationLogEvent {
            operation_id: "operation-1".to_owned(),
            delegation_id: "delegation-1".to_owned(),
            attempt_id: Some("attempt-1".to_owned()),
            target: "codex-cli".to_owned(),
            mode: "edit".to_owned(),
            state: "failed".to_owned(),
            reason_code: Some("protocol_invalid".to_owned()),
            executable_sha256: Some("a".repeat(64)),
            duration_ms: Some(15),
            raw_task: Some("private task secret".to_owned()),
            raw_context: Some("private source secret".to_owned()),
            raw_transcript: Some("hidden reasoning secret".to_owned()),
            raw_diagnostic: Some("token=credential-secret".to_owned()),
        });
        let logs = capture.0.lock().expect("logs");
        let rendered = format!("{:?}", logs.as_slice());
        assert_eq!(logs[0].severity, LogSeverity::Error);
        assert!(rendered.contains("protocol_invalid"));
        for secret in [
            "private task secret",
            "private source secret",
            "hidden reasoning secret",
            "credential-secret",
        ] {
            assert!(!rendered.contains(secret));
        }
    }
}
