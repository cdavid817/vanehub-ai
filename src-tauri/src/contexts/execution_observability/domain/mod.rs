mod attributes;
mod error;
mod identity;
mod model;
mod pagination;
mod settings;

pub(crate) use attributes::{SafeAttributeValue, SafeAttributes};
pub(crate) use error::ExecutionDomainError;
pub(crate) use identity::{ExecutionRunId, SpanId, TraceId};
pub(crate) use model::{
    CapturePolicy, ExecutionContext, ExecutionEvent, ExecutionFidelity, ExecutionLink,
    ExecutionRun, ExecutionSource, ExecutionSpan, ExecutionStatus, ExecutionTimeline,
};
pub(crate) use pagination::{Page, PageRequest};
pub(crate) use settings::{
    ExecutionObservationCapability, McpTransport, ObservabilitySettings, OtlpProtocol,
};

#[cfg(test)]
mod tests;
