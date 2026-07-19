use crate::contexts::tooling::skills::domain::SkillDomainError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillApplicationError {
    Domain(SkillDomainError),
    Validation(String),
    NotFound(String),
    Conflict(String),
    Repository(String),
    Filesystem(String),
    Selection(String),
    Logging(String),
}

impl fmt::Display for SkillApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Domain(error) => error.fmt(formatter),
            Self::Validation(message) => formatter.write_str(message),
            Self::NotFound(skill_id) => write!(formatter, "Skill not found: {skill_id}"),
            Self::Conflict(skill_id) => write!(formatter, "Skill already exists: {skill_id}"),
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
