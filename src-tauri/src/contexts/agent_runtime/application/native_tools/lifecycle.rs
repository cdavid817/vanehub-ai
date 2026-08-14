use super::{
    NativeToolErrorCode, NativeToolExecutionContext, NativeToolHandler, NativeToolLimitProfile,
    NativeToolProgress, NativeToolProgressSink, NativeToolResultEnvelope, NativeToolResultStatus,
    ValidatedNativeToolInput, NATIVE_TOOL_CONTRACT_VERSION,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_SAFE_ERROR_CHARS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperationState {
    Running,
    Terminal,
}

#[derive(Debug)]
struct BoundedProgressSink {
    inner: Arc<dyn NativeToolProgressSink>,
    max_events: u32,
    state: Mutex<ProgressState>,
}

#[derive(Debug, Default)]
struct ProgressState {
    accepted: u32,
    last_sequence: Option<u32>,
    violated: bool,
}

impl BoundedProgressSink {
    fn new(inner: Arc<dyn NativeToolProgressSink>, max_events: u32) -> Self {
        Self {
            inner,
            max_events,
            state: Mutex::new(ProgressState::default()),
        }
    }

    fn violated(&self) -> bool {
        self.state
            .lock()
            .map(|state| state.violated)
            .unwrap_or(true)
    }
}

impl NativeToolProgressSink for BoundedProgressSink {
    fn publish(&self, progress: NativeToolProgress) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let monotonic = state
            .last_sequence
            .is_none_or(|sequence| progress.sequence > sequence);
        if !monotonic || state.accepted >= self.max_events {
            state.violated = true;
            return;
        }
        state.accepted += 1;
        state.last_sequence = Some(progress.sequence);
        drop(state);
        self.inner.publish(progress);
    }
}

pub(super) fn execute_with_lifecycle(
    handler: &dyn NativeToolHandler,
    input: ValidatedNativeToolInput,
    mut context: NativeToolExecutionContext,
    limits: &NativeToolLimitProfile,
) -> NativeToolResultEnvelope {
    let profile_deadline = Instant::now() + Duration::from_millis(limits.max_duration_ms);
    context.deadline = context.deadline.min(profile_deadline);
    let bounded_progress = Arc::new(BoundedProgressSink::new(
        context.progress.clone(),
        limits.max_progress_events,
    ));
    context.progress = bounded_progress.clone();
    let cleanup_context = context.clone();
    let mut state = OperationState::Running;

    let execution = catch_unwind(AssertUnwindSafe(|| handler.execute(input, context)));
    let cleanup = handler.cleanup(&cleanup_context);
    let result = match execution {
        Ok(result) => result,
        Err(_) => failure(
            NativeToolErrorCode::InternalFailure,
            "The native tool failed unexpectedly.",
        ),
    };
    let result = if cleanup.is_err() {
        failure(
            NativeToolErrorCode::InternalFailure,
            "The native tool cleanup failed.",
        )
    } else if cleanup_context.is_cancelled() {
        cancelled()
    } else if cleanup_context.deadline_reached() {
        failure(
            NativeToolErrorCode::DeadlineExceeded,
            "The native tool deadline was reached.",
        )
    } else if bounded_progress.violated() {
        limit_exceeded("The native tool progress limit was reached.")
    } else {
        normalize_result(result, limits.max_output_bytes)
    };
    debug_assert_eq!(state, OperationState::Running);
    state = OperationState::Terminal;
    debug_assert_eq!(state, OperationState::Terminal);
    result
}

fn normalize_result(
    mut result: NativeToolResultEnvelope,
    max_output_bytes: u64,
) -> NativeToolResultEnvelope {
    result.contract_version = NATIVE_TOOL_CONTRACT_VERSION;
    result.safe_error = result.safe_error.map(sanitize_safe_error);
    if serialized_size(&result.output) > max_output_bytes {
        return limit_exceeded("The native tool output limit was reached.");
    }
    result
}

fn serialized_size(output: &Option<Value>) -> u64 {
    output
        .as_ref()
        .and_then(|value| serde_json::to_vec(value).ok())
        .map_or(0, |bytes| bytes.len() as u64)
}

fn sanitize_safe_error(message: String) -> String {
    message
        .chars()
        .filter(|character| !character.is_control())
        .take(MAX_SAFE_ERROR_CHARS)
        .collect()
}

fn cancelled() -> NativeToolResultEnvelope {
    terminal(
        NativeToolResultStatus::Cancelled,
        NativeToolErrorCode::Cancelled,
        "The native tool call was cancelled.",
        false,
    )
}

fn failure(code: NativeToolErrorCode, message: &str) -> NativeToolResultEnvelope {
    terminal(NativeToolResultStatus::Failed, code, message, false)
}

fn limit_exceeded(message: &str) -> NativeToolResultEnvelope {
    terminal(
        NativeToolResultStatus::LimitExceeded,
        NativeToolErrorCode::LimitExceeded,
        message,
        true,
    )
}

fn terminal(
    status: NativeToolResultStatus,
    code: NativeToolErrorCode,
    message: &str,
    truncated: bool,
) -> NativeToolResultEnvelope {
    NativeToolResultEnvelope {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        status,
        output: None,
        error_code: Some(code),
        safe_error: Some(message.to_owned()),
        truncated,
        metadata: BTreeMap::new(),
    }
}

#[cfg(test)]
#[path = "lifecycle_tests.rs"]
mod tests;
