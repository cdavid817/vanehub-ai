use crate::contexts::tooling::skills::domain::SkillDomainError;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillApplicationError {
    Domain(SkillDomainError),
    Validation(String),
    NotFound(String),
    Conflict(String),
    ConcurrentModification(String),
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
            Self::Validation(message) => formatter.write_str(message),
            Self::NotFound(skill_id) => write!(formatter, "Skill not found: {skill_id}"),
            Self::Conflict(skill_id) => write!(formatter, "Skill already exists: {skill_id}"),
            Self::ConcurrentModification(skill_id) => {
                write!(formatter, "Skill changed since it was loaded: {skill_id}")
            }
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
