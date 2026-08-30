use super::{ActivityEnvelopeError, EvolutionActivityEnvelopeV1};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

const MAX_CURSOR_BYTES: usize = 512;
pub(crate) const MAX_SOURCE_SCAN_ITEMS: u16 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvolutionSourceDomain {
    Orchestration,
    Evidence,
    Assessment,
    Generation,
    Curator,
    Overlay,
    AutomaticApplication,
    Probation,
    Breaker,
    SkillCreation,
    Recovery,
    Retention,
}

impl EvolutionSourceDomain {
    /// Every bounded projection source domain, for health reporting.
    pub(crate) const ALL: &'static [Self] = &[
        Self::Orchestration,
        Self::Evidence,
        Self::Assessment,
        Self::Generation,
        Self::Curator,
        Self::Overlay,
        Self::AutomaticApplication,
        Self::Probation,
        Self::Breaker,
        Self::SkillCreation,
        Self::Recovery,
        Self::Retention,
    ];

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Orchestration => "orchestration",
            Self::Evidence => "evidence",
            Self::Assessment => "assessment",
            Self::Generation => "generation",
            Self::Curator => "curator",
            Self::Overlay => "overlay",
            Self::AutomaticApplication => "automatic_application",
            Self::Probation => "probation",
            Self::Breaker => "breaker",
            Self::SkillCreation => "skill_creation",
            Self::Recovery => "recovery",
            Self::Retention => "retention",
        }
    }
}

impl FromStr for EvolutionSourceDomain {
    type Err = ProjectionSourceError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "orchestration" => Ok(Self::Orchestration),
            "evidence" => Ok(Self::Evidence),
            "assessment" => Ok(Self::Assessment),
            "generation" => Ok(Self::Generation),
            "curator" => Ok(Self::Curator),
            "overlay" => Ok(Self::Overlay),
            "automatic_application" => Ok(Self::AutomaticApplication),
            "probation" => Ok(Self::Probation),
            "breaker" => Ok(Self::Breaker),
            "skill_creation" => Ok(Self::SkillCreation),
            "recovery" => Ok(Self::Recovery),
            "retention" => Ok(Self::Retention),
            _ => Err(ProjectionSourceError::ProhibitedOrUnknownDomain),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct OpaqueDomainCursor(String);

impl OpaqueDomainCursor {
    pub(crate) fn parse(value: String) -> Result<Self, ProjectionSourceError> {
        if value.is_empty() || value.len() > MAX_CURSOR_BYTES || value.chars().any(char::is_control)
        {
            return Err(ProjectionSourceError::InvalidCursor);
        }
        Ok(Self(value))
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionScanLimit(u16);

impl ProjectionScanLimit {
    pub(crate) fn new(value: u16) -> Result<Self, ProjectionSourceError> {
        if value == 0 || value > MAX_SOURCE_SCAN_ITEMS {
            return Err(ProjectionSourceError::InvalidScanLimit);
        }
        Ok(Self(value))
    }

    pub(crate) fn get(self) -> u16 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedProjectionEvent {
    pub(crate) source_cursor: OpaqueDomainCursor,
    pub(crate) source_sequence: u64,
    pub(crate) source_integrity_hash: String,
    pub(crate) envelope: EvolutionActivityEnvelopeV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectionSourcePage {
    pub(crate) source_domain: EvolutionSourceDomain,
    pub(crate) events: Vec<VerifiedProjectionEvent>,
    pub(crate) next_cursor: Option<OpaqueDomainCursor>,
    pub(crate) retention_floor: Option<OpaqueDomainCursor>,
    pub(crate) has_more: bool,
}

impl ProjectionSourcePage {
    pub(crate) fn validate(&self, limit: ProjectionScanLimit) -> Result<(), ProjectionSourceError> {
        if self.events.len() > usize::from(limit.get()) {
            return Err(ProjectionSourceError::UnboundedPage);
        }
        let mut previous = None;
        for event in &self.events {
            if event.source_sequence == 0
                || previous.is_some_and(|value| event.source_sequence <= value)
            {
                return Err(ProjectionSourceError::InvalidSequence);
            }
            if event.source_integrity_hash.is_empty() {
                return Err(ProjectionSourceError::IntegrityFailed);
            }
            event.envelope.validate()?;
            previous = Some(event.source_sequence);
        }
        if self.has_more && self.next_cursor.is_none() {
            return Err(ProjectionSourceError::MissingNextCursor);
        }
        Ok(())
    }
}

pub(crate) trait EvolutionProjectionSource: Send + Sync {
    fn domain(&self) -> EvolutionSourceDomain;

    /// Implementations scan only committed immutable records and perform integrity validation
    /// before returning safe mapped envelopes across this boundary.
    fn scan_committed(
        &self,
        after: Option<&OpaqueDomainCursor>,
        limit: ProjectionScanLimit,
    ) -> Result<ProjectionSourcePage, ProjectionSourceError>;
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum ProjectionSourceError {
    #[error("projection source domain is prohibited or unknown")]
    ProhibitedOrUnknownDomain,
    #[error("projection source cursor is invalid")]
    InvalidCursor,
    #[error("projection source scan limit must be between 1 and {MAX_SOURCE_SCAN_ITEMS}")]
    InvalidScanLimit,
    #[error("projection source returned more records than requested")]
    UnboundedPage,
    #[error("projection source sequence is missing or not strictly increasing")]
    InvalidSequence,
    #[error("projection source integrity validation failed")]
    IntegrityFailed,
    #[error("projection source page has more records but no next cursor")]
    MissingNextCursor,
    #[error(transparent)]
    InvalidEnvelope(#[from] ActivityEnvelopeError),
    #[error("projection source is unavailable")]
    Unavailable,
}
