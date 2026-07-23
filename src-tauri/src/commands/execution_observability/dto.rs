use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapturePolicyDto {
    MetadataOnly,
    RedactedContent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OtlpProtocolDto {
    HttpProtobuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ObservabilitySettingsDto {
    pub(crate) local_timeline_enabled: bool,
    pub(crate) otlp_enabled: bool,
    pub(crate) otlp_endpoint: Option<String>,
    pub(crate) otlp_protocol: OtlpProtocolDto,
    pub(crate) sampling_ratio: f64,
    pub(crate) retention_days: u16,
    pub(crate) capture_policy: CapturePolicyDto,
    pub(crate) mcp_relay_enabled: bool,
    pub(crate) otlp_auth_configured: bool,
    #[serde(default, skip_serializing)]
    pub(crate) otlp_auth_token: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExecutionStatusDto {
    Accepted,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Incomplete,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExecutionFidelityDto {
    Native,
    Proxied,
    Inferred,
    Opaque,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExecutionSourceDto {
    Desktop,
    InstantMessage,
    Scheduled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub(crate) enum SafeAttributeDto {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionRunSummaryDto {
    pub(crate) run_id: String,
    pub(crate) trace_id: String,
    pub(crate) root_span_id: String,
    pub(crate) source: ExecutionSourceDto,
    pub(crate) source_id: Option<String>,
    pub(crate) status: ExecutionStatusDto,
    pub(crate) started_at: String,
    pub(crate) ended_at: Option<String>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) session_id: Option<String>,
    pub(crate) operation_id: Option<String>,
    pub(crate) agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionSpanSummaryDto {
    pub(crate) span_id: String,
    pub(crate) parent_span_id: Option<String>,
    pub(crate) name: String,
    pub(crate) status: ExecutionStatusDto,
    pub(crate) fidelity: ExecutionFidelityDto,
    pub(crate) started_at: String,
    pub(crate) ended_at: Option<String>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) error_classification: Option<String>,
    pub(crate) attributes: BTreeMap<String, SafeAttributeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionEventDto {
    pub(crate) sequence: u64,
    pub(crate) span_id: String,
    pub(crate) name: String,
    pub(crate) timestamp: String,
    pub(crate) attributes: BTreeMap<String, SafeAttributeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionTimelineDto {
    pub(crate) run: ExecutionRunSummaryDto,
    pub(crate) spans: Vec<ExecutionSpanSummaryDto>,
    pub(crate) events: Vec<ExecutionEventDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PageRequestDto {
    pub(crate) limit: u16,
    pub(crate) page_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionRunPageDto {
    pub(crate) items: Vec<ExecutionRunSummaryDto>,
    pub(crate) next_page_token: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum McpTransportDto {
    Stdio,
    Http,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExecutionObservationCapabilityDto {
    pub(crate) agent_id: String,
    pub(crate) transport: McpTransportDto,
    pub(crate) tool_fidelity: ExecutionFidelityDto,
    pub(crate) mcp_fidelity: ExecutionFidelityDto,
    pub(crate) relay_supported: bool,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ObservabilityErrorCodeDto {
    InvalidSettings,
    InvalidPageToken,
    RunNotFound,
    StorageUnavailable,
    ExporterUnavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ObservabilityCommandErrorDto {
    pub(crate) code: ObservabilityErrorCodeDto,
    pub(crate) message: String,
    pub(crate) field: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_all_fidelity_values_in_snake_case() {
        let values = [
            (ExecutionFidelityDto::Native, "native"),
            (ExecutionFidelityDto::Proxied, "proxied"),
            (ExecutionFidelityDto::Inferred, "inferred"),
            (ExecutionFidelityDto::Opaque, "opaque"),
        ];
        for (value, expected) in values {
            assert_eq!(serde_json::to_value(value).unwrap(), expected);
        }
    }

    #[test]
    fn keeps_pagination_and_attributes_typed() {
        let page = PageRequestDto {
            limit: 50,
            page_token: Some("cursor-1".to_string()),
        };
        let value = serde_json::to_value(page).unwrap();
        assert_eq!(value["limit"], 50);
        assert_eq!(value["pageToken"], "cursor-1");

        let attributes = BTreeMap::from([
            (
                "agent.id".to_string(),
                SafeAttributeDto::String("codex-cli".to_string()),
            ),
            ("retry.count".to_string(), SafeAttributeDto::Integer(1)),
        ]);
        let value = serde_json::to_value(attributes).unwrap();
        assert_eq!(value["agent.id"], "codex-cli");
        assert_eq!(value["retry.count"], 1);
    }

    #[test]
    fn serializes_capability_without_claiming_native_mcp_observation() {
        let capability = ExecutionObservationCapabilityDto {
            agent_id: "gemini-cli".to_string(),
            transport: McpTransportDto::Stdio,
            tool_fidelity: ExecutionFidelityDto::Inferred,
            mcp_fidelity: ExecutionFidelityDto::Opaque,
            relay_supported: false,
            detail: "No invocation-scoped MCP configuration".to_string(),
        };
        let value = serde_json::to_value(capability).unwrap();
        assert_eq!(value["transport"], "stdio");
        assert_eq!(value["mcpFidelity"], "opaque");
        assert_eq!(value["relaySupported"], false);
    }

    #[test]
    fn observability_settings_never_serialize_write_only_credentials() {
        let settings = ObservabilitySettingsDto {
            local_timeline_enabled: true,
            otlp_enabled: true,
            otlp_endpoint: Some("https://collector.example.com".to_string()),
            otlp_protocol: OtlpProtocolDto::HttpProtobuf,
            sampling_ratio: 1.0,
            retention_days: 30,
            capture_policy: CapturePolicyDto::MetadataOnly,
            mcp_relay_enabled: false,
            otlp_auth_configured: true,
            otlp_auth_token: Some("private-token".to_string()),
        };
        let json = serde_json::to_string(&settings).expect("serialize settings");
        assert!(!json.contains("private-token"));
        assert!(!json.contains("otlpAuthToken"));
        assert!(json.contains("\"otlpAuthConfigured\":true"));
    }
}
