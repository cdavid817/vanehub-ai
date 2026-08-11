use super::overlay_models::{OverlayIntegrityCode, OverlayLimitKind};
use crate::contexts::tooling::skills::domain::SkillDomainError;
use std::fmt;

// Later Overlay service tasks construct these boundary errors; keep this staged contract lint-clean.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OverlayApplicationError {
    InvalidRequest {
        code: String,
    },
    PinnedRefusal {
        skill_id: String,
    },
    StaleWitnesses {
        expected_revision: Option<u64>,
        current_revision: Option<u64>,
        base_changed: bool,
        payload_changed: bool,
        pin_changed: bool,
    },
    LimitExceeded {
        kind: OverlayLimitKind,
        maximum: u64,
        actual: u64,
    },
    Integrity {
        code: OverlayIntegrityCode,
    },
    TrustRequired {
        revision: u64,
    },
    ImportRejected {
        code: String,
    },
    PromotionWitnessMismatch {
        reviewed_revision: u64,
        current_revision: u64,
        document_hash_changed: bool,
        scan_changed: bool,
    },
    NeedsReconciliation {
        conflict_count: usize,
    },
}

impl fmt::Display for OverlayApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest { code } => write!(formatter, "Invalid Overlay request: {code}"),
            Self::PinnedRefusal { skill_id } => {
                write!(formatter, "Pinned Skill refuses Overlay mutation: {skill_id}")
            }
            Self::StaleWitnesses { .. } => {
                formatter.write_str("Overlay witnesses are stale; reload and preview again")
            }
            Self::LimitExceeded {
                kind,
                maximum,
                actual,
            } => write!(
                formatter,
                "Overlay {} limit exceeded: maximum {maximum}, actual {actual}",
                kind.as_str()
            ),
            Self::Integrity { code } => {
                write!(formatter, "Overlay integrity verification failed: {}", code.as_str())
            }
            Self::TrustRequired { revision } => {
                write!(formatter, "Overlay revision {revision} requires trust promotion")
            }
            Self::ImportRejected { code } => write!(formatter, "Overlay import rejected: {code}"),
            Self::PromotionWitnessMismatch {
                reviewed_revision,
                current_revision,
                document_hash_changed,
                scan_changed,
            } => write!(
                formatter,
                "Overlay promotion witness changed from revision {reviewed_revision} to {current_revision} (document hash changed: {document_hash_changed}, scan changed: {scan_changed})"
            ),
            Self::NeedsReconciliation { conflict_count } => write!(
                formatter,
                "Overlay requires reconciliation for {conflict_count} conflict(s)"
            ),
        }
    }
}

impl std::error::Error for OverlayApplicationError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillApplicationError {
    Domain(SkillDomainError),
    Overlay(OverlayApplicationError),
    Validation(String),
    NotFound(String),
    Conflict(String),
    ConcurrentModification(String),
    ImmutablePackage(String),
    InvalidResourceUri,
    ResourceEscape,
    BinaryResource,
    OversizedResource,
    Repository(String),
    Filesystem(String),
    MountRootExternalLink(String),
    MountRootBrokenLink(String),
    MountRootNotDirectory(String),
    Selection(String),
    Logging(String),
}

impl fmt::Display for SkillApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::Overlay(error) => error.fmt(formatter),
            Self::Validation(message) => formatter.write_str(message),
            Self::NotFound(skill_id) => write!(formatter, "Skill not found: {skill_id}"),
            Self::Conflict(skill_id) => write!(formatter, "Skill already exists: {skill_id}"),
            Self::ConcurrentModification(skill_id) => {
                write!(formatter, "Skill changed since it was loaded: {skill_id}")
            }
            Self::ImmutablePackage(skill_id) => write!(
                formatter,
                "System Skill package is immutable: {skill_id}. Create a higher-layer definition to customize it"
            ),
            Self::InvalidResourceUri => formatter.write_str("Invalid Skill resource URI"),
            Self::ResourceEscape => formatter.write_str("Skill resource escapes its package"),
            Self::BinaryResource => formatter.write_str("Skill resource is not UTF-8 text"),
            Self::OversizedResource => formatter.write_str("Skill resource exceeds read limits"),
            Self::MountRootExternalLink(agent_id) => write!(
                formatter,
                "The Skill root for {agent_id} is managed by an external directory link. Migrate the whole-directory link to a normal directory before assigning Skills."
            ),
            Self::MountRootBrokenLink(agent_id) => write!(
                formatter,
                "The Skill root for {agent_id} is a broken directory link. Repair or remove the stale link before assigning Skills."
            ),
            Self::MountRootNotDirectory(agent_id) => write!(
                formatter,
                "The Skill root for {agent_id} is not a directory. Move the conflicting entry before assigning Skills."
            ),
            Self::Repository(message)
            | Self::Filesystem(message)
            | Self::Selection(message)
            | Self::Logging(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for SkillApplicationError {}

impl From<SkillDomainError> for SkillApplicationError {
    fn from(error: SkillDomainError) -> Self {
        Self::Domain(error)
    }
}

impl From<OverlayApplicationError> for SkillApplicationError {
    fn from(error: OverlayApplicationError) -> Self {
        Self::Overlay(error)
    }
}
