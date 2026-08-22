pub(crate) use super::application::{
    ExecutionIdentityPort, ExecutionSettingsPort, ExecutionTelemetryError, ExecutionTelemetryPort,
};
use super::application::{ExecutionObservabilityRepositoryPort, ObservabilityCredentialPort};
pub(crate) use super::domain::{
    CapturePolicy, ExecutionContext, ExecutionEvent, ExecutionFidelity, ExecutionLink,
    ExecutionObservationCapability, ExecutionRun, ExecutionRunId, ExecutionSource, ExecutionSpan,
    ExecutionStatus, ExecutionTimeline, McpTransport, ObservabilitySettings, OtlpProtocol, Page,
    PageRequest, SafeAttributeValue, SafeAttributes, SpanId, TraceId,
};
use std::sync::Arc;

/// The published evidence surface.
///
/// Only the names below cross the context boundary. The SQLite rows, the repository, the
/// projection SQL, the cursor's internals, the migration schema, and the replay/retention
/// statements stay private: a consumer able to reach them could issue a query whose coverage
/// nobody vouches for, and a page whose completeness nobody can attest to is exactly what this
/// capability exists to eliminate.
///
/// The recorder is here for the in-process producer adapters Task Group 4 will add. It is
/// deliberately not reachable from a Tauri command — evidence is written by the runtime observing
/// its own work, never by a client asserting what happened.
pub(crate) mod evidence {
    pub(crate) use crate::contexts::execution_observability::application::evidence::models::{
        EvidenceCorrelationCounts, EvidenceQueryScope, EvidenceRecordPage,
        EvidenceSubscriptionBootstrap, ExecutionRecordDetailFields, ExecutionRecordDetailQuery,
        ExecutionRecordDetailView, ExecutionRecordFilters, ExecutionRecordKind,
        ExecutionRecordProjection, ExecutionRecordQuery, WorkspaceEvidenceSummary,
        WorkspaceEvidenceSummaryQuery, DEFAULT_EVIDENCE_PAGE_SIZE, MAX_EVIDENCE_PAGE_SIZE,
    };
    pub(crate) use crate::contexts::execution_observability::application::evidence::ports::EvidenceApplicationError;
    pub(crate) use crate::contexts::execution_observability::application::evidence::service::{
        ProjectionRepair, RecordEvidenceInput,
    };
    /// The vocabulary a producer adapter needs to build one `RecordEvidenceInput`, and no more.
    ///
    /// These are the value types the payload enum is made of, so an adapter can construct a
    /// correlation and a safe payload without ever naming an event, a row, or the repository.
    pub(crate) use crate::contexts::execution_observability::domain::{
        fidelity_token, parse_fidelity_token, parse_status_token, status_token, CommandRuntimeKind,
        EvidenceAgentId, EvidenceCorrelation, EvidenceOperationId, EvidenceOutcome, EvidenceSeatId,
        EvidenceSessionId, EvidenceSourceContext, ExecutionFidelity, ExecutionStatus,
        QueryCoverage, RedactionReceipt, SafeEvidencePayload, SafeReasonCode, SourceEventId,
        UsageQuality,
    };
    pub(crate) use crate::contexts::execution_observability::ExecutionEvidenceApi;
}

#[derive(Clone)]
pub(crate) struct ExecutionObservabilityApi {
    repository: Arc<dyn ExecutionObservabilityRepositoryPort>,
    credentials: Arc<dyn ObservabilityCredentialPort>,
}

impl ExecutionObservabilityApi {
    pub(crate) fn new(
        repository: Arc<dyn ExecutionObservabilityRepositoryPort>,
        credentials: Arc<dyn ObservabilityCredentialPort>,
    ) -> Self {
        Self {
            repository,
            credentials,
        }
    }

    pub(crate) fn settings(&self) -> Result<ObservabilitySettings, ExecutionTelemetryError> {
        let mut settings = self.repository.load_settings()?;
        settings.otlp_auth_configured = self.credentials.has_otlp_auth()?;
        Ok(settings)
    }

    pub(crate) fn update_settings(
        &self,
        settings: &ObservabilitySettings,
        auth_token: Option<&str>,
        updated_at: &str,
    ) -> Result<ObservabilitySettings, ExecutionTelemetryError> {
        settings.validate().map_err(|error| match error {
            super::domain::ExecutionDomainError::InvalidSetting { field, message } => {
                ExecutionTelemetryError::InvalidSettings { field, message }
            }
            _ => ExecutionTelemetryError::InvalidSettings {
                field: "settings",
                message: "invalid observability settings",
            },
        })?;
        let previous_auth = self.credentials.load_otlp_auth()?;
        let mut updated = settings.clone();
        updated.otlp_auth_configured = match auth_token {
            Some(secret) if secret.trim().is_empty() => {
                self.credentials.delete_otlp_auth()?;
                false
            }
            Some(secret) => {
                self.credentials.set_otlp_auth(secret)?;
                true
            }
            None => previous_auth.is_some(),
        };
        if let Err(error) = self.repository.update_settings(&updated, updated_at) {
            if auth_token.is_some() {
                match previous_auth.as_deref() {
                    Some(secret) => {
                        let _ = self.credentials.set_otlp_auth(secret);
                    }
                    None => {
                        let _ = self.credentials.delete_otlp_auth();
                    }
                }
            }
            return Err(error);
        }
        self.settings()
    }

    pub(crate) fn observation_capabilities(&self) -> Vec<ExecutionObservationCapability> {
        [
            ("claude-code", true),
            ("codex-cli", true),
            ("gemini-cli", false),
            ("opencode", false),
        ]
        .into_iter()
        .flat_map(|(agent_id, invocation_config_supported)| {
            [McpTransport::Stdio, McpTransport::Http].map(move |transport| {
                let relay_supported = invocation_config_supported;
                ExecutionObservationCapability {
                    agent_id: agent_id.to_string(),
                    transport,
                    tool_fidelity: ExecutionFidelity::Inferred,
                    mcp_fidelity: if relay_supported {
                        ExecutionFidelity::Proxied
                    } else {
                        ExecutionFidelity::Opaque
                    },
                    relay_supported,
                    detail: if invocation_config_supported {
                        "Invocation-scoped managed relay is available when explicitly enabled"
                            .to_string()
                    } else {
                        "No safe invocation-scoped MCP configuration is verified".to_string()
                    },
                }
            })
        })
        .collect()
    }

    pub(crate) fn list_runs(
        &self,
        request: &PageRequest,
        session_id: Option<&str>,
    ) -> Result<Page<ExecutionRun>, ExecutionTelemetryError> {
        self.repository.list_runs(request, session_id)
    }

    pub(crate) fn timeline(
        &self,
        run_id: &ExecutionRunId,
    ) -> Result<Option<ExecutionTimeline>, ExecutionTelemetryError> {
        self.repository.timeline(run_id)
    }
}

#[cfg(test)]
pub(crate) use super::application::test_adapter::{
    CapturedTelemetryRecord, CapturingExecutionTelemetry,
};
#[cfg(test)]
pub(crate) use super::infrastructure::RandomExecutionIdentity;

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
