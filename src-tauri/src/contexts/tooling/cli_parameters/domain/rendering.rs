use super::selection::CliParameterValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CliArgumentSlot {
    /// Tokens that must precede a provider subcommand such as `codex exec` or `opencode run`.
    Global,
    /// Tokens owned by the interactive/fresh-chat/resume invocation grammar.
    Invocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliConfigEncoding {
    TomlString,
    TomlBoolean,
}

/// Render strategies are declarative so the runtime never branches on a parameter id.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) enum CliParameterRenderer {
    #[serde(rename_all = "camelCase")]
    PresenceFlag { flag: String, slot: CliArgumentSlot },
    #[serde(rename_all = "camelCase")]
    PositiveNegativeFlag {
        positive_flag: String,
        negative_flag: String,
        slot: CliArgumentSlot,
    },
    #[serde(rename_all = "camelCase")]
    FlagValue { flag: String, slot: CliArgumentSlot },
    #[serde(rename_all = "camelCase")]
    RepeatFlagValue { flag: String, slot: CliArgumentSlot },
    #[serde(rename_all = "camelCase")]
    JoinedList {
        flag: String,
        separator: String,
        slot: CliArgumentSlot,
    },
    #[serde(rename_all = "camelCase")]
    ConfigKeyValue {
        flag: String,
        key: String,
        encoding: CliConfigEncoding,
        slot: CliArgumentSlot,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliArgumentToken {
    pub(crate) value: String,
    pub(crate) parameter_id: String,
    pub(crate) segment: CliArgumentSlot,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliArgumentSegments {
    pub(crate) global: Vec<CliArgumentToken>,
    pub(crate) invocation: Vec<CliArgumentToken>,
}

impl CliArgumentSegments {
    pub(crate) fn push(&mut self, token: CliArgumentToken) {
        match token.segment {
            CliArgumentSlot::Global => self.global.push(token),
            CliArgumentSlot::Invocation => self.invocation.push(token),
        }
    }

    pub(crate) fn global_values(&self) -> Vec<String> {
        self.global
            .iter()
            .map(|token| token.value.clone())
            .collect()
    }

    pub(crate) fn invocation_values(&self) -> Vec<String> {
        self.invocation
            .iter()
            .map(|token| token.value.clone())
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.global.is_empty() && self.invocation.is_empty()
    }
}

/// TOML basic-string escaping. Validation already rejects control characters, so only the two
/// structural characters can appear, but the escape stays complete rather than assuming that.
fn toml_basic_string(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len() + 2);
    encoded.push('"');
    for character in value.chars() {
        match character {
            '"' => encoded.push_str("\\\""),
            '\\' => encoded.push_str("\\\\"),
            '\u{8}' => encoded.push_str("\\b"),
            '\t' => encoded.push_str("\\t"),
            '\n' => encoded.push_str("\\n"),
            '\u{c}' => encoded.push_str("\\f"),
            '\r' => encoded.push_str("\\r"),
            other if (other as u32) < 0x20 || other as u32 == 0x7f => {
                encoded.push_str(&format!("\\u{:04X}", other as u32));
            }
            other => encoded.push(other),
        }
    }
    encoded.push('"');
    encoded
}

impl CliParameterRenderer {
    pub(crate) fn slot(&self) -> CliArgumentSlot {
        match self {
            Self::PresenceFlag { slot, .. }
            | Self::PositiveNegativeFlag { slot, .. }
            | Self::FlagValue { slot, .. }
            | Self::RepeatFlagValue { slot, .. }
            | Self::JoinedList { slot, .. }
            | Self::ConfigKeyValue { slot, .. } => *slot,
        }
    }

    /// Every literal flag this renderer can emit, used by registry safety validation.
    pub(crate) fn flags(&self) -> Vec<&str> {
        match self {
            Self::PresenceFlag { flag, .. }
            | Self::FlagValue { flag, .. }
            | Self::RepeatFlagValue { flag, .. }
            | Self::JoinedList { flag, .. }
            | Self::ConfigKeyValue { flag, .. } => vec![flag.as_str()],
            Self::PositiveNegativeFlag {
                positive_flag,
                negative_flag,
                ..
            } => vec![positive_flag.as_str(), negative_flag.as_str()],
        }
    }

    pub(crate) fn supports_explicit_false(&self) -> bool {
        matches!(
            self,
            Self::PositiveNegativeFlag { .. } | Self::ConfigKeyValue { .. }
        )
    }

    pub(crate) fn accepts(&self, value: &CliParameterValue) -> bool {
        match self {
            Self::PresenceFlag { .. } | Self::PositiveNegativeFlag { .. } => {
                matches!(value, CliParameterValue::Boolean(_))
            }
            Self::FlagValue { .. } => matches!(value, CliParameterValue::Text(_)),
            Self::RepeatFlagValue { .. } | Self::JoinedList { .. } => {
                matches!(value, CliParameterValue::TextList(_))
            }
            Self::ConfigKeyValue { encoding, .. } => match encoding {
                CliConfigEncoding::TomlString => matches!(value, CliParameterValue::Text(_)),
                CliConfigEncoding::TomlBoolean => matches!(value, CliParameterValue::Boolean(_)),
            },
        }
    }

    pub(crate) fn render(
        &self,
        parameter_id: &str,
        value: &CliParameterValue,
    ) -> Vec<CliArgumentToken> {
        let slot = self.slot();
        let token = |value: String| CliArgumentToken {
            value,
            parameter_id: parameter_id.to_string(),
            segment: slot,
        };
        match self {
            Self::PresenceFlag { flag, .. } => match value.as_bool() {
                Some(true) => vec![token(flag.clone())],
                _ => Vec::new(),
            },
            Self::PositiveNegativeFlag {
                positive_flag,
                negative_flag,
                ..
            } => match value.as_bool() {
                Some(true) => vec![token(positive_flag.clone())],
                Some(false) => vec![token(negative_flag.clone())],
                None => Vec::new(),
            },
            Self::FlagValue { flag, .. } => match value.as_text() {
                Some(text) => vec![token(flag.clone()), token(text.to_string())],
                None => Vec::new(),
            },
            Self::RepeatFlagValue { flag, .. } => value
                .as_text_list()
                .into_iter()
                .flatten()
                .flat_map(|entry| vec![token(flag.clone()), token(entry.clone())])
                .collect(),
            Self::JoinedList {
                flag, separator, ..
            } => match value.as_text_list() {
                Some(entries) if !entries.is_empty() => {
                    vec![token(flag.clone()), token(entries.join(separator))]
                }
                _ => Vec::new(),
            },
            Self::ConfigKeyValue {
                flag,
                key,
                encoding,
                ..
            } => {
                let encoded = match encoding {
                    CliConfigEncoding::TomlString => value.as_text().map(toml_basic_string),
                    CliConfigEncoding::TomlBoolean => value.as_bool().map(|entry| {
                        if entry {
                            "true".to_string()
                        } else {
                            "false".to_string()
                        }
                    }),
                };
                match encoded {
                    Some(encoded) => vec![token(flag.clone()), token(format!("{key}={encoded}"))],
                    None => Vec::new(),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn values(tokens: &[CliArgumentToken]) -> Vec<&str> {
        tokens.iter().map(|token| token.value.as_str()).collect()
    }

    #[test]
    fn presence_flag_emits_nothing_for_explicit_false() {
        let renderer = CliParameterRenderer::PresenceFlag {
            flag: "--search".to_string(),
            slot: CliArgumentSlot::Global,
        };
        assert_eq!(
            values(&renderer.render("search", &CliParameterValue::Boolean(true))),
            ["--search"]
        );
        assert!(renderer
            .render("search", &CliParameterValue::Boolean(false))
            .is_empty());
        assert!(!renderer.supports_explicit_false());
    }

    #[test]
    fn tri_state_emits_at_most_one_of_the_mutually_exclusive_flags() {
        let renderer = CliParameterRenderer::PositiveNegativeFlag {
            positive_flag: "--chrome".to_string(),
            negative_flag: "--no-chrome".to_string(),
            slot: CliArgumentSlot::Global,
        };
        assert_eq!(
            values(&renderer.render("chrome", &CliParameterValue::Boolean(true))),
            ["--chrome"]
        );
        assert_eq!(
            values(&renderer.render("chrome", &CliParameterValue::Boolean(false))),
            ["--no-chrome"]
        );
        assert!(renderer.supports_explicit_false());
        assert_eq!(renderer.flags(), ["--chrome", "--no-chrome"]);
    }

    #[test]
    fn flag_value_keeps_whitespace_inside_one_token() {
        let renderer = CliParameterRenderer::FlagValue {
            flag: "--model".to_string(),
            slot: CliArgumentSlot::Global,
        };
        let tokens = renderer.render(
            "model",
            &CliParameterValue::Text("my model name".to_string()),
        );
        assert_eq!(values(&tokens), ["--model", "my model name"]);
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn literal_default_is_rendered_like_any_other_provider_value() {
        let renderer = CliParameterRenderer::FlagValue {
            flag: "--approval-mode".to_string(),
            slot: CliArgumentSlot::Global,
        };
        assert_eq!(
            values(&renderer.render(
                "approvalMode",
                &CliParameterValue::Text("default".to_string())
            )),
            ["--approval-mode", "default"]
        );
    }

    #[test]
    fn repeat_and_joined_list_strategies_differ() {
        let repeated = CliParameterRenderer::RepeatFlagValue {
            flag: "--extensions".to_string(),
            slot: CliArgumentSlot::Global,
        };
        let joined = CliParameterRenderer::JoinedList {
            flag: "--fallback-model".to_string(),
            separator: ",".to_string(),
            slot: CliArgumentSlot::Global,
        };
        let list = CliParameterValue::TextList(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(
            values(&repeated.render("extensions", &list)),
            ["--extensions", "a", "--extensions", "b"]
        );
        assert_eq!(
            values(&joined.render("fallbackModels", &list)),
            ["--fallback-model", "a,b"]
        );
        let empty = CliParameterValue::TextList(Vec::new());
        assert!(repeated.render("extensions", &empty).is_empty());
        assert!(joined.render("fallbackModels", &empty).is_empty());
    }

    #[test]
    fn config_key_value_emits_two_tokens_without_shell_quoting() {
        let renderer = CliParameterRenderer::ConfigKeyValue {
            flag: "--config".to_string(),
            key: "model_reasoning_effort".to_string(),
            encoding: CliConfigEncoding::TomlString,
            slot: CliArgumentSlot::Global,
        };
        let tokens = renderer.render(
            "reasoningEffort",
            &CliParameterValue::Text("high".to_string()),
        );
        assert_eq!(
            values(&tokens),
            ["--config", "model_reasoning_effort=\"high\""]
        );
    }

    #[test]
    fn toml_encoding_escapes_structural_characters() {
        assert_eq!(toml_basic_string(r#"a"b"#), r#""a\"b""#);
        assert_eq!(toml_basic_string(r"a\b"), r#""a\\b""#);
        assert_eq!(toml_basic_string("a\nb"), r#""a\nb""#);
    }

    #[test]
    fn renderers_reject_a_mismatched_value_kind() {
        let flag_value = CliParameterRenderer::FlagValue {
            flag: "--model".to_string(),
            slot: CliArgumentSlot::Global,
        };
        assert!(flag_value.accepts(&CliParameterValue::Text("x".to_string())));
        assert!(!flag_value.accepts(&CliParameterValue::Boolean(true)));
        assert!(flag_value
            .render("model", &CliParameterValue::Boolean(true))
            .is_empty());
    }

    #[test]
    fn segments_keep_slot_ownership() {
        let mut segments = CliArgumentSegments::default();
        segments.push(CliArgumentToken {
            value: "--oss".to_string(),
            parameter_id: "oss".to_string(),
            segment: CliArgumentSlot::Global,
        });
        segments.push(CliArgumentToken {
            value: "--ephemeral".to_string(),
            parameter_id: "ephemeral".to_string(),
            segment: CliArgumentSlot::Invocation,
        });
        assert_eq!(segments.global_values(), ["--oss"]);
        assert_eq!(segments.invocation_values(), ["--ephemeral"]);
        assert!(!segments.is_empty());
    }
}
