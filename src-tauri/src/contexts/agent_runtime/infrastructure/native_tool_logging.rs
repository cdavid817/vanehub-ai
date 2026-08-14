#![allow(dead_code)]

use crate::contexts::agent_runtime::application::{
    NativeToolLogEvent, NativeToolLogEventKind, NativeToolSafeLogMetadata,
};
use crate::contexts::operations::api::{LogSeverity, OperationLog, OperationLogPort};
use crate::platform::logging::redact_text;
use std::collections::BTreeMap;
use std::sync::Arc;

const MAX_SAFE_VALUE_CHARS: usize = 128;

#[derive(Clone)]
pub(crate) struct NativeToolLogger {
    logging: Arc<dyn OperationLogPort>,
}

impl NativeToolLogger {
    pub(crate) fn new(logging: Arc<dyn OperationLogPort>) -> Self {
        Self { logging }
    }

    pub(crate) fn record(&self, event: NativeToolLogEvent) {
        let _ = self.logging.write_operation(build_log(event));
    }
}

fn build_log(event: NativeToolLogEvent) -> OperationLog {
    let NativeToolLogEvent {
        identity,
        kind,
        safe,
        private,
    } = event;
    drop(private);
    let mut context = BTreeMap::from([
        ("event".to_string(), kind.as_str().to_string()),
        ("callId".to_string(), sanitize(identity.call_id)),
        (
            "executionRunId".to_string(),
            sanitize(identity.execution_run_id),
        ),
        ("agentId".to_string(), sanitize(identity.agent_id)),
        ("sessionId".to_string(), sanitize(identity.session_id)),
        ("toolName".to_string(), sanitize(identity.tool_name)),
    ]);
    append_safe_metadata(&mut context, safe);
    OperationLog {
        operation_id: sanitize(identity.operation_id),
        severity: severity(kind),
        category: "agent_runtime.native_tool".to_string(),
        message: "Native tool operation observation".to_string(),
        context,
    }
}

fn append_safe_metadata(
    context: &mut BTreeMap<String, String>,
    metadata: NativeToolSafeLogMetadata,
) {
    insert_optional(context, "operation", metadata.operation);
    insert_optional(context, "outcome", metadata.outcome);
    insert_optional(context, "reasonCode", metadata.reason_code);
    insert_optional(context, "resourceHash", metadata.resource_hash);
    insert_optional(context, "target", metadata.target);
    insert_optional(context, "mode", metadata.mode);
    insert_number(context, "durationMs", metadata.duration_ms);
    insert_number(context, "observedCount", metadata.observed_count);
    insert_number(context, "limitCount", metadata.limit_count);
}

fn insert_optional(context: &mut BTreeMap<String, String>, key: &str, value: Option<String>) {
    if let Some(value) = value {
        context.insert(key.to_string(), sanitize(value));
    }
}

fn insert_number(context: &mut BTreeMap<String, String>, key: &str, value: Option<u64>) {
    if let Some(value) = value {
        context.insert(key.to_string(), value.to_string());
    }
}

fn sanitize(value: String) -> String {
    redact_text(&value)
        .chars()
        .take(MAX_SAFE_VALUE_CHARS)
        .collect()
}

const fn severity(kind: NativeToolLogEventKind) -> LogSeverity {
    match kind {
        NativeToolLogEventKind::Progress => LogSeverity::Debug,
        NativeToolLogEventKind::Denied
        | NativeToolLogEventKind::LimitExceeded
        | NativeToolLogEventKind::ReadinessUnavailable => LogSeverity::Warn,
        NativeToolLogEventKind::ExternalProcessFailed | NativeToolLogEventKind::Failed => {
            LogSeverity::Error
        }
        NativeToolLogEventKind::Started
        | NativeToolLogEventKind::AwaitingApproval
        | NativeToolLogEventKind::Completed
        | NativeToolLogEventKind::Cancelled => LogSeverity::Info,
    }
}

#[cfg(test)]
#[path = "native_tool_logging_tests.rs"]
mod tests;
