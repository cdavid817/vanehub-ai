#![cfg_attr(not(test), allow(dead_code))]

use super::{LEARNED_GUIDANCE_END_MARKER, LEARNED_GUIDANCE_HEADING, LEARNED_GUIDANCE_START_MARKER};

pub(crate) const OVERLAY_TEXT_SCANNER_VERSION: &str = "overlay-text-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayTextRuleId {
    PrivateKeyMaterial,
    CredentialStructure,
    PromptAuthorityOverride,
    ScriptMarkup,
    GuidanceDelimiterForgery,
}

impl OverlayTextRuleId {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::PrivateKeyMaterial => "overlay.private-key-material",
            Self::CredentialStructure => "overlay.credential-structure",
            Self::PromptAuthorityOverride => "overlay.prompt-authority-override",
            Self::ScriptMarkup => "overlay.script-markup",
            Self::GuidanceDelimiterForgery => "overlay.guidance-delimiter-forgery",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayTextScan {
    scanner_version: &'static str,
    rule_ids: Vec<OverlayTextRuleId>,
}

impl OverlayTextScan {
    pub(crate) fn passed(&self) -> bool {
        self.rule_ids.is_empty()
    }

    pub(crate) fn scanner_version(&self) -> &str {
        self.scanner_version
    }

    pub(crate) fn rule_ids(&self) -> &[OverlayTextRuleId] {
        &self.rule_ids
    }

    pub(crate) fn safe_rule_ids(&self) -> Vec<&'static str> {
        self.rule_ids.iter().map(|rule| rule.as_str()).collect()
    }
}

pub(crate) fn scan_overlay_text(value: &str) -> OverlayTextScan {
    let lowercase = value.to_lowercase();
    let mut rule_ids = Vec::new();
    push_when(
        &mut rule_ids,
        contains_private_key(&lowercase),
        OverlayTextRuleId::PrivateKeyMaterial,
    );
    push_when(
        &mut rule_ids,
        contains_credential_structure(value, &lowercase),
        OverlayTextRuleId::CredentialStructure,
    );
    push_when(
        &mut rule_ids,
        contains_prompt_override(&lowercase),
        OverlayTextRuleId::PromptAuthorityOverride,
    );
    push_when(
        &mut rule_ids,
        contains_script_markup(&lowercase),
        OverlayTextRuleId::ScriptMarkup,
    );
    push_when(
        &mut rule_ids,
        contains_guidance_delimiter(value),
        OverlayTextRuleId::GuidanceDelimiterForgery,
    );
    OverlayTextScan {
        scanner_version: OVERLAY_TEXT_SCANNER_VERSION,
        rule_ids,
    }
}

fn push_when(rule_ids: &mut Vec<OverlayTextRuleId>, matched: bool, rule: OverlayTextRuleId) {
    if matched {
        rule_ids.push(rule);
    }
}

fn contains_private_key(lowercase: &str) -> bool {
    lowercase
        .lines()
        .any(|line| line.trim_start().starts_with("-----begin") && line.contains("private key"))
}

fn contains_credential_structure(original: &str, lowercase: &str) -> bool {
    const LABELS: &[&str] = &[
        "api_key",
        "api-key",
        "apikey",
        "access_token",
        "access-token",
        "client_secret",
        "client-secret",
        "password",
    ];
    LABELS
        .iter()
        .any(|label| contains_assigned_secret(lowercase, label))
        || original
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .any(is_known_token_structure)
}

fn contains_assigned_secret(value: &str, label: &str) -> bool {
    value.match_indices(label).any(|(index, _)| {
        let remainder = &value[index + label.len()..];
        let trimmed = remainder
            .trim_start()
            .trim_start_matches(['\'', '"'])
            .trim_start();
        let Some(separator) = trimmed.chars().next() else {
            return false;
        };
        if separator != ':' && separator != '=' {
            return false;
        }
        let candidate = trimmed[separator.len_utf8()..]
            .trim_start()
            .trim_start_matches(['\'', '"']);
        candidate
            .chars()
            .take_while(|character| is_secret_character(*character))
            .count()
            >= 8
    })
}

fn is_secret_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '/' | '+' | '=')
}

fn is_known_token_structure(token: &str) -> bool {
    (token.len() == 20
        && token.starts_with("AKIA")
        && token
            .chars()
            .all(|character| character.is_ascii_uppercase() || character.is_ascii_digit()))
        || (["ghp_", "gho_", "ghu_", "ghs_", "ghr_"]
            .iter()
            .any(|prefix| token.starts_with(prefix))
            && token.len() >= 20
            && token
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '_'))
}

fn contains_prompt_override(lowercase: &str) -> bool {
    [
        "ignore previous instructions",
        "ignore all previous instructions",
        "disregard prior instructions",
        "disregard all prior instructions",
        "override the system message",
        "system message override",
    ]
    .iter()
    .any(|phrase| lowercase.contains(phrase))
}

fn contains_script_markup(lowercase: &str) -> bool {
    let mut remainder = lowercase;
    while let Some(index) = remainder.find('<') {
        remainder = &remainder[index + 1..];
        let candidate = remainder.trim_start();
        if ["script", "iframe", "object", "embed"]
            .iter()
            .any(|tag| starts_with_tag(candidate, tag))
        {
            return true;
        }
        if remainder.is_empty() {
            break;
        }
    }
    lowercase.contains("javascript:")
}

fn starts_with_tag(value: &str, tag: &str) -> bool {
    value.strip_prefix(tag).is_some_and(|remainder| {
        remainder
            .chars()
            .next()
            .is_none_or(|character| character.is_whitespace() || matches!(character, '>' | '/'))
    })
}

fn contains_guidance_delimiter(value: &str) -> bool {
    value.contains(LEARNED_GUIDANCE_START_MARKER)
        || value.contains(LEARNED_GUIDANCE_END_MARKER)
        || value.contains(LEARNED_GUIDANCE_HEADING)
}
