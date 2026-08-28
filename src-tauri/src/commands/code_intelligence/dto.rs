use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::contexts::code_intelligence::api::{
    resolve_language, CodeIntelligenceApiError, DiscoveredServer, DiscoveryAvailability,
    DiscoveryReason, DocumentSyncMode, IsolatedServerTestResult, Language, LanguageConfiguration,
    LspConfiguration, NegotiatedCapabilities, PositionEncoding, ProcessState, ServerStatus,
    ServerStatusReason, ServerTestPhase, ServerTestPhaseStatus, ServerTestReason, WorkspaceTrust,
    LANGUAGE_DEFINITIONS,
};

// Language and server ids cross the wire as plain strings. They were closed enums while the
// supported set was compiled in; now that the registry owns that set, an enum here would put it
// back in a second place that has to be edited in lockstep. The serialized values are unchanged.
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
    UnsupportedOnThisPlatform,
    PrerequisiteMissing,
    InstallDirectoryNotSet,
    LauncherNotFound,
    AmbiguousInstall,
    ExecutableUnavailable,
    InstallRefused,
    InstallFailed,
    // Distinct from `RequestTimeout`, which is about an LSP request. A 50 MB artifact against a
    // ten-minute budget needs about 85 KB/s sustained, so this is reachable on an ordinary slow
    // link -- and telling such a user that a "language-server request" timed out describes a
    // request that was never made.
    InstallTimedOut,
    ChecksumMismatch,
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

/// Names a language for an install or uninstall action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspServerInstallInputDto {
    pub(crate) language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspLanguageConfigurationDto {
    pub(crate) language: String,
    pub(crate) enabled: bool,
    pub(crate) executable_override: Option<String>,
    /// Absent means "use the registry default"; present but empty means the user chose none.
    ///
    /// `default` so a caller written before this field existed still saves successfully, rather
    /// than failing on a field whose absence already has a defined meaning.
    #[serde(default)]
    pub(crate) startup_arguments: Option<Vec<String>>,
    pub(crate) initialization_options: Value,
}

/// What the frontend needs to render one language's controls without compiling in a language list.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspLanguageDescriptorDto {
    pub(crate) language: String,
    pub(crate) server: String,
    pub(crate) supported_on_host: bool,
    pub(crate) default_startup_arguments: Vec<String>,
    /// What the override control means for this language: an executable file, or the server's
    /// install directory. Reported so the settings card learns it from the backend rather than by
    /// checking a language id -- a second interpreter-shaped language must need no frontend edit.
    pub(crate) override_target: LspOverrideTargetDto,
    /// The host runtime the user has to install themselves, for the languages that need one.
    pub(crate) prerequisite: Option<String>,
    /// Present when VaneHub can fetch this server. `None` means no install action is offered,
    /// which the card reads from here rather than from the language's identity.
    pub(crate) distribution: Option<LspDistributionDto>,
    /// Whether a managed install exists right now.
    pub(crate) installed: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspDistributionDto {
    /// Whether the download is checked against a published digest. Reported so the surface can
    /// say it, rather than presenting an unverified download as a verified one.
    pub(crate) verified: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LspOverrideTargetDto {
    ExecutableFile,
    InstallDirectory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspConfigurationDto {
    pub(crate) enabled: bool,
    pub(crate) languages: Vec<LspLanguageConfigurationDto>,
    /// Every language this build registers, whether or not it has saved configuration yet.
    ///
    /// Output only. The backend rebuilds it from the registry on every read, so requiring a caller
    /// to send it back would make them restate a fact they did not author — and would reject any
    /// caller that reaches the command directly instead of through the frontend adapter.
    #[serde(default, skip_deserializing)]
    pub(crate) descriptors: Vec<LspLanguageDescriptorDto>,
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
    pub(crate) language: String,
    pub(crate) server: String,
    pub(crate) availability: LspDiscoveryAvailabilityDto,
    pub(crate) executable_path: Option<String>,
    pub(crate) arguments: Vec<String>,
    pub(crate) reason_code: Option<LspSafeReasonCodeDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspServerTestInputDto {
    pub(crate) language: String,
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
    /// One entry per method this build implements, in a stable order. A consumer renders what it
    /// is given rather than holding its own copy of the method set.
    pub(crate) methods: Vec<LspNegotiatedMethodDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspNegotiatedMethodDto {
    pub(crate) method: String,
    pub(crate) supported: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspServerTestResultDto {
    pub(crate) server: String,
    pub(crate) phases: Vec<LspServerTestPhaseResultDto>,
    pub(crate) negotiated_capabilities: Option<LspNegotiatedCapabilitiesDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LspServerStatusDto {
    pub(crate) language: String,
    pub(crate) server: String,
    pub(crate) relative_project_root: String,
    pub(crate) state: LspProcessStateDto,
    pub(crate) restart_count: u32,
    pub(crate) last_response_at: Option<String>,
    pub(crate) diagnostic_count: usize,
    pub(crate) reason_code: Option<LspSafeReasonCodeDto>,
    pub(crate) negotiated_capabilities: Option<LspNegotiatedCapabilitiesDto>,
}

impl TryFrom<LspConfigurationDto> for LspConfiguration {
    type Error = CodeIntelligenceApiError;

    fn try_from(value: LspConfigurationDto) -> Result<Self, Self::Error> {
        let mut languages = BTreeMap::new();
        for entry in value.languages {
            // An id the registry does not know is refused rather than stored: persisting
            // configuration for a language nothing can serve only defers the failure.
            let language = resolve_language(&entry.language)
                .ok_or(CodeIntelligenceApiError::InvalidConfiguration)?;
            let configuration = LanguageConfiguration {
                enabled: entry.enabled,
                executable_override: entry.executable_override,
                startup_arguments: entry.startup_arguments,
                initialization_options: entry.initialization_options,
            };
            if languages
                .insert(language.language_id(), configuration)
                .is_some()
            {
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
                    language: language.as_str().to_owned(),
                    enabled: configuration.enabled,
                    executable_override: configuration.executable_override,
                    startup_arguments: configuration.startup_arguments,
                    initialization_options: configuration.initialization_options,
                })
                .collect(),
            descriptors: LANGUAGE_DEFINITIONS
                .iter()
                .map(|definition| LspLanguageDescriptorDto {
                    language: definition.id.to_owned(),
                    server: definition.server_id.to_owned(),
                    supported_on_host: definition.supports_host(),
                    default_startup_arguments: definition
                        .default_startup_arguments
                        .iter()
                        .map(|argument| (*argument).to_string())
                        .collect(),
                    override_target: match definition.launch.interpreter() {
                        Some(_) => LspOverrideTargetDto::InstallDirectory,
                        None => LspOverrideTargetDto::ExecutableFile,
                    },
                    prerequisite: definition
                        .launch
                        .interpreter()
                        .map(|launch| launch.prerequisite.to_owned()),
                    distribution: definition.distribution.as_ref().map(|distribution| {
                        LspDistributionDto {
                            verified: distribution.is_verified(),
                        }
                    }),
                    // Filled in by the command layer, which knows the profile directory. The
                    // conversion from a bare configuration cannot: it has no filesystem.
                    installed: false,
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
            language: value.language.id.to_owned(),
            server: value.language.server_id.to_owned(),
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
    pub(crate) fn from_result(language: Language, result: IsolatedServerTestResult) -> Self {
        Self {
            server: language.server_id.to_owned(),
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
            methods: value
                .methods
                .iter()
                .map(|entry| LspNegotiatedMethodDto {
                    method: entry.method.id().to_owned(),
                    supported: entry.supported,
                })
                .collect(),
        }
    }
}

impl From<ServerStatus> for LspServerStatusDto {
    fn from(value: ServerStatus) -> Self {
        Self {
            language: value.language.id.to_owned(),
            server: value.language.server_id.to_owned(),
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
        DiscoveryReason::UnsupportedOnThisPlatform => {
            LspSafeReasonCodeDto::UnsupportedOnThisPlatform
        }
        DiscoveryReason::PrerequisiteMissing => LspSafeReasonCodeDto::PrerequisiteMissing,
        DiscoveryReason::InstallDirectoryNotSet => LspSafeReasonCodeDto::InstallDirectoryNotSet,
        DiscoveryReason::LauncherNotFound => LspSafeReasonCodeDto::LauncherNotFound,
        DiscoveryReason::AmbiguousInstall => LspSafeReasonCodeDto::AmbiguousInstall,
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
