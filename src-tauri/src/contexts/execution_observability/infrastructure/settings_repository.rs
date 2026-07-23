use super::storage_mapping::storage_error;
use super::SqliteExecutionTimelineRepository;
use crate::contexts::execution_observability::application::ExecutionTelemetryError;
use crate::contexts::execution_observability::domain::{
    CapturePolicy, ObservabilitySettings, OtlpProtocol,
};
use rusqlite::params;

impl crate::contexts::execution_observability::application::ExecutionSettingsPort
    for SqliteExecutionTimelineRepository
{
    fn load_settings(&self) -> Result<ObservabilitySettings, ExecutionTelemetryError> {
        SqliteExecutionTimelineRepository::load_settings(self)
    }
}

impl SqliteExecutionTimelineRepository {
    pub(crate) fn load_settings(&self) -> Result<ObservabilitySettings, ExecutionTelemetryError> {
        self.connection()?
            .query_row(
                r#"SELECT local_timeline_enabled, otlp_enabled, otlp_endpoint,
                          otlp_protocol, sampling_ratio, retention_days, capture_policy,
                          mcp_relay_enabled, otlp_auth_ref IS NOT NULL
                   FROM execution_observability_settings WHERE singleton_id = 1"#,
                [],
                |row| {
                    Ok((
                        row.get::<_, bool>(0)?,
                        row.get::<_, bool>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, f64>(4)?,
                        row.get::<_, u16>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, bool>(7)?,
                        row.get::<_, bool>(8)?,
                    ))
                },
            )
            .map_err(|error| storage_error(error.to_string()))
            .and_then(settings_from_row)
    }

    pub(crate) fn update_settings(
        &self,
        settings: &ObservabilitySettings,
        updated_at: &str,
    ) -> Result<(), ExecutionTelemetryError> {
        settings
            .validate()
            .map_err(|error| storage_error(error.to_string()))?;
        if updated_at.trim().is_empty() {
            return Err(storage_error("settings update timestamp is required"));
        }
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction()
            .map_err(|error| storage_error(error.to_string()))?;
        transaction
            .execute(
                r#"UPDATE execution_observability_settings SET
                    local_timeline_enabled = ?1, otlp_enabled = ?2, otlp_endpoint = ?3,
                    otlp_protocol = ?4, sampling_ratio = ?5, retention_days = ?6,
                    capture_policy = ?7, mcp_relay_enabled = ?8,
                    otlp_auth_ref = ?9, updated_at = ?10
                   WHERE singleton_id = 1"#,
                params![
                    settings.local_timeline_enabled,
                    settings.otlp_enabled,
                    settings.otlp_endpoint,
                    protocol_value(settings.otlp_protocol),
                    settings.sampling_ratio,
                    settings.retention_days,
                    capture_value(settings.capture_policy),
                    settings.mcp_relay_enabled,
                    settings
                        .otlp_auth_configured
                        .then_some("os-keyring:execution-observability-otlp-auth"),
                    updated_at,
                ],
            )
            .map_err(|error| storage_error(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| storage_error(error.to_string()))
    }
}

type SettingsRow = (
    bool,
    bool,
    Option<String>,
    String,
    f64,
    u16,
    String,
    bool,
    bool,
);

fn settings_from_row(row: SettingsRow) -> Result<ObservabilitySettings, ExecutionTelemetryError> {
    let settings = ObservabilitySettings {
        local_timeline_enabled: row.0,
        otlp_enabled: row.1,
        otlp_endpoint: row.2,
        otlp_protocol: match row.3.as_str() {
            "http_protobuf" => OtlpProtocol::HttpProtobuf,
            value => return Err(storage_error(format!("unsupported OTLP protocol: {value}"))),
        },
        sampling_ratio: row.4,
        retention_days: row.5,
        capture_policy: match row.6.as_str() {
            "metadata_only" => CapturePolicy::MetadataOnly,
            "redacted_content" => CapturePolicy::RedactedContent,
            value => {
                return Err(storage_error(format!(
                    "unsupported capture policy: {value}"
                )));
            }
        },
        mcp_relay_enabled: row.7,
        otlp_auth_configured: row.8,
    };
    settings
        .validate()
        .map_err(|error| storage_error(error.to_string()))?;
    Ok(settings)
}

fn protocol_value(value: OtlpProtocol) -> &'static str {
    match value {
        OtlpProtocol::HttpProtobuf => "http_protobuf",
    }
}

fn capture_value(value: CapturePolicy) -> &'static str {
    match value {
        CapturePolicy::MetadataOnly => "metadata_only",
        CapturePolicy::RedactedContent => "redacted_content",
    }
}
