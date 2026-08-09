use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use url::Url;

pub(crate) const PAYLOAD_VERSION: i64 = 1;
pub(crate) const SUPPORTED_AGENT_IDS: [&str; 4] =
    ["claude-code", "opencode", "codex-cli", "antigravity-cli"];

/// Keys VaneHub owns inside Antigravity's settings document. Everything else in that file belongs
/// to the user and is preserved on apply.
pub(crate) const ANTIGRAVITY_MANAGED_KEYS: [&str; 4] = [
    "enableTerminalSandbox",
    "model",
    "toolPermission",
    "verbosity",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliConfigDriftState {
    Detached,
    Applied,
    Drifted,
    Malformed,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliConfigValidationState {
    Valid,
    NeedsCredential,
    Invalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliConfigAppliedState {
    Saved,
    Applied,
    Drifted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliConfigStartupSyncState {
    Pending,
    Imported,
    Updated,
    Unchanged,
    Skipped,
    Warning,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliConfigStartupSyncResult {
    pub(crate) agent_id: String,
    pub(crate) state: CliConfigStartupSyncState,
    pub(crate) imported: usize,
    pub(crate) updated: usize,
    pub(crate) skipped: usize,
    pub(crate) warnings: Vec<String>,
    pub(crate) synchronized_at: Option<String>,
    pub(crate) simulated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliConfigDriftResolution {
    ImportCurrent,
    Discard,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ClaudeAuthMode {
    AuthToken,
    ApiKey,
    None,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CodexWireApi {
    Responses,
    Chat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CodexAuthStrategy {
    PreserveOfficial,
    BearerToken,
    ReplaceAuth,
}

/// Antigravity CLI's graduated tool-approval modes, which live in its settings document rather
/// than in launch flags.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum AntigravityToolPermission {
    RequestReview,
    ProceedInSandbox,
    AlwaysProceed,
    Strict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum CliConfigPayload {
    ClaudeCode {
        base_url: String,
        auth_mode: ClaudeAuthMode,
        model: String,
        haiku_model: String,
        sonnet_model: String,
        opus_model: String,
        advanced_env: BTreeMap<String, String>,
    },
    CodexCli {
        provider_id: String,
        base_url: String,
        model: String,
        wire_api: CodexWireApi,
        reasoning_effort: String,
        auth_strategy: CodexAuthStrategy,
        advanced_toml: BTreeMap<String, Value>,
    },
    Opencode {
        provider_id: String,
        provider_name: String,
        npm: String,
        base_url: String,
        headers: BTreeMap<String, String>,
        models: Vec<OpenCodeModelDefinition>,
        default_model: String,
    },
    /// Antigravity CLI authenticates through the OS keyring with Google Sign-In and speaks a
    /// Google-proprietary protocol, so this payload carries no credential and no endpoint: it
    /// manages the settings the CLI actually honors.
    Antigravity {
        tool_permission: AntigravityToolPermission,
        enable_terminal_sandbox: bool,
        verbosity: String,
        model: String,
        advanced_settings: BTreeMap<String, Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenCodeModelDefinition {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliConfigProfile {
    pub(crate) id: String,
    pub(crate) agent_id: String,
    pub(crate) name: String,
    pub(crate) payload_version: i64,
    pub(crate) payload: CliConfigPayload,
    pub(crate) source_preset_id: Option<String>,
    pub(crate) source_preset_version: Option<i64>,
    pub(crate) credential_configured: bool,
    pub(crate) validation_state: CliConfigValidationState,
    pub(crate) applied_state: CliConfigAppliedState,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveCliConfigProfileInput {
    pub(crate) id: Option<String>,
    pub(crate) agent_id: String,
    pub(crate) name: String,
    pub(crate) payload: CliConfigPayload,
    pub(crate) source_preset_id: Option<String>,
    pub(crate) source_preset_version: Option<i64>,
    pub(crate) credential: Option<String>,
    #[serde(default)]
    pub(crate) remove_credential: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ValidateCliConfigCredentialInput {
    pub(crate) agent_id: String,
    pub(crate) profile_id: Option<String>,
    pub(crate) payload: Option<CliConfigPayload>,
    pub(crate) source_preset_id: Option<String>,
    pub(crate) credential: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportCliConfigProfileInput {
    pub(crate) agent_id: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliConfigDiscoveryState {
    Available,
    ParseError,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliConfigDiscoveryCandidate {
    pub(crate) candidate_key: String,
    pub(crate) suggested_name: String,
    pub(crate) provider_name: String,
    pub(crate) endpoint: String,
    pub(crate) model: String,
    pub(crate) credential_detected: bool,
    pub(crate) is_default: bool,
    pub(crate) resolved_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliConfigDiscoveryResult {
    pub(crate) agent_id: String,
    pub(crate) state: CliConfigDiscoveryState,
    pub(crate) candidates: Vec<CliConfigDiscoveryCandidate>,
    pub(crate) resolved_paths: Vec<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) error: Option<String>,
    pub(crate) simulated: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportDiscoveredCliConfigInput {
    pub(crate) agent_id: String,
    pub(crate) candidate_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkippedCliConfigDiscoveryCandidate {
    pub(crate) candidate_key: String,
    pub(crate) suggested_name: String,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportDiscoveredCliConfigResult {
    pub(crate) imported: Vec<CliConfigProfile>,
    pub(crate) skipped: Vec<SkippedCliConfigDiscoveryCandidate>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteCliConfigProfileInput {
    pub(crate) agent_id: String,
    pub(crate) profile_id: String,
    #[serde(default)]
    pub(crate) detach_applied: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApplyCliConfigProfileInput {
    pub(crate) agent_id: String,
    pub(crate) profile_id: String,
    pub(crate) drift_resolution: Option<CliConfigDriftResolution>,
    #[serde(default)]
    pub(crate) confirm_auth_file_replacement: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliConfigStatus {
    pub(crate) agent_id: String,
    pub(crate) applied_profile_id: Option<String>,
    pub(crate) drift_state: CliConfigDriftState,
    pub(crate) resolved_paths: Vec<String>,
    pub(crate) last_applied_at: Option<String>,
    pub(crate) simulated: bool,
    pub(crate) startup_sync: CliConfigStartupSyncResult,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliConfigApplyResult {
    pub(crate) operation_id: String,
    pub(crate) status: String,
    pub(crate) agent_id: String,
    pub(crate) profile_id: String,
    pub(crate) affected_paths: Vec<String>,
    pub(crate) drift_resolution: Option<CliConfigDriftResolution>,
    pub(crate) backfilled_profile_id: Option<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) restart_required: bool,
    pub(crate) simulated: bool,
    pub(crate) restored: bool,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProfileRecord {
    pub(crate) id: String,
    pub(crate) agent_id: String,
    pub(crate) name: String,
    pub(crate) payload_version: i64,
    pub(crate) payload: CliConfigPayload,
    pub(crate) managed_keys: Vec<String>,
    pub(crate) source_preset_id: Option<String>,
    pub(crate) source_preset_version: Option<i64>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) sort_position: i64,
}

#[derive(Debug, Clone)]
pub(crate) struct AppliedStateRecord {
    pub(crate) agent_id: String,
    pub(crate) profile_id: Option<String>,
    pub(crate) projection_fingerprint: String,
    pub(crate) live_fingerprint: String,
    pub(crate) drift_state: CliConfigDriftState,
    pub(crate) applied_at: String,
    pub(crate) applied_payload: Option<CliConfigPayload>,
    pub(crate) managed_keys: Vec<String>,
}

#[derive(Debug, Error)]
pub(crate) enum CliConfigError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("profile not found")]
    NotFound,
    #[error("live configuration changed while the operation was in progress; retry the switch")]
    DriftConflict,
    #[error("credential repair is required")]
    CredentialRequired,
    #[error("explicit auth.json replacement confirmation is required")]
    AuthConfirmationRequired,
    #[error("configuration parse failed at {path}: {message}")]
    Parse { path: String, message: String },
    #[error("configuration storage operation failed")]
    Repository,
    #[error("secure credential operation failed")]
    Credential,
    #[error("configuration file operation failed at {path}")]
    Filesystem { path: String },
    #[error("configuration rollback was incomplete")]
    RollbackIncomplete,
}

pub(crate) fn validate_supported_agent(agent_id: &str) -> Result<(), CliConfigError> {
    if SUPPORTED_AGENT_IDS.contains(&agent_id) {
        Ok(())
    } else {
        Err(CliConfigError::Validation(format!(
            "unsupported CLI agent id: {agent_id}"
        )))
    }
}

pub(crate) fn validate_profile_input(
    input: &SaveCliConfigProfileInput,
) -> Result<(), CliConfigError> {
    validate_supported_agent(&input.agent_id)?;
    validate_name(&input.name)?;
    if input
        .credential
        .as_deref()
        .is_some_and(|secret| secret.is_empty() || secret.chars().any(char::is_control))
    {
        return Err(CliConfigError::Validation(
            "credential must not be empty or contain control characters".into(),
        ));
    }
    if input.payload.agent_id() != input.agent_id {
        return Err(CliConfigError::Validation(
            "profile payload does not match the selected Agent".into(),
        ));
    }
    if input.credential.is_some() && !input.payload.supports_credential() {
        return Err(CliConfigError::Validation(
            "this CLI authenticates outside VaneHub and does not accept a credential".into(),
        ));
    }
    input.payload.validate()
}

fn validate_name(name: &str) -> Result<(), CliConfigError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 80 || trimmed.chars().any(char::is_control) {
        return Err(CliConfigError::Validation(
            "profile name must contain 1 to 80 visible characters".into(),
        ));
    }
    Ok(())
}

fn validate_url(value: &str) -> Result<(), CliConfigError> {
    let parsed = Url::parse(value).map_err(|_| {
        CliConfigError::Validation("base URL must be an absolute HTTP(S) URL".into())
    })?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
    {
        return Err(CliConfigError::Validation(
            "base URL must be HTTP(S) without credentials or fragments".into(),
        ));
    }
    Ok(())
}

fn validate_id(value: &str, field: &str) -> Result<(), CliConfigError> {
    if value.is_empty()
        || value.len() > 120
        || value.contains("..")
        || value.contains(['/', '\\'])
        || value.chars().any(char::is_control)
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || "-_.:@".contains(character))
    {
        return Err(CliConfigError::Validation(format!("invalid {field}")));
    }
    Ok(())
}

fn validate_text(value: &str, field: &str) -> Result<(), CliConfigError> {
    if value.trim().is_empty() || value.len() > 512 || value.chars().any(char::is_control) {
        return Err(CliConfigError::Validation(format!("invalid {field}")));
    }
    Ok(())
}

fn validate_model_id(value: &str) -> Result<(), CliConfigError> {
    if value.trim().is_empty()
        || value.len() > 256
        || value.contains("..")
        || value.contains('\\')
        || value.chars().any(char::is_control)
    {
        return Err(CliConfigError::Validation("invalid model id".into()));
    }
    Ok(())
}

fn validate_string_map(
    values: &BTreeMap<String, String>,
    field: &str,
) -> Result<(), CliConfigError> {
    if values.len() > 32 {
        return Err(CliConfigError::Validation(format!(
            "too many {field} entries"
        )));
    }
    for (key, value) in values {
        validate_id(key, field)?;
        if value.len() > 2048 || value.chars().any(char::is_control) {
            return Err(CliConfigError::Validation(format!("invalid {field} value")));
        }
    }
    Ok(())
}

fn looks_like_secret_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    let compact = normalized.replace('_', "");
    [
        "apikey",
        "authtoken",
        "accesstoken",
        "refreshtoken",
        "bearertoken",
        "authorization",
        "credential",
        "password",
        "clientsecret",
        "privatekey",
        "secretaccesskey",
    ]
    .iter()
    .any(|marker| compact.contains(marker))
        || normalized
            .split('_')
            .any(|segment| matches!(segment, "secret" | "token" | "passwd"))
}

impl CliConfigPayload {
    pub(crate) fn agent_id(&self) -> String {
        match self {
            Self::ClaudeCode { .. } => "claude-code",
            Self::CodexCli { .. } => "codex-cli",
            Self::Opencode { .. } => "opencode",
            Self::Antigravity { .. } => "antigravity-cli",
        }
        .to_string()
    }

    /// Whether this kind can hold a credential at all. Declared here rather than branched on by
    /// Agent id at each call site, so credential capture, validation, and the `needs-credential`
    /// state all derive from one fact.
    pub(crate) fn supports_credential(&self) -> bool {
        !matches!(self, Self::Antigravity { .. })
    }

    pub(crate) fn requires_credential(&self) -> bool {
        match self {
            Self::ClaudeCode { auth_mode, .. } => *auth_mode != ClaudeAuthMode::None,
            Self::CodexCli { auth_strategy, .. } => {
                *auth_strategy != CodexAuthStrategy::PreserveOfficial
            }
            Self::Opencode { .. } => true,
            Self::Antigravity { .. } => false,
        }
    }

    pub(crate) fn managed_keys(&self) -> Vec<String> {
        match self {
            Self::ClaudeCode { advanced_env, .. } => {
                let mut keys = vec![
                    "ANTHROPIC_BASE_URL",
                    "ANTHROPIC_AUTH_TOKEN",
                    "ANTHROPIC_API_KEY",
                    "ANTHROPIC_MODEL",
                    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
                    "ANTHROPIC_DEFAULT_SONNET_MODEL",
                    "ANTHROPIC_DEFAULT_OPUS_MODEL",
                ]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>();
                keys.extend(advanced_env.keys().cloned());
                keys.sort();
                keys.dedup();
                keys
            }
            Self::CodexCli { provider_id, .. } => vec![
                "model".into(),
                "model_provider".into(),
                "model_reasoning_effort".into(),
                format!("model_providers.{provider_id}"),
            ],
            Self::Opencode { provider_id, .. } => {
                vec!["model".into(), format!("provider.{provider_id}")]
            }
            Self::Antigravity {
                advanced_settings, ..
            } => {
                let mut keys = ANTIGRAVITY_MANAGED_KEYS
                    .iter()
                    .map(|key| (*key).to_string())
                    .collect::<Vec<_>>();
                keys.extend(advanced_settings.keys().cloned());
                keys.sort();
                keys.dedup();
                keys
            }
        }
    }

    pub(crate) fn validate(&self) -> Result<(), CliConfigError> {
        match self {
            Self::ClaudeCode {
                base_url,
                model,
                haiku_model,
                sonnet_model,
                opus_model,
                advanced_env,
                ..
            } => {
                validate_url(base_url)?;
                for (value, field) in [
                    (model, "model"),
                    (haiku_model, "haiku model"),
                    (sonnet_model, "sonnet model"),
                    (opus_model, "opus model"),
                ] {
                    validate_text(value, field)?;
                }
                validate_string_map(advanced_env, "advanced environment")?;
                let reserved = [
                    "ANTHROPIC_BASE_URL",
                    "ANTHROPIC_AUTH_TOKEN",
                    "ANTHROPIC_API_KEY",
                    "ANTHROPIC_MODEL",
                    "ANTHROPIC_DEFAULT_HAIKU_MODEL",
                    "ANTHROPIC_DEFAULT_SONNET_MODEL",
                    "ANTHROPIC_DEFAULT_OPUS_MODEL",
                ];
                if advanced_env
                    .keys()
                    .any(|key| reserved.contains(&key.as_str()))
                {
                    return Err(CliConfigError::Validation(
                        "advanced environment cannot replace managed keys".into(),
                    ));
                }
                if advanced_env.keys().any(|key| looks_like_secret_key(key)) {
                    return Err(CliConfigError::Validation(
                        "advanced environment credentials must use the credential field".into(),
                    ));
                }
            }
            Self::CodexCli {
                provider_id,
                base_url,
                model,
                reasoning_effort,
                advanced_toml,
                ..
            } => {
                validate_id(provider_id, "provider id")?;
                validate_url(base_url)?;
                validate_text(model, "model")?;
                if !["none", "low", "medium", "high", "xhigh", "max"]
                    .contains(&reasoning_effort.as_str())
                {
                    return Err(CliConfigError::Validation(
                        "unsupported reasoning effort".into(),
                    ));
                }
                if advanced_toml.len() > 16
                    || advanced_toml.keys().any(|key| {
                        validate_id(key, "advanced TOML key").is_err()
                            || ["mcp_servers", "projects", "profiles"].contains(&key.as_str())
                            || looks_like_secret_key(key)
                    })
                    || advanced_toml.values().any(|value| {
                        !matches!(value, Value::String(_) | Value::Bool(_) | Value::Number(_))
                    })
                {
                    return Err(CliConfigError::Validation(
                        "advanced TOML contains unsupported keys or values".into(),
                    ));
                }
            }
            Self::Opencode {
                provider_id,
                provider_name,
                npm,
                base_url,
                headers,
                models,
                default_model,
            } => {
                validate_id(provider_id, "provider id")?;
                validate_text(provider_name, "provider name")?;
                validate_text(npm, "npm package")?;
                validate_url(base_url)?;
                validate_string_map(headers, "header")?;
                if headers.keys().any(|key| looks_like_secret_key(key)) {
                    return Err(CliConfigError::Validation(
                        "authorization credentials must use the credential field".into(),
                    ));
                }
                if models.is_empty() || models.len() > 64 {
                    return Err(CliConfigError::Validation(
                        "OpenCode requires 1 to 64 models".into(),
                    ));
                }
                let mut ids = BTreeSet::new();
                for model in models {
                    validate_model_id(&model.id)?;
                    validate_text(&model.name, "model name")?;
                    if !ids.insert(model.id.as_str()) {
                        return Err(CliConfigError::Validation("duplicate model id".into()));
                    }
                }
                if !ids.contains(default_model.as_str()) {
                    return Err(CliConfigError::Validation(
                        "default model must exist in the model list".into(),
                    ));
                }
            }
            Self::Antigravity {
                verbosity,
                model,
                advanced_settings,
                ..
            } => {
                validate_text(verbosity, "verbosity")?;
                validate_text(model, "model")?;
                if advanced_settings.len() > 16
                    || advanced_settings.keys().any(|key| {
                        validate_id(key, "advanced setting key").is_err()
                            || ANTIGRAVITY_MANAGED_KEYS.contains(&key.as_str())
                            || key == "permissions"
                            || looks_like_secret_key(key)
                    })
                    || advanced_settings.values().any(|value| {
                        !matches!(value, Value::String(_) | Value::Bool(_) | Value::Number(_))
                    })
                {
                    return Err(CliConfigError::Validation(
                        "advanced settings contain unsupported keys or values".into(),
                    ));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claude_input() -> SaveCliConfigProfileInput {
        SaveCliConfigProfileInput {
            id: None,
            agent_id: "claude-code".into(),
            name: "DeepSeek".into(),
            payload: CliConfigPayload::ClaudeCode {
                base_url: "https://api.deepseek.com/anthropic".into(),
                auth_mode: ClaudeAuthMode::AuthToken,
                model: "deepseek-chat".into(),
                haiku_model: "deepseek-chat".into(),
                sonnet_model: "deepseek-chat".into(),
                opus_model: "deepseek-chat".into(),
                advanced_env: BTreeMap::new(),
            },
            source_preset_id: None,
            source_preset_version: None,
            credential: Some("secret".into()),
            remove_credential: false,
        }
    }

    fn antigravity_input() -> SaveCliConfigProfileInput {
        SaveCliConfigProfileInput {
            id: None,
            agent_id: "antigravity-cli".into(),
            name: "Antigravity".into(),
            payload: CliConfigPayload::Antigravity {
                tool_permission: AntigravityToolPermission::RequestReview,
                enable_terminal_sandbox: false,
                verbosity: "high".into(),
                model: "gemini-3-pro".into(),
                advanced_settings: BTreeMap::new(),
            },
            source_preset_id: None,
            source_preset_version: None,
            credential: None,
            remove_credential: false,
        }
    }

    /// A credential-free kind must refuse a submitted secret before anything touches a config
    /// file, so a mis-wired caller cannot persist a credential the CLI would never read.
    #[test]
    fn credential_free_kinds_reject_a_submitted_credential() {
        assert!(validate_profile_input(&antigravity_input()).is_ok());

        let mut with_credential = antigravity_input();
        with_credential.credential = Some("should-not-be-accepted".into());
        let error = validate_profile_input(&with_credential)
            .expect_err("a credential-free kind must reject a credential");
        assert!(
            matches!(error, CliConfigError::Validation(_)),
            "expected a validation error, got {error:?}"
        );
    }

    /// `needs-credential` has to be unreachable for this kind, otherwise the UI would render a
    /// repair prompt for a credential that does not exist.
    #[test]
    fn antigravity_never_requires_or_supports_a_credential() {
        let CliConfigPayload::Antigravity { .. } = antigravity_input().payload else {
            panic!("fixture must be an Antigravity payload");
        };
        let payload = antigravity_input().payload;
        assert!(!payload.supports_credential());
        assert!(!payload.requires_credential());
    }

    #[test]
    fn validates_supported_tagged_payloads() {
        assert!(validate_profile_input(&claude_input()).is_ok());
        let mut mismatched = claude_input();
        mismatched.agent_id = "codex-cli".into();
        assert!(validate_profile_input(&mismatched).is_err());
    }

    #[test]
    fn rejects_out_of_scope_and_path_like_fields() {
        let mut invalid = claude_input();
        if let CliConfigPayload::ClaudeCode { advanced_env, .. } = &mut invalid.payload {
            advanced_env.insert("ANTHROPIC_API_KEY".into(), "leak".into());
        }
        assert!(validate_profile_input(&invalid).is_err());
        assert!(validate_id("../provider", "provider").is_err());
    }

    #[test]
    fn rejects_secret_bearing_advanced_fields_before_persistence() {
        let mut claude = claude_input().payload;
        if let CliConfigPayload::ClaudeCode { advanced_env, .. } = &mut claude {
            advanced_env.insert("OPENAI_API_KEY".into(), "must-not-reach-sqlite".into());
        }
        assert!(claude.validate().is_err());

        let codex = CliConfigPayload::CodexCli {
            provider_id: "third-party".into(),
            base_url: "https://example.com/v1".into(),
            model: "model".into(),
            wire_api: CodexWireApi::Responses,
            reasoning_effort: "medium".into(),
            auth_strategy: CodexAuthStrategy::BearerToken,
            advanced_toml: BTreeMap::from([(
                "api_key".into(),
                Value::String("must-not-reach-sqlite".into()),
            )]),
        };
        assert!(codex.validate().is_err());

        let opencode = CliConfigPayload::Opencode {
            provider_id: "third-party".into(),
            provider_name: "Third Party".into(),
            npm: "@ai-sdk/openai-compatible".into(),
            base_url: "https://example.com/v1".into(),
            headers: BTreeMap::from([("X-API-Key".into(), "must-not-reach-sqlite".into())]),
            models: vec![OpenCodeModelDefinition {
                id: "model".into(),
                name: "Model".into(),
            }],
            default_model: "model".into(),
        };
        assert!(opencode.validate().is_err());
    }
}
