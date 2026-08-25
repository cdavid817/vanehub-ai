use super::dto;
use crate::contexts::execution_observability::api::{
    classify_span_kind, derive_waterfall, CapturePolicy, ExecutionEvent, ExecutionFidelity,
    ExecutionLink, ExecutionObservationCapability, ExecutionRun, ExecutionSource, ExecutionSpan,
    ExecutionStatus, ExecutionTelemetryError, ExecutionTimeline, McpTransport,
    ObservabilitySettings, OtlpProtocol, Page, SafeAttributeValue, SafeAttributes,
    SpanWaterfallMetadata,
};

pub(super) fn settings_to_dto(settings: ObservabilitySettings) -> dto::ObservabilitySettingsDto {
    dto::ObservabilitySettingsDto {
        local_timeline_enabled: settings.local_timeline_enabled,
        otlp_enabled: settings.otlp_enabled,
        otlp_endpoint: settings.otlp_endpoint,
        otlp_protocol: protocol_to_dto(settings.otlp_protocol),
        sampling_ratio: settings.sampling_ratio,
        retention_days: settings.retention_days,
        capture_policy: capture_to_dto(settings.capture_policy),
        mcp_relay_enabled: settings.mcp_relay_enabled,
        otlp_auth_configured: settings.otlp_auth_configured,
        otlp_auth_token: None,
    }
}

pub(super) fn settings_from_dto(
    settings: dto::ObservabilitySettingsDto,
) -> (ObservabilitySettings, Option<String>) {
    let auth_token = settings.otlp_auth_token;
    let domain = ObservabilitySettings {
        local_timeline_enabled: settings.local_timeline_enabled,
        otlp_enabled: settings.otlp_enabled,
        otlp_endpoint: settings.otlp_endpoint,
        otlp_protocol: match settings.otlp_protocol {
            dto::OtlpProtocolDto::HttpProtobuf => OtlpProtocol::HttpProtobuf,
        },
        sampling_ratio: settings.sampling_ratio,
        retention_days: settings.retention_days,
        capture_policy: match settings.capture_policy {
            dto::CapturePolicyDto::MetadataOnly => CapturePolicy::MetadataOnly,
            dto::CapturePolicyDto::RedactedContent => CapturePolicy::RedactedContent,
        },
        mcp_relay_enabled: settings.mcp_relay_enabled,
        otlp_auth_configured: settings.otlp_auth_configured,
    };
    (domain, auth_token)
}

pub(super) fn capability_to_dto(
    capability: ExecutionObservationCapability,
) -> dto::ExecutionObservationCapabilityDto {
    dto::ExecutionObservationCapabilityDto {
        agent_id: capability.agent_id,
        transport: match capability.transport {
            McpTransport::Stdio => dto::McpTransportDto::Stdio,
            McpTransport::Http => dto::McpTransportDto::Http,
        },
        tool_fidelity: fidelity_to_dto(capability.tool_fidelity),
        mcp_fidelity: fidelity_to_dto(capability.mcp_fidelity),
        relay_supported: capability.relay_supported,
        detail: capability.detail,
    }
}

pub(super) fn runs_to_dto(page: Page<ExecutionRun>) -> dto::ExecutionRunPageDto {
    dto::ExecutionRunPageDto {
        items: page.items.into_iter().map(run_to_dto).collect(),
        next_page_token: page.next_page_token,
    }
}

pub(super) fn timeline_to_dto(timeline: ExecutionTimeline) -> dto::ExecutionTimelineDto {
    // Derived once for the whole timeline, because depth, offset and critical path are properties
    // of a span's position among the others — a per-span derivation would only ever see the span
    // it was called for.
    let waterfall = derive_waterfall(&timeline);
    dto::ExecutionTimelineDto {
        run: run_to_dto(timeline.run),
        spans: timeline
            .spans
            .into_iter()
            .map(|span| {
                let derived = waterfall
                    .get(span.context.span_id.as_str())
                    .copied()
                    .unwrap_or_default();
                span_to_dto(span, derived)
            })
            .collect(),
        events: timeline.events.into_iter().map(event_to_dto).collect(),
    }
}

pub(super) fn run_to_dto(run: ExecutionRun) -> dto::ExecutionRunSummaryDto {
    let (source, source_id) = source_to_dto(run.source);
    dto::ExecutionRunSummaryDto {
        run_id: run.context.run_id.as_str().to_string(),
        trace_id: run.context.trace_id.as_str().to_string(),
        root_span_id: run.context.span_id.as_str().to_string(),
        source,
        source_id,
        status: status_to_dto(run.status),
        duration_ms: duration_ms(&run.started_at, run.ended_at.as_deref()),
        started_at: run.started_at,
        ended_at: run.ended_at,
        session_id: run.session_id,
        operation_id: run.operation_id,
        agent_id: run.agent_id,
    }
}

fn span_to_dto(
    span: ExecutionSpan,
    derived: SpanWaterfallMetadata,
) -> dto::ExecutionSpanSummaryDto {
    let kind = classify_span_kind(&span.attributes);
    dto::ExecutionSpanSummaryDto {
        span_id: span.context.span_id.as_str().to_string(),
        parent_span_id: span.parent_span_id.map(|value| value.as_str().to_string()),
        name: span.name,
        kind: kind.token().to_string(),
        status: status_to_dto(span.status),
        fidelity: fidelity_to_dto(span.fidelity),
        duration_ms: duration_ms(&span.started_at, span.ended_at.as_deref()),
        started_at: span.started_at,
        ended_at: span.ended_at,
        error_classification: span.error_classification,
        // Identifiers only. What a client does with them is ask the service that owns the evidence,
        // which is the whole reason the trace DTO does not carry any.
        links: span.links.into_iter().map(link_to_dto).collect(),
        attributes: attributes_to_dto(span.attributes),
        waterfall: dto::SpanWaterfallDto {
            depth: derived.depth,
            start_offset_ms: derived.start_offset_ms,
            completed_duration_ms: derived.completed_duration_ms,
            attempt: derived.attempt,
            delegated: derived.delegated,
            critical_path: derived.critical_path,
        },
    }
}

/// One link, as identifiers and a relationship name.
///
/// These were previously dropped on the way out, so a client could see that a span existed but not
/// what it was related to. Carrying them costs nothing a redaction pass has to worry about — a run
/// id and a relationship word are not content — and it is what makes the linked-evidence query in
/// 9.10 addressable rather than a guess from the current scope.
fn link_to_dto(link: ExecutionLink) -> dto::ExecutionLinkDto {
    dto::ExecutionLinkDto {
        run_id: link.run_id.as_str().to_string(),
        trace_id: link.trace_id.as_str().to_string(),
        span_id: link.span_id.map(|value| value.as_str().to_string()),
        relationship: link.relationship,
    }
}

fn event_to_dto(event: ExecutionEvent) -> dto::ExecutionEventDto {
    dto::ExecutionEventDto {
        sequence: event.sequence,
        span_id: event.span_id.as_str().to_string(),
        name: event.name,
        timestamp: event.timestamp,
        attributes: attributes_to_dto(event.attributes),
    }
}

fn attributes_to_dto(
    attributes: SafeAttributes,
) -> std::collections::BTreeMap<String, dto::SafeAttributeDto> {
    attributes
        .entries()
        .iter()
        .map(|(key, value)| {
            let value = match value {
                SafeAttributeValue::Boolean(value) => dto::SafeAttributeDto::Boolean(*value),
                SafeAttributeValue::Integer(value) => dto::SafeAttributeDto::Integer(*value),
                SafeAttributeValue::Float(value) => dto::SafeAttributeDto::Float(*value),
                SafeAttributeValue::String(value) => dto::SafeAttributeDto::String(value.clone()),
            };
            (key.clone(), value)
        })
        .collect()
}

fn source_to_dto(source: ExecutionSource) -> (dto::ExecutionSourceDto, Option<String>) {
    match source {
        ExecutionSource::Desktop => (dto::ExecutionSourceDto::Desktop, None),
        ExecutionSource::InstantMessage { connector_id } => {
            (dto::ExecutionSourceDto::InstantMessage, Some(connector_id))
        }
        ExecutionSource::Scheduled { task_id } => {
            (dto::ExecutionSourceDto::Scheduled, Some(task_id))
        }
    }
}

fn status_to_dto(status: ExecutionStatus) -> dto::ExecutionStatusDto {
    match status {
        ExecutionStatus::Accepted => dto::ExecutionStatusDto::Accepted,
        ExecutionStatus::Running => dto::ExecutionStatusDto::Running,
        ExecutionStatus::Succeeded => dto::ExecutionStatusDto::Succeeded,
        ExecutionStatus::Failed => dto::ExecutionStatusDto::Failed,
        ExecutionStatus::Cancelled => dto::ExecutionStatusDto::Cancelled,
        ExecutionStatus::Incomplete => dto::ExecutionStatusDto::Incomplete,
    }
}

fn fidelity_to_dto(fidelity: ExecutionFidelity) -> dto::ExecutionFidelityDto {
    match fidelity {
        ExecutionFidelity::Native => dto::ExecutionFidelityDto::Native,
        ExecutionFidelity::Proxied => dto::ExecutionFidelityDto::Proxied,
        ExecutionFidelity::Inferred => dto::ExecutionFidelityDto::Inferred,
        ExecutionFidelity::Opaque => dto::ExecutionFidelityDto::Opaque,
    }
}

fn capture_to_dto(capture: CapturePolicy) -> dto::CapturePolicyDto {
    match capture {
        CapturePolicy::MetadataOnly => dto::CapturePolicyDto::MetadataOnly,
        CapturePolicy::RedactedContent => dto::CapturePolicyDto::RedactedContent,
    }
}

fn protocol_to_dto(protocol: OtlpProtocol) -> dto::OtlpProtocolDto {
    match protocol {
        OtlpProtocol::HttpProtobuf => dto::OtlpProtocolDto::HttpProtobuf,
    }
}

fn duration_ms(started_at: &str, ended_at: Option<&str>) -> Option<u64> {
    let started_at = chrono::DateTime::parse_from_rfc3339(started_at).ok()?;
    let ended_at = chrono::DateTime::parse_from_rfc3339(ended_at?).ok()?;
    u64::try_from((ended_at - started_at).num_milliseconds()).ok()
}

pub(super) fn adapter_error(error: ExecutionTelemetryError) -> dto::ObservabilityCommandErrorDto {
    if let ExecutionTelemetryError::InvalidSettings { field, message } = error {
        return invalid_settings(Some(field.to_string()), message.to_string());
    }
    let code = match error {
        ExecutionTelemetryError::Storage(_) => dto::ObservabilityErrorCodeDto::StorageUnavailable,
        ExecutionTelemetryError::Unavailable(_) => {
            dto::ObservabilityErrorCodeDto::ExporterUnavailable
        }
        ExecutionTelemetryError::InvalidSettings { .. } => unreachable!(),
    };
    dto::ObservabilityCommandErrorDto {
        code,
        message: "Execution observability is temporarily unavailable".to_string(),
        field: None,
    }
}

pub(super) fn invalid_settings(
    field: Option<String>,
    message: String,
) -> dto::ObservabilityCommandErrorDto {
    dto::ObservabilityCommandErrorDto {
        code: dto::ObservabilityErrorCodeDto::InvalidSettings,
        message,
        field,
    }
}

pub(super) fn invalid_page(message: String) -> dto::ObservabilityCommandErrorDto {
    dto::ObservabilityCommandErrorDto {
        code: dto::ObservabilityErrorCodeDto::InvalidPageToken,
        message,
        field: None,
    }
}

pub(super) fn run_not_found() -> dto::ObservabilityCommandErrorDto {
    dto::ObservabilityCommandErrorDto {
        code: dto::ObservabilityErrorCodeDto::RunNotFound,
        message: "Execution run was not found".to_string(),
        field: None,
    }
}
