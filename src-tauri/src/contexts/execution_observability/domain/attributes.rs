use super::ExecutionDomainError;
use std::collections::BTreeMap;

const MAX_ATTRIBUTE_COUNT: usize = 32;
const MAX_ATTRIBUTE_KEY_LENGTH: usize = 128;
const MAX_ATTRIBUTE_VALUE_LENGTH: usize = 256;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SafeAttributeValue {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

impl SafeAttributeValue {
    pub(crate) fn bounded_string(value: impl Into<String>) -> Result<Self, ExecutionDomainError> {
        let value = value.into();
        if value.chars().count() > MAX_ATTRIBUTE_VALUE_LENGTH {
            return Err(ExecutionDomainError::AttributeValueTooLong {
                max: MAX_ATTRIBUTE_VALUE_LENGTH,
            });
        }
        Ok(Self::String(value))
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct SafeAttributes(BTreeMap<String, SafeAttributeValue>);

impl SafeAttributes {
    pub(crate) fn try_from_entries(
        entries: impl IntoIterator<Item = (String, SafeAttributeValue)>,
    ) -> Result<Self, ExecutionDomainError> {
        let entries = entries.into_iter().collect::<BTreeMap<_, _>>();
        if entries.len() > MAX_ATTRIBUTE_COUNT {
            return Err(ExecutionDomainError::TooManyAttributes {
                max: MAX_ATTRIBUTE_COUNT,
            });
        }
        if entries
            .keys()
            .any(|key| key.is_empty() || key.chars().count() > MAX_ATTRIBUTE_KEY_LENGTH)
        {
            return Err(ExecutionDomainError::InvalidAttributeKey {
                max: MAX_ATTRIBUTE_KEY_LENGTH,
            });
        }
        if entries.values().any(|value| {
            matches!(value, SafeAttributeValue::String(value) if value.chars().count() > MAX_ATTRIBUTE_VALUE_LENGTH)
        }) {
            return Err(ExecutionDomainError::AttributeValueTooLong {
                max: MAX_ATTRIBUTE_VALUE_LENGTH,
            });
        }
        Ok(Self(entries))
    }

    pub(crate) fn entries(&self) -> &BTreeMap<String, SafeAttributeValue> {
        &self.0
    }
}
