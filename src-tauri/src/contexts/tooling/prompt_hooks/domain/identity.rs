use super::PromptHookDomainError;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PromptHookId(String);

impl PromptHookId {
    pub(crate) fn parse(value: impl Into<String>) -> Result<Self, PromptHookDomainError> {
        let value = value.into();
        let is_valid = (3..=64).contains(&value.len())
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            && value
                .bytes()
                .next()
                .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit());
        if is_valid {
            Ok(Self(value))
        } else {
            Err(PromptHookDomainError::InvalidId)
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookName(String);

impl PromptHookName {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, PromptHookDomainError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            Err(PromptHookDomainError::NameRequired)
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_preserves_the_existing_length_and_character_contract() {
        for value in ["abc", "hook-2", "abc-", "a--b"] {
            assert_eq!(
                PromptHookId::parse(value).expect("valid id").as_str(),
                value
            );
        }
        for value in ["", "ab", "-abc", "ABC", "a_b", "a b"] {
            assert_eq!(
                PromptHookId::parse(value),
                Err(PromptHookDomainError::InvalidId)
            );
        }
        assert_eq!(
            PromptHookId::parse("a".repeat(65)),
            Err(PromptHookDomainError::InvalidId)
        );
    }

    #[test]
    fn name_is_required_and_normalized_at_the_domain_boundary() {
        assert_eq!(
            PromptHookName::new(" \t "),
            Err(PromptHookDomainError::NameRequired)
        );
        assert_eq!(
            PromptHookName::new("  Review focus  ")
                .expect("name")
                .as_str(),
            "Review focus"
        );
    }
}
