use super::EvidenceDomainError;

pub(crate) const MAX_IDENTIFIER_LENGTH: usize = 128;
pub(crate) const MAX_REASON_CODE_LENGTH: usize = 64;
pub(crate) const MAX_LABEL_LENGTH: usize = 128;
pub(crate) const MAX_BASENAME_LENGTH: usize = 255;
pub(crate) const MAX_FINGERPRINT_LENGTH: usize = 64;

/// A bounded opaque identifier that some other context owns.
///
/// Evidence never interprets these: they exist so a reader can get from a record back to the
/// canonical store that owns the thing. Validation is therefore about bounds and shape, not about
/// whether the referent exists — asserting existence here would make evidence capture depend on
/// the availability of the context it is describing.
macro_rules! bounded_id {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn parse(value: impl Into<String>) -> Result<Self, EvidenceDomainError> {
                let value = value.into();
                if value.is_empty()
                    || value.chars().count() > MAX_IDENTIFIER_LENGTH
                    || value.chars().any(|character| character.is_control())
                {
                    return Err(EvidenceDomainError::InvalidIdentifier {
                        field: $field,
                        max: MAX_IDENTIFIER_LENGTH,
                    });
                }
                Ok(Self(value))
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

bounded_id!(EvidenceEventId, "event id");
bounded_id!(SourceEventId, "source event id");
bounded_id!(EvidenceSessionId, "session id");
bounded_id!(EvidenceSeatId, "seat id");
bounded_id!(EvidenceAgentId, "agent id");
bounded_id!(EvidenceOperationId, "operation id");
bounded_id!(EvidenceCommandId, "command id");
bounded_id!(EvidenceToolCallId, "tool call id");
bounded_id!(EvidenceFileMutationId, "file mutation id");

/// Which context produced an event. Closed rather than a free string, because the pair
/// `(source_context, source_event_id)` is the idempotency key: an open set would let a typo
/// create a second identity for the same producer and defeat de-duplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum EvidenceSourceContext {
    AgentRuntime,
    Workspaces,
    Operations,
    Sessions,
    Review,
    ExecutionObservability,
}

impl EvidenceSourceContext {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::AgentRuntime => "agent_runtime",
            Self::Workspaces => "workspaces",
            Self::Operations => "operations",
            Self::Sessions => "sessions",
            Self::Review => "review",
            Self::ExecutionObservability => "execution_observability",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "agent_runtime" => Some(Self::AgentRuntime),
            "workspaces" => Some(Self::Workspaces),
            "operations" => Some(Self::Operations),
            "sessions" => Some(Self::Sessions),
            "review" => Some(Self::Review),
            "execution_observability" => Some(Self::ExecutionObservability),
            _ => None,
        }
    }
}

/// A stable, machine-readable explanation. Constrained to `[a-z0-9_]` so it can be used as a
/// localization key and can never smuggle a message, a path, or a value into a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct SafeReasonCode(String);

impl SafeReasonCode {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, EvidenceDomainError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_REASON_CODE_LENGTH
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        if !valid {
            return Err(EvidenceDomainError::InvalidReasonCode {
                max: MAX_REASON_CODE_LENGTH,
            });
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A short human-readable label such as a tool or verification name. Control characters are
/// refused because a label reaches a log line and a UI row, and a newline there is the start of a
/// terminal transcript rather than a name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct BoundedLabel(String);

impl BoundedLabel {
    pub(crate) fn parse(
        field: &'static str,
        value: impl Into<String>,
    ) -> Result<Self, EvidenceDomainError> {
        let value = value.into();
        if value.is_empty()
            || value.chars().count() > MAX_LABEL_LENGTH
            || value.chars().any(|character| character.is_control())
        {
            return Err(EvidenceDomainError::InvalidLabel {
                field,
                max: MAX_LABEL_LENGTH,
            });
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

/// A content-addressed stand-in for something evidence must not store. Hex so it can never be
/// mistaken for, or decoded back into, the value it replaces.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct SafeFingerprint(String);

impl SafeFingerprint {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, EvidenceDomainError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.len() <= MAX_FINGERPRINT_LENGTH
            && value.bytes().all(|byte| byte.is_ascii_hexdigit());
        if !valid {
            return Err(EvidenceDomainError::InvalidIdentifier {
                field: "fingerprint",
                max: MAX_FINGERPRINT_LENGTH,
            });
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}
