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

identity_type!(SessionId, "Session id");
identity_type!(MessageId, "Message id");
identity_type!(CategoryId, "Category id");

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
}
