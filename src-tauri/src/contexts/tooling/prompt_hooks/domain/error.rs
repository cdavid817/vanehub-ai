use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PromptHookDomainError {
    InvalidId,
    NameRequired,
    NegativeOrder,
    UnsupportedControlCharacter,
    UnsupportedAgent(String),
    DuplicateOrder,
    IdentityChanged,
    BuiltinContentImmutable,
    BuiltinCannotBeDeleted,
    CannotBeDisabled,
}

impl fmt::Display for PromptHookDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId => formatter.write_str("Invalid Prompt Hook id."),
            Self::NameRequired => formatter.write_str("Prompt Hook name is required."),
            Self::NegativeOrder => formatter.write_str("Prompt Hook order must be non-negative."),
            Self::UnsupportedControlCharacter => {
                formatter.write_str("Prompt Hook content contains unsupported control characters.")
            }
            Self::UnsupportedAgent(agent_id) => {
                write!(formatter, "Unsupported Prompt Hook CLI binding: {agent_id}")
            }
            Self::DuplicateOrder => formatter
                .write_str("Prompt Hook order is already in use for this stage and category."),
            Self::IdentityChanged => formatter.write_str("Prompt Hook id cannot be changed."),
            Self::BuiltinContentImmutable => {
                formatter.write_str("Built-in Prompt Hook content cannot be edited.")
            }
            Self::BuiltinCannotBeDeleted => {
                formatter.write_str("Built-in Prompt Hook cannot be deleted.")
            }
            Self::CannotBeDisabled => formatter.write_str("Prompt Hook cannot be disabled."),
        }
    }
}

impl std::error::Error for PromptHookDomainError {}
