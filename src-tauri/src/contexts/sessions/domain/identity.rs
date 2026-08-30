use super::SessionsDomainError;

fn validate_identity(value: String, kind: &'static str) -> Result<String, SessionsDomainError> {
    if value.trim().is_empty() {
        return Err(SessionsDomainError::IdentityRequired(kind));
    }
    if value.chars().any(char::is_control) {
        return Err(SessionsDomainError::IdentityContainsControl(kind));
    }
    Ok(value)
}

macro_rules! identity_type {
    ($name:ident, $kind:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub(crate) struct $name(String);

        impl $name {
            pub(crate) fn parse(value: impl Into<String>) -> Result<Self, SessionsDomainError> {
                validate_identity(value.into(), $kind).map(Self)
            }

            pub(crate) fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

identity_type!(MessageId, "Message id");
identity_type!(CategoryId, "Category id");

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct SessionId(String);

impl SessionId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, SessionsDomainError> {
        let value = validate_identity(value.into(), "Session id")?;
        // System activity sessions are projections, not Agent sessions: no interactive command —
        // create, rename, pin, archive, categorize, delete, send, stop, terminal, provider
        // resume, or chat configuration — may ever address one, so the id is refused at the
        // domain boundary instead of at each command.
        if value.starts_with("system-activity-v1-") {
            return Err(SessionsDomainError::SystemActivitySessionRefused);
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identities_require_stable_non_control_values() {
        assert_eq!(
            SessionId::parse("session-1").expect("session id").as_str(),
            "session-1"
        );
        assert_eq!(
            MessageId::parse("  "),
            Err(SessionsDomainError::IdentityRequired("Message id"))
        );
        assert_eq!(
            CategoryId::parse("category\n1"),
            Err(SessionsDomainError::IdentityContainsControl("Category id"))
        );
    }

    #[test]
    fn system_activity_session_ids_are_refused_by_every_interactive_command_path() {
        assert_eq!(
            SessionId::parse("system-activity-v1-abcdef"),
            Err(SessionsDomainError::SystemActivitySessionRefused)
        );
        // The literal here must stay in step with the system activity context's namespace; the
        // domain layer cannot import another context, so the agreement is asserted instead.
        assert!(
            crate::contexts::skill_evolution_system_activity::api::is_system_activity_session_id(
                "system-activity-v1-abcdef"
            )
        );
    }
}
