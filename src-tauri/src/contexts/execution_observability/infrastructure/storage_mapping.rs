use crate::contexts::execution_observability::application::ExecutionTelemetryError;
use crate::contexts::execution_observability::domain::{
    CapturePolicy, ExecutionFidelity, ExecutionSource, ExecutionStatus, SafeAttributeValue,
    SafeAttributes,
};
use serde_json::{Map, Number, Value};

pub(super) fn status_value(value: ExecutionStatus) -> &'static str {
    match value {
        ExecutionStatus::Accepted => "accepted",
        ExecutionStatus::Running => "running",
        ExecutionStatus::Succeeded => "succeeded",
        ExecutionStatus::Failed => "failed",
        ExecutionStatus::Cancelled => "cancelled",
        ExecutionStatus::Incomplete => "incomplete",
    }
}

pub(super) fn parse_status(value: &str) -> Result<ExecutionStatus, ExecutionTelemetryError> {
    match value {
        "accepted" => Ok(ExecutionStatus::Accepted),
        "running" => Ok(ExecutionStatus::Running),
        "succeeded" => Ok(ExecutionStatus::Succeeded),
        "failed" => Ok(ExecutionStatus::Failed),
        "cancelled" => Ok(ExecutionStatus::Cancelled),
        "incomplete" => Ok(ExecutionStatus::Incomplete),
        _ => Err(storage_error(format!("invalid execution status: {value}"))),
    }
}

pub(super) fn fidelity_value(value: ExecutionFidelity) -> &'static str {
    match value {
        ExecutionFidelity::Native => "native",
        ExecutionFidelity::Proxied => "proxied",
        ExecutionFidelity::Inferred => "inferred",
        ExecutionFidelity::Opaque => "opaque",
    }
}

pub(super) fn parse_fidelity(value: &str) -> Result<ExecutionFidelity, ExecutionTelemetryError> {
    match value {
        "native" => Ok(ExecutionFidelity::Native),
        "proxied" => Ok(ExecutionFidelity::Proxied),
        "inferred" => Ok(ExecutionFidelity::Inferred),
        "opaque" => Ok(ExecutionFidelity::Opaque),
        _ => Err(storage_error(format!(
            "invalid execution fidelity: {value}"
        ))),
    }
}

pub(super) fn capture_value(value: CapturePolicy) -> &'static str {
    match value {
        CapturePolicy::MetadataOnly => "metadata_only",
        CapturePolicy::RedactedContent => "redacted_content",
    }
}

pub(super) fn parse_capture(value: &str) -> Result<CapturePolicy, ExecutionTelemetryError> {
    match value {
        "metadata_only" => Ok(CapturePolicy::MetadataOnly),
        "redacted_content" => Ok(CapturePolicy::RedactedContent),
        _ => Err(storage_error(format!("invalid capture policy: {value}"))),
    }
}

pub(super) fn source_parts(source: &ExecutionSource) -> (&'static str, Option<&str>) {
    match source {
        ExecutionSource::Desktop => ("desktop", None),
        ExecutionSource::InstantMessage { connector_id } => {
            ("instant_message", Some(connector_id.as_str()))
        }
        ExecutionSource::Scheduled { task_id } => ("scheduled", Some(task_id.as_str())),
    }
}

pub(super) fn parse_source(
    source: &str,
    source_id: Option<String>,
) -> Result<ExecutionSource, ExecutionTelemetryError> {
    match source {
        "desktop" => Ok(ExecutionSource::Desktop),
        "instant_message" => Ok(ExecutionSource::InstantMessage {
            connector_id: source_id.unwrap_or_default(),
        }),
        "scheduled" => Ok(ExecutionSource::Scheduled {
            task_id: source_id.unwrap_or_default(),
        }),
        _ => Err(storage_error(format!("invalid execution source: {source}"))),
    }
}

pub(super) fn attributes_json(
    attributes: &SafeAttributes,
) -> Result<String, ExecutionTelemetryError> {
    let values = attributes
        .entries()
        .iter()
        .map(|(key, value)| (key.clone(), attribute_json(value)))
        .collect::<Map<_, _>>();
    serde_json::to_string(&values).map_err(|error| storage_error(error.to_string()))
}

pub(super) fn parse_attributes(value: &str) -> Result<SafeAttributes, ExecutionTelemetryError> {
    let values = serde_json::from_str::<Map<String, Value>>(value)
        .map_err(|error| storage_error(error.to_string()))?;
    let entries = values
        .into_iter()
        .map(|(key, value)| parse_attribute(value).map(|value| (key, value)))
        .collect::<Result<Vec<_>, _>>()?;
    SafeAttributes::try_from_entries(entries).map_err(|error| storage_error(error.to_string()))
}

fn attribute_json(value: &SafeAttributeValue) -> Value {
    match value {
        SafeAttributeValue::Boolean(value) => Value::Bool(*value),
        SafeAttributeValue::Integer(value) => Value::Number((*value).into()),
        SafeAttributeValue::Float(value) => Number::from_f64(*value)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        SafeAttributeValue::String(value) => Value::String(value.clone()),
    }
}

fn parse_attribute(value: Value) -> Result<SafeAttributeValue, ExecutionTelemetryError> {
    match value {
        Value::Bool(value) => Ok(SafeAttributeValue::Boolean(value)),
        Value::Number(value) if value.is_i64() => value
            .as_i64()
            .map(SafeAttributeValue::Integer)
            .ok_or_else(|| storage_error("invalid integer attribute")),
        Value::Number(value) => value
            .as_f64()
            .map(SafeAttributeValue::Float)
            .ok_or_else(|| storage_error("invalid float attribute")),
        Value::String(value) => SafeAttributeValue::bounded_string(value)
            .map_err(|error| storage_error(error.to_string())),
        _ => Err(storage_error("unsupported safe attribute value")),
    }
}

pub(super) fn storage_error(error: impl Into<String>) -> ExecutionTelemetryError {
    ExecutionTelemetryError::Storage(error.into())
}
