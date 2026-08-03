use crate::contexts::tooling::mcp::domain::McpFailureCode;
use serde::Serialize;
use serde_json::Value;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct McpRuntimeError {
    code: McpFailureCode,
    diagnostic: Option<String>,
}

impl McpRuntimeError {
    pub(crate) fn new(code: McpFailureCode) -> Self {
        Self {
            code,
            diagnostic: None,
        }
    }

    pub(crate) fn with_diagnostic(code: McpFailureCode, diagnostic: impl Into<String>) -> Self {
        Self {
            code,
            diagnostic: Some(diagnostic.into()),
        }
    }

    pub(crate) fn code(&self) -> McpFailureCode {
        self.code
    }

    #[cfg(test)]
    pub(crate) fn diagnostic(&self) -> Option<&str> {
        self.diagnostic.as_deref()
    }
}

impl fmt::Display for McpRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.code.safe_message())
    }
}

impl std::error::Error for McpRuntimeError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct McpLimits {
    pub(crate) import_document_bytes: usize,
    pub(crate) import_server_entries: usize,
    pub(crate) configuration_collection_entries: usize,
    pub(crate) configuration_serialized_bytes: usize,
    pub(crate) protocol_message_bytes: usize,
    pub(crate) tools_per_server: usize,
    pub(crate) catalog_serialized_bytes: usize,
    pub(crate) provider_tools: usize,
    pub(crate) tool_name_bytes: usize,
    pub(crate) tool_description_bytes: usize,
    pub(crate) schema_bytes: usize,
    pub(crate) json_depth: usize,
    pub(crate) tool_arguments_bytes: usize,
    pub(crate) tool_result_bytes: usize,
    pub(crate) stderr_bytes: usize,
}

impl McpLimits {
    pub(crate) const DEFAULT: Self = Self {
        import_document_bytes: 1024 * 1024,
        import_server_entries: 128,
        configuration_collection_entries: 128,
        configuration_serialized_bytes: 256 * 1024,
        protocol_message_bytes: 2 * 1024 * 1024,
        tools_per_server: 128,
        catalog_serialized_bytes: 2 * 1024 * 1024,
        provider_tools: 256,
        tool_name_bytes: 256,
        tool_description_bytes: 8 * 1024,
        schema_bytes: 128 * 1024,
        json_depth: 32,
        tool_arguments_bytes: 256 * 1024,
        tool_result_bytes: 1024 * 1024,
        stderr_bytes: 64 * 1024,
    };

    pub(crate) fn validate_bytes(
        &self,
        label: &'static str,
        actual: usize,
        maximum: usize,
    ) -> Result<(), McpRuntimeError> {
        if actual <= maximum {
            Ok(())
        } else {
            Err(limit_error(label, actual, maximum))
        }
    }

    pub(crate) fn validate_count(
        &self,
        label: &'static str,
        actual: usize,
        maximum: usize,
    ) -> Result<(), McpRuntimeError> {
        self.validate_bytes(label, actual, maximum)
    }

    pub(crate) fn validate_serialized<T: Serialize>(
        &self,
        label: &'static str,
        value: &T,
        maximum: usize,
    ) -> Result<usize, McpRuntimeError> {
        let size = serde_json::to_vec(value)
            .map_err(|error| {
                McpRuntimeError::with_diagnostic(McpFailureCode::Validation, error.to_string())
            })?
            .len();
        self.validate_bytes(label, size, maximum)?;
        Ok(size)
    }

    pub(crate) fn validate_json(
        &self,
        label: &'static str,
        value: &Value,
        maximum_bytes: usize,
        maximum_depth: usize,
    ) -> Result<(), McpRuntimeError> {
        self.validate_serialized(label, value, maximum_bytes)?;
        let depth = json_depth(value);
        if depth <= maximum_depth {
            Ok(())
        } else {
            Err(limit_error(label, depth, maximum_depth))
        }
    }
}

impl Default for McpLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

fn limit_error(label: &'static str, actual: usize, maximum: usize) -> McpRuntimeError {
    McpRuntimeError::with_diagnostic(
        McpFailureCode::LimitExceeded,
        format!("{label} exceeded its limit: {actual} > {maximum}"),
    )
}

fn json_depth(value: &Value) -> usize {
    let mut deepest = 1;
    let mut pending = vec![(value, 1)];
    while let Some((current, depth)) = pending.pop() {
        deepest = deepest.max(depth);
        match current {
            Value::Array(items) => {
                pending.extend(items.iter().map(|item| (item, depth + 1)));
            }
            Value::Object(entries) => {
                pending.extend(entries.values().map(|item| (item, depth + 1)));
            }
            _ => {}
        }
    }
    deepest
}

#[derive(Debug, Clone, Default)]
pub(crate) struct McpCancellation {
    cancelled: Arc<AtomicBool>,
}

impl McpCancellation {
    pub(crate) fn from_shared(cancelled: Arc<AtomicBool>) -> Self {
        Self { cancelled }
    }

    #[cfg(test)]
    pub(crate) fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct McpExecutionControl {
    deadline: Instant,
    cancellation: McpCancellation,
}

impl McpExecutionControl {
    pub(crate) fn with_timeout(timeout: Duration) -> Self {
        Self::with_deadline(Instant::now() + timeout)
    }

    pub(crate) fn with_deadline(deadline: Instant) -> Self {
        Self {
            deadline,
            cancellation: McpCancellation::default(),
        }
    }

    pub(crate) fn with_timeout_and_cancellation(
        timeout: Duration,
        cancellation: McpCancellation,
    ) -> Self {
        Self {
            deadline: Instant::now() + timeout,
            cancellation,
        }
    }

    pub(crate) fn cancellation(&self) -> McpCancellation {
        self.cancellation.clone()
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    pub(crate) fn remaining(&self) -> Result<Duration, McpRuntimeError> {
        if self.is_cancelled() {
            return Err(McpRuntimeError::new(McpFailureCode::Cancelled));
        }
        self.deadline_remaining()
    }

    pub(crate) fn deadline_remaining(&self) -> Result<Duration, McpRuntimeError> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| McpRuntimeError::new(McpFailureCode::Timeout))
    }
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod tests;
