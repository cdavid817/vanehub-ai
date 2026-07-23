use super::{ExecutionDomainError, ExecutionRunId, SafeAttributes, SpanId, TraceId};

const MAX_SPAN_NAME_LENGTH: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionStatus {
    Accepted,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Incomplete,
}

impl ExecutionStatus {
    pub(crate) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::Incomplete
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExecutionFidelity {
    Native,
    Proxied,
    Inferred,
    Opaque,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CapturePolicy {
    MetadataOnly,
    RedactedContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExecutionSource {
    Desktop,
    InstantMessage { connector_id: String },
    Scheduled { task_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionLink {
    pub(crate) run_id: ExecutionRunId,
    pub(crate) trace_id: TraceId,
    pub(crate) span_id: Option<SpanId>,
    pub(crate) relationship: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionContext {
    pub(crate) run_id: ExecutionRunId,
    pub(crate) trace_id: TraceId,
    pub(crate) span_id: SpanId,
    pub(crate) capture_policy: CapturePolicy,
    pub(crate) sampling_per_million: u32,
    pub(crate) mcp_relay_enabled: bool,
}

impl ExecutionContext {
    pub(crate) fn traceparent(&self) -> String {
        format!("00-{}-{}-01", self.trace_id.as_str(), self.span_id.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExecutionRun {
    pub(crate) context: ExecutionContext,
    pub(crate) source: ExecutionSource,
    pub(crate) status: ExecutionStatus,
    pub(crate) started_at: String,
    pub(crate) ended_at: Option<String>,
    pub(crate) error_classification: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) user_message_id: Option<String>,
    pub(crate) assistant_message_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) agent_id: Option<String>,
    pub(crate) provider_session_id: Option<String>,
    pub(crate) attributes: SafeAttributes,
    pub(crate) links: Vec<ExecutionLink>,
}

impl ExecutionRun {
    pub(crate) fn validate(&self) -> Result<(), ExecutionDomainError> {
        require_timestamp(&self.started_at)?;
        if let Some(ended_at) = &self.ended_at {
            require_timestamp(ended_at)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExecutionSpan {
    pub(crate) context: ExecutionContext,
    pub(crate) parent_span_id: Option<SpanId>,
    pub(crate) name: String,
    pub(crate) status: ExecutionStatus,
    pub(crate) fidelity: ExecutionFidelity,
    pub(crate) started_at: String,
    pub(crate) ended_at: Option<String>,
    pub(crate) error_classification: Option<String>,
    pub(crate) attributes: SafeAttributes,
    pub(crate) links: Vec<ExecutionLink>,
}

impl ExecutionSpan {
    pub(crate) fn validate(&self) -> Result<(), ExecutionDomainError> {
        if self.name.trim().is_empty() || self.name.chars().count() > MAX_SPAN_NAME_LENGTH {
            return Err(ExecutionDomainError::InvalidSpanName {
                max: MAX_SPAN_NAME_LENGTH,
            });
        }
        require_timestamp(&self.started_at)?;
        if let Some(ended_at) = &self.ended_at {
            require_timestamp(ended_at)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExecutionEvent {
    pub(crate) run_id: ExecutionRunId,
    pub(crate) span_id: SpanId,
    pub(crate) sequence: u64,
    pub(crate) name: String,
    pub(crate) timestamp: String,
    pub(crate) attributes: SafeAttributes,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExecutionTimeline {
    pub(crate) run: ExecutionRun,
    pub(crate) spans: Vec<ExecutionSpan>,
    pub(crate) events: Vec<ExecutionEvent>,
}

impl ExecutionEvent {
    pub(crate) fn validate(&self) -> Result<(), ExecutionDomainError> {
        if self.name.trim().is_empty() || self.name.chars().count() > MAX_SPAN_NAME_LENGTH {
            return Err(ExecutionDomainError::InvalidSpanName {
                max: MAX_SPAN_NAME_LENGTH,
            });
        }
        require_timestamp(&self.timestamp)
    }
}

fn require_timestamp(value: &str) -> Result<(), ExecutionDomainError> {
    if value.trim().is_empty() {
        Err(ExecutionDomainError::TimestampRequired)
    } else {
        Ok(())
    }
}
