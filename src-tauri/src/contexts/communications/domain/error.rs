use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommunicationsDomainError {
    RequiredValue(&'static str),
    ControlCharacters(&'static str),
    SensitivePublicConfigField(String),
    InvalidConnectorTransition {
        from: &'static str,
        to: &'static str,
    },
    InvalidAuthorizationDeadline,
    InvalidAuthorizationTransition {
        from: &'static str,
        to: &'static str,
    },
    AuthorizationErrorCodeRequired,
}

impl fmt::Display for CommunicationsDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequiredValue(kind) => write!(formatter, "{kind} cannot be empty."),
            Self::ControlCharacters(kind) => {
                write!(formatter, "{kind} contains invalid control characters.")
            }
            Self::SensitivePublicConfigField(field) => write!(
                formatter,
                "sensitive connector field is not allowed in public config: {field}"
            ),
            Self::InvalidConnectorTransition { from, to } => {
                write!(
                    formatter,
                    "Connector cannot transition from {from} to {to}."
                )
            }
            Self::InvalidAuthorizationDeadline => {
                formatter.write_str("Authorization expiry must be after its start time.")
            }
            Self::InvalidAuthorizationTransition { from, to } => {
                write!(
                    formatter,
                    "Authorization cannot transition from {from} to {to}."
                )
            }
            Self::AuthorizationErrorCodeRequired => {
                formatter.write_str("Authorization failures require a safe error code.")
            }
        }
    }
}

impl std::error::Error for CommunicationsDomainError {}
