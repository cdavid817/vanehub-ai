use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::contexts::code_intelligence::api::{
    CodeIntelligenceApiError, DiscoveredServer, DiscoveryAvailability, DiscoveryReason,
    DocumentSyncMode, IsolatedServerTestResult, LanguageConfiguration, LanguageFamily,
    LspConfiguration, NegotiatedCapabilities, PositionEncoding, ProcessState, ServerKind,
    ServerStatus, ServerStatusReason, ServerTestPhase, ServerTestPhaseStatus, ServerTestReason,
    WorkspaceTrust,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LspLanguageIdDto {
    Rust,
    #[serde(rename = "typescript_javascript")]
    TypeScriptJavaScript,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LspServerKindDto {
    RustAnalyzer,
    #[serde(rename = "typescript_language_server")]
    TypeScriptLanguageServer,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LspProcessStateDto {
    Absent,
    Starting,
    Initializing,
    Ready,
    Stopping,
    Backoff,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LspDiscoveryAvailabilityDto {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LspServerTestPhaseDto {
    Discovery,
    Spawn,
    Initialize,
    Cleanup,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LspServerTestPhaseStatusDto {
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LspPositionEncodingDto {
    Utf8,
    Utf16,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LspDocumentSyncDto {
    None,
    Full,
    Incremental,
}

/// Closed reason vocabulary prevents raw process, protocol, path, or storage errors crossing the
/// command boundary.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LspSafeReasonCodeDto {
    ExecutableNotFound,
    OverrideMissing,
    OverrideNotExecutable,
    ExecutableUnavailable,
    MinimalProjectFailed,
    SpawnFailed,
    InitializeFailed,
    InitializeTimedOut,
    ForcedTermination,
    CleanupFailed,
    InvalidDeadline,
    RestartExhausted,
    ProtocolLimit,
    RequestTimeout,
    Cancelled,
    Untrusted,
    UnsupportedMethod,
    InvalidConfiguration,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspLanguageConfigurationDto {
    pub(crate) language: LspLanguageIdDto,
    pub(crate) enabled: bool,
    pub(crate) executable_override: Option<String>,
    pub(crate) initialization_options: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspConfigurationDto {
    pub(crate) enabled: bool,
    pub(crate) languages: Vec<LspLanguageConfigurationDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspWorkspaceTrustDto {
    pub(crate) canonical_root: String,
    pub(crate) trusted: bool,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspWorkspaceTrustUpdateDto {
    pub(crate) canonical_root: String,
    pub(crate) trusted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspServerDiscoveryDto {
    pub(crate) language: LspLanguageIdDto,
    pub(crate) server: LspServerKindDto,
    pub(crate) availability: LspDiscoveryAvailabilityDto,
    pub(crate) executable_path: Option<String>,
    pub(crate) arguments: Vec<String>,
    pub(crate) reason_code: Option<LspSafeReasonCodeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspServerTestInputDto {
    pub(crate) language: LspLanguageIdDto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspServerTestPhaseResultDto {
    pub(crate) phase: LspServerTestPhaseDto,
    pub(crate) status: LspServerTestPhaseStatusDto,
    pub(crate) reason_code: Option<LspSafeReasonCodeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspNegotiatedCapabilitiesDto {
    pub(crate) position_encoding: LspPositionEncodingDto,
    pub(crate) document_sync: LspDocumentSyncDto,
    pub(crate) definition: bool,
    pub(crate) references: bool,
    pub(crate) hover: bool,
    pub(crate) diagnostics: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspServerTestResultDto {
    pub(crate) server: LspServerKindDto,
    pub(crate) phases: Vec<LspServerTestPhaseResultDto>,
    pub(crate) negotiated_capabilities: Option<LspNegotiatedCapabilitiesDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspServerStatusDto {
    pub(crate) language: LspLanguageIdDto,
    pub(crate) server: LspServerKindDto,
    pub(crate) relative_project_root: String,
    pub(crate) state: LspProcessStateDto,
    pub(crate) restart_count: u32,
    pub(crate) last_response_at: Option<String>,
    pub(crate) diagnostic_count: usize,
    pub(crate) reason_code: Option<LspSafeReasonCodeDto>,
    pub(crate) negotiated_capabilities: Option<LspNegotiatedCapabilitiesDto>,
}

impl From<LanguageFamily> for LspLanguageIdDto {
    fn from(value: LanguageFamily) -> Self {
        match value {
            LanguageFamily::Rust => Self::Rust,
            LanguageFamily::TypeScriptJavaScript => Self::TypeScriptJavaScript,
        }
    }
}

impl From<LspLanguageIdDto> for LanguageFamily {
    fn from(value: LspLanguageIdDto) -> Self {
        match value {
            LspLanguageIdDto::Rust => Self::Rust,
            LspLanguageIdDto::TypeScriptJavaScript => Self::TypeScriptJavaScript,
        }
    }
}

impl From<ServerKind> for LspServerKindDto {
    fn from(value: ServerKind) -> Self {
        match value {
            ServerKind::RustAnalyzer => Self::RustAnalyzer,
            ServerKind::TypeScriptLanguageServer => Self::TypeScriptLanguageServer,
        }
    }
}

impl TryFrom<LspConfigurationDto> for LspConfiguration {
    type Error = CodeIntelligenceApiError;

    fn try_from(value: LspConfigurationDto) -> Result<Self, Self::Error> {
        let mut languages = BTreeMap::new();
        for entry in value.languages {
            let language = LanguageFamily::from(entry.language);
            let configuration = LanguageConfiguration {
                enabled: entry.enabled,
                executable_override: entry.executable_override,
                initialization_options: entry.initialization_options,
            };
            if languages.insert(language, configuration).is_some() {
                return Err(CodeIntelligenceApiError::InvalidConfiguration);
            }
        }
        let configuration = Self {
            enabled: value.enabled,
            languages,
        };
        configuration
            .validate()
            .map_err(|_| CodeIntelligenceApiError::InvalidConfiguration)?;
        Ok(configuration)
    }
}

impl From<LspConfiguration> for LspConfigurationDto {
    fn from(value: LspConfiguration) -> Self {
        Self {
            enabled: value.enabled,
            languages: value
                .languages
                .into_iter()
                .map(|(language, configuration)| LspLanguageConfigurationDto {
                    language: language.into(),
                    enabled: configuration.enabled,
                    executable_override: configuration.executable_override,
                    initialization_options: configuration.initialization_options,
                })
                .collect(),
        }
    }
}

impl From<WorkspaceTrust> for LspWorkspaceTrustDto {
    fn from(value: WorkspaceTrust) -> Self {
        Self {
            canonical_root: value.canonical_root().to_string(),
            trusted: value.is_trusted(),
            revision: value.revision(),
        }
    }
}

impl From<DiscoveredServer> for LspServerDiscoveryDto {
    fn from(value: DiscoveredServer) -> Self {
        Self {
            language: value.language.into(),
            server: value.server.into(),
            availability: match value.availability {
                DiscoveryAvailability::Available => LspDiscoveryAvailabilityDto::Available,
                DiscoveryAvailability::Unavailable => LspDiscoveryAvailabilityDto::Unavailable,
            },
            executable_path: value.executable_path,
            arguments: value.arguments,
            reason_code: value.reason.map(discovery_reason),
        }
    }
}

impl LspServerTestResultDto {
    pub(crate) fn from_result(server: ServerKind, result: IsolatedServerTestResult) -> Self {
        Self {
            server: server.into(),
            phases: result
                .phases()
                .iter()
                .map(|phase| LspServerTestPhaseResultDto {
                    phase: test_phase(phase.phase),
                    status: test_phase_status(phase.status),
                    reason_code: phase.reason.map(test_reason),
                })
                .collect(),
            negotiated_capabilities: result
                .negotiated_capabilities()
                .map(LspNegotiatedCapabilitiesDto::from),
        }
    }
}

impl From<&NegotiatedCapabilities> for LspNegotiatedCapabilitiesDto {
    fn from(value: &NegotiatedCapabilities) -> Self {
        Self {
            position_encoding: match value.position_encoding {
                PositionEncoding::Utf8 => LspPositionEncodingDto::Utf8,
                PositionEncoding::Utf16 => LspPositionEncodingDto::Utf16,
            },
            document_sync: match value.document_sync {
                DocumentSyncMode::None => LspDocumentSyncDto::None,
                DocumentSyncMode::Full => LspDocumentSyncDto::Full,
                DocumentSyncMode::Incremental => LspDocumentSyncDto::Incremental,
            },
            definition: value.definition,
            references: value.references,
            hover: value.hover,
            diagnostics: value.diagnostics,
        }
    }
}

impl From<ServerStatus> for LspServerStatusDto {
    fn from(value: ServerStatus) -> Self {
        Self {
            language: value.language.into(),
            server: value.server.into(),
            relative_project_root: value.relative_project_root,
            state: process_state(value.state),
            restart_count: value.restart_count,
            last_response_at: value.last_response_at,
            diagnostic_count: value.diagnostic_count,
            reason_code: value.reason.map(status_reason),
            negotiated_capabilities: value
                .negotiated_capabilities
                .as_ref()
                .map(LspNegotiatedCapabilitiesDto::from),
        }
    }
}

fn discovery_reason(value: DiscoveryReason) -> LspSafeReasonCodeDto {
    match value {
        DiscoveryReason::ExecutableNotFound => LspSafeReasonCodeDto::ExecutableNotFound,
        DiscoveryReason::OverrideMissing => LspSafeReasonCodeDto::OverrideMissing,
        DiscoveryReason::OverrideNotExecutable => LspSafeReasonCodeDto::OverrideNotExecutable,
    }
}

fn test_phase(value: ServerTestPhase) -> LspServerTestPhaseDto {
    match value {
        ServerTestPhase::Discovery => LspServerTestPhaseDto::Discovery,
        ServerTestPhase::Spawn => LspServerTestPhaseDto::Spawn,
        ServerTestPhase::Initialize => LspServerTestPhaseDto::Initialize,
        ServerTestPhase::Cleanup => LspServerTestPhaseDto::Cleanup,
    }
}

fn test_phase_status(value: ServerTestPhaseStatus) -> LspServerTestPhaseStatusDto {
    match value {
        ServerTestPhaseStatus::Succeeded => LspServerTestPhaseStatusDto::Succeeded,
        ServerTestPhaseStatus::Failed => LspServerTestPhaseStatusDto::Failed,
        ServerTestPhaseStatus::Skipped => LspServerTestPhaseStatusDto::Skipped,
    }
}

fn test_reason(value: ServerTestReason) -> LspSafeReasonCodeDto {
    match value {
        ServerTestReason::ExecutableUnavailable => LspSafeReasonCodeDto::ExecutableUnavailable,
        ServerTestReason::MinimalProjectFailed => LspSafeReasonCodeDto::MinimalProjectFailed,
        ServerTestReason::SpawnFailed => LspSafeReasonCodeDto::SpawnFailed,
        ServerTestReason::InitializeFailed => LspSafeReasonCodeDto::InitializeFailed,
        ServerTestReason::InitializeTimedOut => LspSafeReasonCodeDto::InitializeTimedOut,
        ServerTestReason::ForcedTermination => LspSafeReasonCodeDto::ForcedTermination,
        ServerTestReason::CleanupFailed => LspSafeReasonCodeDto::CleanupFailed,
        ServerTestReason::InvalidDeadline => LspSafeReasonCodeDto::InvalidDeadline,
    }
}

fn process_state(value: ProcessState) -> LspProcessStateDto {
    match value {
        ProcessState::Absent => LspProcessStateDto::Absent,
        ProcessState::Starting => LspProcessStateDto::Starting,
        ProcessState::Initializing => LspProcessStateDto::Initializing,
        ProcessState::Ready => LspProcessStateDto::Ready,
        ProcessState::Stopping => LspProcessStateDto::Stopping,
        ProcessState::Backoff => LspProcessStateDto::Backoff,
        ProcessState::Failed => LspProcessStateDto::Failed,
    }
}

fn status_reason(value: ServerStatusReason) -> LspSafeReasonCodeDto {
    match value {
        ServerStatusReason::RestartExhausted => LspSafeReasonCodeDto::RestartExhausted,
        ServerStatusReason::ProtocolLimit => LspSafeReasonCodeDto::ProtocolLimit,
    }
}
