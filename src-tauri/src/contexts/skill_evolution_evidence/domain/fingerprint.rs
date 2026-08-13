use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use thiserror::Error;
use zeroize::Zeroizing;

use super::{SignalCategory, SignalDraft};

pub(crate) const TASK_FINGERPRINT_V1: u16 = 1;
const MIN_FINGERPRINT_KEY_BYTES: usize = 32;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum FingerprintError {
    #[error("installation fingerprint key is too short")]
    WeakInstallationKey,
    #[error("installation fingerprint key is invalid")]
    InvalidInstallationKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskFingerprint {
    version: u16,
    value: String,
}

impl TaskFingerprint {
    pub(crate) fn version(&self) -> u16 {
        self.version
    }

    pub(crate) fn value(&self) -> &str {
        &self.value
    }
}

pub(crate) struct TaskFingerprintBuilder {
    key: Zeroizing<Vec<u8>>,
}

pub(crate) fn canonical_workspace_scope(
    key: &[u8],
    workspace: &str,
) -> Result<String, FingerprintError> {
    if key.len() < MIN_FINGERPRINT_KEY_BYTES {
        return Err(FingerprintError::WeakInstallationKey);
    }
    if workspace.starts_with("workspace:")
        && workspace.len() == 34
        && workspace[10..].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Ok(workspace.to_ascii_lowercase());
    }
    let normalized = Zeroizing::new(workspace.trim().replace('\\', "/").to_lowercase());
    let mut mac = Hmac::<Sha256>::new_from_slice(key)
        .map_err(|_| FingerprintError::InvalidInstallationKey)?;
    mac.update(b"workspace-v1|");
    mac.update(normalized.as_bytes());
    let digest = mac.finalize().into_bytes();
    let token = digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!("workspace:{token}"))
}

impl TaskFingerprintBuilder {
    pub(crate) fn new(key: &[u8]) -> Result<Self, FingerprintError> {
        if key.len() < MIN_FINGERPRINT_KEY_BYTES {
            return Err(FingerprintError::WeakInstallationKey);
        }
        Ok(Self {
            key: Zeroizing::new(key.to_vec()),
        })
    }

    pub(crate) fn build(&self, draft: &SignalDraft) -> Result<TaskFingerprint, FingerprintError> {
        let mut mac = Hmac::<Sha256>::new_from_slice(self.key.as_slice())
            .map_err(|_| FingerprintError::InvalidInstallationKey)?;
        mac.update(normalized_material(draft).as_bytes());
        let value = mac
            .finalize()
            .into_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Ok(TaskFingerprint {
            version: TASK_FINGERPRINT_V1,
            value,
        })
    }
}

fn normalized_material(draft: &SignalDraft) -> String {
    let workspace = draft.workspace().unwrap_or("global").trim().to_lowercase();
    let category = grouping_category(draft.category());
    let shape = if draft.category() == SignalCategory::ExplicitFeedback {
        normalize_whitespace(draft.safe_summary())
    } else {
        normalize_discriminator(draft.discriminator())
    };
    format!("v{TASK_FINGERPRINT_V1}|{workspace}|{category}|{shape}")
}

fn grouping_category(category: SignalCategory) -> &'static str {
    match category {
        SignalCategory::RetryRecovery | SignalCategory::ExecutionFailure => "execution_pattern",
        SignalCategory::ExplicitFeedback => "explicit_feedback",
        SignalCategory::VerificationOutcome => "verification_outcome",
        SignalCategory::DelegationOutcome => "delegation_outcome",
        SignalCategory::SkillLifecycleAnomaly => "skill_lifecycle_anomaly",
    }
}

fn normalize_discriminator(value: &str) -> String {
    if value.starts_with("retry:") {
        "execution".to_string()
    } else {
        normalize_whitespace(value)
    }
}

fn normalize_whitespace(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
