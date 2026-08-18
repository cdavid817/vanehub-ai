use std::collections::BTreeMap;
use thiserror::Error;

pub(crate) const MAX_RUNNER_ID_CHARS: usize = 128;
pub(crate) const MAX_RUNNER_LABEL_CHARS: usize = 160;
pub(crate) const MAX_RUNNER_ARGUMENTS: usize = 256;
pub(crate) const MAX_RUNNER_ARGUMENT_CHARS: usize = 16_384;
pub(crate) const MAX_RUNNER_ENVIRONMENT_KEYS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RunnerKind {
    Local,
    Ssh,
    Docker,
    Cloud,
}

impl RunnerKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Ssh => "ssh",
            Self::Docker => "docker",
            Self::Cloud => "cloud",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunnerRecoveryMode {
    None,
    InspectOnly,
    Reattach,
}

impl RunnerRecoveryMode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::InspectOnly => "inspect_only",
            Self::Reattach => "reattach",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunnerCapabilities {
    pub(crate) interactive_input: bool,
    pub(crate) pty: bool,
    pub(crate) cancellation: bool,
    pub(crate) inspection: bool,
    pub(crate) recovery: RunnerRecoveryMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunnerSelection {
    pub(crate) kind: RunnerKind,
    pub(crate) target_id: Option<String>,
    pub(crate) target_revision: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunnerPermissionContext {
    pub(crate) agent_id: String,
    pub(crate) session_id: String,
    pub(crate) generation_id: String,
    pub(crate) project_key: String,
    pub(crate) action: String,
    pub(crate) selection: RunnerSelection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunnerPolicyWitness {
    pub(crate) fingerprint: String,
}

pub(crate) trait RunnerPermissionPort: Send + Sync {
    fn authorize(
        &self,
        context: &RunnerPermissionContext,
    ) -> Result<RunnerPolicyWitness, RunnerError>;

    fn revalidate(
        &self,
        context: &RunnerPermissionContext,
        witness: &RunnerPolicyWitness,
    ) -> Result<(), RunnerError>;
}

impl Default for RunnerSelection {
    fn default() -> Self {
        Self::local()
    }
}

impl RunnerSelection {
    pub(crate) fn local() -> Self {
        Self {
            kind: RunnerKind::Local,
            target_id: None,
            target_revision: None,
        }
    }

    pub(crate) fn ssh(target_id: String, target_revision: i64) -> Result<Self, RunnerError> {
        validate_token(&target_id, "runner_target")?;
        if target_revision < 1 {
            return Err(RunnerError::new(RunnerErrorKind::InvalidSelection));
        }
        Ok(Self {
            kind: RunnerKind::Ssh,
            target_id: Some(target_id),
            target_revision: Some(target_revision),
        })
    }

    pub(crate) fn validate(&self) -> Result<(), RunnerError> {
        match self.kind {
            RunnerKind::Local => {
                if self.target_id.is_none() && self.target_revision.is_none() {
                    Ok(())
                } else {
                    Err(RunnerError::new(RunnerErrorKind::InvalidSelection))
                }
            }
            RunnerKind::Ssh => {
                validate_token(
                    self.target_id
                        .as_deref()
                        .ok_or_else(|| RunnerError::new(RunnerErrorKind::InvalidSelection))?,
                    "runner_target",
                )?;
                if self.target_revision.is_some_and(|revision| revision > 0) {
                    Ok(())
                } else {
                    Err(RunnerError::new(RunnerErrorKind::InvalidSelection))
                }
            }
            _ => Err(RunnerError::new(RunnerErrorKind::UnsupportedCapability)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunnerDescriptor {
    pub(crate) selection: RunnerSelection,
    pub(crate) label: String,
    pub(crate) host_label: Option<String>,
    pub(crate) available: bool,
    pub(crate) unavailable_reason: Option<&'static str>,
    pub(crate) simulated: bool,
    pub(crate) capabilities: RunnerCapabilities,
}

impl RunnerDescriptor {
    pub(crate) fn validate(&self) -> Result<(), RunnerError> {
        self.selection.validate()?;
        validate_value(&self.label, MAX_RUNNER_LABEL_CHARS)?;
        if self
            .host_label
            .as_deref()
            .is_some_and(|value| validate_value(value, MAX_RUNNER_LABEL_CHARS).is_err())
            || (self.available && self.unavailable_reason.is_some())
        {
            return Err(RunnerError::new(RunnerErrorKind::InvalidSelection));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunnerLaunchSpec {
    pub(crate) session_id: Option<String>,
    pub(crate) executable: String,
    pub(crate) arguments: Vec<String>,
    pub(crate) cwd: Option<String>,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) pipe_stdin: bool,
}

impl RunnerLaunchSpec {
    pub(crate) fn validate(&self) -> Result<(), RunnerError> {
        if self
            .session_id
            .as_deref()
            .is_some_and(|value| validate_token(value, "session_id").is_err())
        {
            return Err(RunnerError::new(RunnerErrorKind::InvalidLaunch));
        }
        validate_value(&self.executable, MAX_RUNNER_ID_CHARS)?;
        if self.arguments.len() > MAX_RUNNER_ARGUMENTS
            || self.environment.len() > MAX_RUNNER_ENVIRONMENT_KEYS
            || self
                .arguments
                .iter()
                .any(|value| validate_value(value, MAX_RUNNER_ARGUMENT_CHARS).is_err())
            || self
                .cwd
                .as_deref()
                .is_some_and(|value| validate_value(value, MAX_RUNNER_ARGUMENT_CHARS).is_err())
            || self.environment.iter().any(|(key, value)| {
                !valid_environment_key(key)
                    || validate_value(value, MAX_RUNNER_ARGUMENT_CHARS).is_err()
            })
        {
            return Err(RunnerError::new(RunnerErrorKind::InvalidLaunch));
        }
        Ok(())
    }

    pub(crate) fn validate_for(&self, kind: RunnerKind) -> Result<(), RunnerError> {
        self.validate()?;
        if self
            .environment
            .keys()
            .any(|key| !matches!(key.as_str(), "TRACEPARENT"))
            || (kind == RunnerKind::Ssh && self.arguments.iter().any(|value| secret_flag(value)))
        {
            return Err(RunnerError::new(RunnerErrorKind::PermissionDenied));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunnerReference {
    pub(crate) kind: RunnerKind,
    pub(crate) target_id: String,
    pub(crate) target_revision: Option<i64>,
    pub(crate) recovery: RunnerRecoveryMode,
    pub(crate) authority_witness: String,
}

impl RunnerReference {
    pub(crate) fn local() -> Self {
        Self {
            kind: RunnerKind::Local,
            target_id: "local".to_string(),
            target_revision: None,
            recovery: RunnerRecoveryMode::None,
            authority_witness: "local-runner-v1".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedRunnerLaunch {
    pub(crate) reference: RunnerReference,
    pub(crate) spec: RunnerLaunchSpec,
    pub(crate) preparation_id: Option<String>,
    pub(crate) admission_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunnerHandle {
    pub(crate) id: String,
    pub(crate) reference: RunnerReference,
    pub(crate) process_reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunnerEvent {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
    Exited(Option<i32>),
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunnerInspection {
    Running,
    Exited(Option<i32>),
    Disconnected,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunnerErrorKind {
    UnsupportedCapability,
    InvalidSelection,
    AuthorityStale,
    PermissionDenied,
    InvalidLaunch,
    Preparation,
    Spawn,
    Input,
    Disconnected,
    ReconnectExhausted,
    Cancellation,
    Inspection,
    Cleanup,
    ResourceExhausted,
}

impl RunnerErrorKind {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::UnsupportedCapability => "runner_unsupported_capability",
            Self::InvalidSelection => "runner_invalid_selection",
            Self::AuthorityStale => "runner_authority_stale",
            Self::PermissionDenied => "runner_permission_denied",
            Self::InvalidLaunch => "runner_invalid_launch",
            Self::Preparation => "runner_preparation_failed",
            Self::Spawn => "runner_spawn_failed",
            Self::Input => "runner_input_failed",
            Self::Disconnected => "runner_disconnected",
            Self::ReconnectExhausted => "runner_reconnect_exhausted",
            Self::Cancellation => "runner_cancellation_failed",
            Self::Inspection => "runner_inspection_failed",
            Self::Cleanup => "runner_cleanup_failed",
            Self::ResourceExhausted => "runner_resource_exhausted",
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("{kind:?}")]
pub(crate) struct RunnerError {
    pub(crate) kind: RunnerErrorKind,
}

impl RunnerError {
    pub(crate) const fn new(kind: RunnerErrorKind) -> Self {
        Self { kind }
    }

    pub(crate) const fn code(&self) -> &'static str {
        self.kind.code()
    }
}

pub(crate) trait AgentRunner: Send + Sync {
    fn kind(&self) -> RunnerKind;
    fn capabilities(&self) -> RunnerCapabilities;
    fn prepare(
        &self,
        selection: &RunnerSelection,
        spec: RunnerLaunchSpec,
    ) -> Result<PreparedRunnerLaunch, RunnerError>;
    fn spawn(&self, prepared: PreparedRunnerLaunch) -> Result<RunnerHandle, RunnerError>;
    fn send_input(&self, handle: &RunnerHandle, content: &[u8]) -> Result<(), RunnerError>;
    fn next_event(&self, handle: &RunnerHandle) -> Result<Option<RunnerEvent>, RunnerError>;
    fn cancel(&self, handle: &RunnerHandle) -> Result<bool, RunnerError>;
    fn inspect(&self, handle: &RunnerHandle) -> Result<RunnerInspection, RunnerError>;
    fn cleanup(&self, handle: &RunnerHandle) -> Result<(), RunnerError>;
    fn recover(
        &self,
        reference: &RunnerReference,
        process_reference: Option<&str>,
    ) -> Result<RunnerInspection, RunnerError>;
}

fn validate_token(value: &str, _field: &'static str) -> Result<(), RunnerError> {
    if value.is_empty()
        || value.len() > MAX_RUNNER_ID_CHARS
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(RunnerError::new(RunnerErrorKind::InvalidSelection));
    }
    Ok(())
}

fn validate_value(value: &str, limit: usize) -> Result<(), RunnerError> {
    if value.is_empty() || value.len() > limit || value.chars().any(char::is_control) {
        Err(RunnerError::new(RunnerErrorKind::InvalidLaunch))
    } else {
        Ok(())
    }
}

fn valid_environment_key(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RUNNER_ID_CHARS
        && value.bytes().enumerate().all(|(index, byte)| {
            byte == b'_' || byte.is_ascii_alphabetic() || (index > 0 && byte.is_ascii_digit())
        })
}

fn secret_flag(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase();
    [
        "api-key",
        "apikey",
        "token",
        "password",
        "secret",
        "credential",
    ]
    .iter()
    .any(|marker| normalized.starts_with('-') && normalized.contains(marker))
}
