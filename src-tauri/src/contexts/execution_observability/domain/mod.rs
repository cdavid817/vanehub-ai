mod attributes;
mod error;
mod evaluation;
mod evaluation_manifest;
pub(crate) mod evidence;
mod identity;
mod model;
mod pagination;
mod settings;
mod span_kind;
#[cfg(test)]
mod span_kind_tests;

pub(crate) use attributes::{SafeAttributeValue, SafeAttributes};
pub(crate) use error::ExecutionDomainError;
pub(crate) use evaluation::*;
pub(crate) use evaluation_manifest::*;
pub(crate) use evidence::*;
pub(crate) use identity::{ExecutionRunId, SpanId, TraceId};
pub(crate) use model::{
    CapturePolicy, ExecutionContext, ExecutionEvent, ExecutionFidelity, ExecutionLink,
    ExecutionRun, ExecutionSource, ExecutionSpan, ExecutionStatus, ExecutionTimeline,
};
pub(crate) use pagination::{Page, PageRequest};
pub(crate) use settings::{
    ExecutionObservationCapability, McpTransport, ObservabilitySettings, OtlpProtocol,
};
pub(crate) use span_kind::{classify_span_kind, ExecutionSpanKind};

#[cfg(test)]
mod tests;
