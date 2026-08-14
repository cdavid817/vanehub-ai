use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum EvidenceValidationError {
    #[error("unsupported evidence envelope schema version {0}")]
    UnsupportedSchemaVersion(u16),
    #[error("invalid timestamp in {0}")]
    InvalidTimestamp(&'static str),
    #[error("malformed identifier in {0}")]
    MalformedIdentifier(&'static str),
    #[error("too many observed Skill revisions; maximum is {max}")]
    TooManyObservedSkillRevisions { max: usize },
    #[error("too many CLI Skill bindings; maximum is {max}")]
    TooManyCliBindings { max: usize },
    #[error("correction note exceeds {max} characters")]
    CorrectionNoteTooLong { max: usize },
}
