use super::{CapturePolicy, ExecutionDomainError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OtlpProtocol {
    HttpProtobuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum McpTransport {
    Stdio,
    Http,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionObservationCapability {
    pub(crate) agent_id: String,
    pub(crate) transport: McpTransport,
    pub(crate) tool_fidelity: super::ExecutionFidelity,
    pub(crate) mcp_fidelity: super::ExecutionFidelity,
    pub(crate) relay_supported: bool,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ObservabilitySettings {
    pub(crate) local_timeline_enabled: bool,
    pub(crate) otlp_enabled: bool,
    pub(crate) otlp_endpoint: Option<String>,
    pub(crate) otlp_protocol: OtlpProtocol,
    pub(crate) sampling_ratio: f64,
    pub(crate) retention_days: u16,
    pub(crate) capture_policy: CapturePolicy,
    pub(crate) mcp_relay_enabled: bool,
    pub(crate) otlp_auth_configured: bool,
}

impl Default for ObservabilitySettings {
    fn default() -> Self {
        Self {
            local_timeline_enabled: true,
            otlp_enabled: false,
            otlp_endpoint: None,
            otlp_protocol: OtlpProtocol::HttpProtobuf,
            sampling_ratio: 1.0,
            retention_days: 30,
            capture_policy: CapturePolicy::MetadataOnly,
            mcp_relay_enabled: false,
            otlp_auth_configured: false,
        }
    }
}

impl ObservabilitySettings {
    pub(crate) fn validate(&self) -> Result<(), ExecutionDomainError> {
        if !self.sampling_ratio.is_finite() || !(0.0..=1.0).contains(&self.sampling_ratio) {
            return Err(invalid("sampling_ratio", "must be between 0 and 1"));
        }
        if !(1..=90).contains(&self.retention_days) {
            return Err(invalid("retention_days", "must be between 1 and 90"));
        }
        match self.otlp_endpoint.as_deref() {
            Some(value) => validate_endpoint(value)?,
            None if self.otlp_enabled => {
                return Err(invalid(
                    "otlp_endpoint",
                    "is required when OTLP export is enabled",
                ));
            }
            None => {}
        }
        Ok(())
    }
}

fn validate_endpoint(value: &str) -> Result<(), ExecutionDomainError> {
    let endpoint = url::Url::parse(value)
        .map_err(|_| invalid("otlp_endpoint", "must be a valid HTTP(S) URL"))?;
    if !matches!(endpoint.scheme(), "http" | "https")
        || endpoint.host_str().is_none()
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(invalid(
            "otlp_endpoint",
            "must not contain credentials or fragments",
        ));
    }
    Ok(())
}

fn invalid(field: &'static str, message: &'static str) -> ExecutionDomainError {
    ExecutionDomainError::InvalidSetting { field, message }
}
