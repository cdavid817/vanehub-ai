use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillDomainError {
    InvalidId,
    MissingMetadataFields,
    WorkspacePathRequired,
    CreateIdMismatch,
    UpdateIdChanged,
    InvalidMountPath(String),
    UnknownAgent(String),
    InvalidUserSource(String),
    UnknownBuiltin(String),
}

impl fmt::Display for SkillDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId => {
                formatter.write_str("Skill id must be kebab-case letters, digits, and hyphens")
            }
            Self::MissingMetadataFields => formatter
                .write_str("Skill metadata requires name, description, category, and version"),
            Self::WorkspacePathRequired => formatter.write_str("Workspace path is required"),
            Self::CreateIdMismatch => formatter.write_str("Skill id must match metadata id"),
            Self::UpdateIdChanged => formatter.write_str("Skill id cannot be changed"),
            Self::InvalidMountPath(path) => write!(
                formatter,
                "Skill mount path must be a relative path without traversal: {path}"
            ),
            Self::UnknownAgent(agent_id) => write!(formatter, "agent not found: {agent_id}"),
            Self::InvalidUserSource(source) => {
                write!(
                    formatter,
                    "Skill source is not valid for user creation: {source}"
                )
            }
            Self::UnknownBuiltin(skill_id) => {
                write!(formatter, "Unknown built-in Skill: {skill_id}")
            }
        }
    }
}

impl std::error::Error for SkillDomainError {}
