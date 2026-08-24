/// Every way the personalization domain can refuse an input.
///
/// Kept as one enum per context convention so the application layer maps a single error type to
/// typed native results. Variants carry the offending *shape*, never the offending content: an
/// instruction body or a memory body must not travel in an error string that ends up in a log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PersonalizationDomainError {
    InvalidAgentId(IdentityRejection),
    InvalidSessionId(IdentityRejection),
    InvalidWorkspaceKey(IdentityRejection),
    UnknownScopeKind(String),
    InconsistentScopeColumns {
        kind: &'static str,
        reason: &'static str,
    },
    UnknownSessionMode(String),
    UnknownMergeMode(String),
    UnknownPolicyToggle(String),
    /// A global row stored `inherit`. There is nothing below global to inherit from, so the
    /// resolved value would silently fall through to the built-in fallback with no way for the UI
    /// to explain where it came from.
    GlobalScopeCannotInherit {
        field: &'static str,
    },
    InstructionFieldTooLong {
        field: &'static str,
        limit: usize,
        actual: usize,
    },
    InvalidMemoryId(IdentityRejection),
    MemoryFieldEmpty {
        field: &'static str,
    },
    MemoryFieldTooLong {
        field: &'static str,
        limit: usize,
        actual: usize,
    },
    /// A selected audience with no Agents means "visible to nobody", which no UI action produces
    /// and which reads as data loss rather than as a restriction.
    EmptyMemoryAudience,
    MemoryAudienceTooLarge {
        limit: usize,
        actual: usize,
    },
    /// Legacy rows migrate as explicitly untyped so their content is never guessed at or lost; a
    /// newly created record has a user in front of it and must declare a real type.
    UntypedMemoryRequiresLegacySource,
    UnknownMemoryStatus(String),
    UnknownMemorySource(String),
    UnknownMemoryType(String),
    UnknownMemorySensitivity(String),
    UnknownMemoryScopeKind(String),
    /// The address a pre-governance caller used. Distinct from a source id on purpose.
    InvalidLegacyAddressKey(IdentityRejection),
    InvalidLegacySourceId(IdentityRejection),
    InvalidLegacySourcePath(IdentityRejection),
    UnknownLegacyTableKind(String),
    UnknownMigrationStage(String),
}

/// Why an identity string was refused. Deliberately does not echo the value — an identity is not
/// secret, but reflecting arbitrary input into an error message is how a control character or a
/// separator ends up somewhere it was just rejected from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdentityRejection {
    Empty,
    NotTrimmed,
    TooLong { limit: usize },
    TooShort { limit: usize },
    ContainsSeparator,
    ContainsControlCharacter,
    UnsupportedCharacter,
}

impl std::fmt::Display for IdentityRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(formatter, "must not be empty"),
            Self::NotTrimmed => write!(formatter, "must not have leading or trailing whitespace"),
            Self::TooLong { limit } => {
                write!(formatter, "must be at most {limit} characters")
            }
            Self::TooShort { limit } => {
                write!(formatter, "must be at least {limit} characters")
            }
            Self::ContainsSeparator => write!(formatter, "must not contain a path separator"),
            Self::ContainsControlCharacter => {
                write!(formatter, "must not contain control characters")
            }
            Self::UnsupportedCharacter => write!(
                formatter,
                "must contain only letters, digits, hyphens, and underscores"
            ),
        }
    }
}

impl std::fmt::Display for PersonalizationDomainError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidAgentId(reason) => write!(formatter, "Agent id {reason}."),
            Self::InvalidSessionId(reason) => write!(formatter, "Session id {reason}."),
            Self::InvalidWorkspaceKey(reason) => write!(formatter, "Workspace key {reason}."),
            Self::UnknownScopeKind(kind) => {
                write!(formatter, "Unknown personalization scope kind \"{kind}\".")
            }
            Self::InconsistentScopeColumns { kind, reason } => write!(
                formatter,
                "Personalization scope \"{kind}\" is inconsistent: {reason}."
            ),
            Self::UnknownSessionMode(value) => {
                write!(formatter, "Unknown session personalization mode \"{value}\".")
            }
            Self::UnknownMergeMode(value) => {
                write!(formatter, "Unknown instruction merge mode \"{value}\".")
            }
            Self::UnknownPolicyToggle(value) => {
                write!(formatter, "Unknown policy toggle \"{value}\".")
            }
            Self::GlobalScopeCannotInherit { field } => write!(
                formatter,
                "The global personalization policy cannot inherit \"{field}\"; it must store a concrete value."
            ),
            Self::InstructionFieldTooLong {
                field,
                limit,
                actual,
            } => write!(
                formatter,
                "Custom instruction field \"{field}\" is {actual} characters; the limit is {limit}."
            ),
            Self::InvalidMemoryId(reason) => write!(formatter, "Memory id {reason}."),
            Self::MemoryFieldEmpty { field } => {
                write!(formatter, "Memory field \"{field}\" must not be empty.")
            }
            Self::MemoryFieldTooLong {
                field,
                limit,
                actual,
            } => write!(
                formatter,
                "Memory field \"{field}\" is {actual} characters; the limit is {limit}."
            ),
            Self::EmptyMemoryAudience => write!(
                formatter,
                "A selected Agent audience must contain at least one Agent."
            ),
            Self::MemoryAudienceTooLarge { limit, actual } => write!(
                formatter,
                "A selected Agent audience holds {actual} Agents; the limit is {limit}."
            ),
            Self::UntypedMemoryRequiresLegacySource => write!(
                formatter,
                "Only a migrated legacy memory may remain untyped; a new memory must declare a supported type."
            ),
            Self::UnknownMemoryStatus(value) => {
                write!(formatter, "Unknown memory status \"{value}\".")
            }
            Self::UnknownMemorySource(value) => {
                write!(formatter, "Unknown memory source \"{value}\".")
            }
            Self::UnknownMemoryType(value) => write!(formatter, "Unknown memory type \"{value}\"."),
            Self::UnknownMemorySensitivity(value) => {
                write!(formatter, "Unknown memory sensitivity \"{value}\".")
            }
            Self::UnknownMemoryScopeKind(value) => {
                write!(formatter, "Unknown memory scope kind \"{value}\".")
            }
            Self::InvalidLegacyAddressKey(reason) => {
                write!(formatter, "Legacy memory address {reason}.")
            }
            Self::InvalidLegacySourceId(reason) => {
                write!(formatter, "Legacy memory source id {reason}.")
            }
            Self::InvalidLegacySourcePath(reason) => {
                write!(formatter, "Legacy memory source path {reason}.")
            }
            Self::UnknownLegacyTableKind(value) => {
                write!(formatter, "Unknown legacy table kind \"{value}\".")
            }
            Self::UnknownMigrationStage(value) => {
                write!(formatter, "Unknown migration stage \"{value}\".")
            }
        }
    }
}
