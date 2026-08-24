use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Inheritance is a first-class state rather than a magic provider value, so a provider that
/// genuinely accepts the literal string `default` stays representable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub(crate) enum CliParameterSelection {
    Inherit,
    Value {
        #[serde(rename = "value")]
        value: CliParameterValue,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum CliParameterValue {
    Boolean(bool),
    Text(String),
    TextList(Vec<String>),
}

impl CliParameterSelection {
    pub(crate) fn value(value: CliParameterValue) -> Self {
        Self::Value { value }
    }

    pub(crate) fn text(value: impl Into<String>) -> Self {
        Self::value(CliParameterValue::Text(value.into()))
    }

    pub(crate) fn boolean(value: bool) -> Self {
        Self::value(CliParameterValue::Boolean(value))
    }

    pub(crate) fn text_list(values: Vec<String>) -> Self {
        Self::value(CliParameterValue::TextList(values))
    }

    pub(crate) fn is_inherit(&self) -> bool {
        matches!(self, Self::Inherit)
    }

    pub(crate) fn as_value(&self) -> Option<&CliParameterValue> {
        match self {
            Self::Inherit => None,
            Self::Value { value } => Some(value),
        }
    }

    #[cfg(test)]
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Inherit => "inherit",
            Self::Value { value } => value.kind(),
        }
    }
}

impl CliParameterValue {
    #[cfg(test)]
    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::Boolean(_) => "boolean",
            Self::Text(_) => "text",
            Self::TextList(_) => "text-list",
        }
    }

    pub(crate) fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    pub(crate) fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            _ => None,
        }
    }

    pub(crate) fn as_text_list(&self) -> Option<&[String]> {
        match self {
            Self::TextList(values) => Some(values),
            _ => None,
        }
    }
}

pub(crate) type CliParameterSelectionMap = BTreeMap<String, CliParameterSelection>;

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(selection: &CliParameterSelection) -> CliParameterSelection {
        let encoded = serde_json::to_string(selection).expect("encode");
        serde_json::from_str(&encoded).expect("decode")
    }

    #[test]
    fn inherit_is_distinct_from_every_provider_value() {
        let inherit = CliParameterSelection::Inherit;
        let literal_default = CliParameterSelection::text("default");
        assert_ne!(inherit, literal_default);
        assert!(inherit.is_inherit());
        assert!(!literal_default.is_inherit());
        assert_eq!(
            literal_default
                .as_value()
                .and_then(CliParameterValue::as_text),
            Some("default")
        );
    }

    #[test]
    fn inherit_serializes_without_a_value_field() {
        let encoded = serde_json::to_string(&CliParameterSelection::Inherit).expect("encode");
        assert_eq!(encoded, r#"{"state":"inherit"}"#);
        assert_eq!(
            round_trip(&CliParameterSelection::Inherit),
            CliParameterSelection::Inherit
        );
    }

    #[test]
    fn every_value_kind_round_trips() {
        for selection in [
            CliParameterSelection::text("sonnet"),
            CliParameterSelection::text("default"),
            CliParameterSelection::boolean(true),
            CliParameterSelection::boolean(false),
            CliParameterSelection::text_list(vec!["a".to_string(), "b".to_string()]),
        ] {
            assert_eq!(round_trip(&selection), selection);
        }
    }

    #[test]
    fn explicit_false_is_not_inheritance() {
        let explicit_false = CliParameterSelection::boolean(false);
        assert!(!explicit_false.is_inherit());
        assert_eq!(
            explicit_false
                .as_value()
                .and_then(CliParameterValue::as_bool),
            Some(false)
        );
        assert_eq!(round_trip(&explicit_false), explicit_false);
    }

    #[test]
    fn value_selection_serializes_with_a_tagged_state() {
        let encoded = serde_json::to_string(&CliParameterSelection::text("high")).expect("encode");
        assert_eq!(encoded, r#"{"state":"value","value":"high"}"#);
        let list = CliParameterSelection::text_list(vec!["x".to_string()]);
        assert_eq!(
            serde_json::to_string(&list).expect("encode"),
            r#"{"state":"value","value":["x"]}"#
        );
    }

    #[test]
    fn a_malformed_envelope_is_rejected_instead_of_defaulting() {
        for raw in [
            r#"{"state":"unknown"}"#,
            r#"{"state":"value"}"#,
            r#""sonnet""#,
            r#"{"value":"sonnet"}"#,
        ] {
            assert!(
                serde_json::from_str::<CliParameterSelection>(raw).is_err(),
                "expected {raw} to be rejected"
            );
        }
    }

    #[test]
    fn value_kinds_are_reported_for_structured_type_errors() {
        assert_eq!(CliParameterSelection::Inherit.kind(), "inherit");
        assert_eq!(CliParameterSelection::text("a").kind(), "text");
        assert_eq!(CliParameterSelection::boolean(true).kind(), "boolean");
        assert_eq!(
            CliParameterSelection::text_list(Vec::new()).kind(),
            "text-list"
        );
    }
}
