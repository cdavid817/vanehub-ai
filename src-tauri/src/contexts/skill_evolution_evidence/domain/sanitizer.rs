use std::collections::BTreeMap;

use hmac::{Hmac, KeyInit, Mac};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use thiserror::Error;
use zeroize::Zeroizing;

pub(crate) const EVIDENCE_SANITIZER_V1: u16 = 1;
pub(crate) const MAX_SANITIZER_INPUT_CHARS: usize = 1_000;
const MIN_INSTALLATION_KEY_BYTES: usize = 32;
const MARKER_TOKEN_BYTES: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RedactionClass {
    PrivateKey,
    Token,
    Authorization,
    CredentialAssignment,
    CredentialUrl,
    ConnectionString,
    SecretEnvironment,
    UserHomePath,
    Email,
    Phone,
    NetworkIdentifier,
    CloudAccount,
}

impl RedactionClass {
    pub(crate) const ALL: [Self; 12] = [
        Self::PrivateKey,
        Self::Token,
        Self::Authorization,
        Self::CredentialAssignment,
        Self::CredentialUrl,
        Self::ConnectionString,
        Self::SecretEnvironment,
        Self::UserHomePath,
        Self::Email,
        Self::Phone,
        Self::NetworkIdentifier,
        Self::CloudAccount,
    ];

    fn as_str(self) -> &'static str {
        match self {
            Self::PrivateKey => "private-key",
            Self::Token => "token",
            Self::Authorization => "authorization",
            Self::CredentialAssignment => "credential",
            Self::CredentialUrl => "credential-url",
            Self::ConnectionString => "connection-string",
            Self::SecretEnvironment => "secret-env",
            Self::UserHomePath => "user-home",
            Self::Email => "email",
            Self::Phone => "phone",
            Self::NetworkIdentifier => "network",
            Self::CloudAccount => "cloud-account",
        }
    }

    pub(crate) fn marker_prefix(self) -> String {
        format!("<redacted:{}:", self.as_str())
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum SanitizationError {
    #[error("installation-scoped evidence key is too short")]
    WeakInstallationKey,
    #[error("evidence sanitizer input exceeds {max} characters")]
    InputTooLong { max: usize },
    #[error("evidence sanitizer registry is unavailable")]
    RegistryUnavailable,
    #[error("evidence sanitizer marker derivation failed")]
    MarkerDerivationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SanitizationResult {
    text: String,
    class_counts: BTreeMap<RedactionClass, u32>,
    sanitizer_version: u16,
}

impl SanitizationResult {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn count(&self, class: RedactionClass) -> u32 {
        self.class_counts.get(&class).copied().unwrap_or(0)
    }

    pub(crate) fn total_redactions(&self) -> u32 {
        self.class_counts.values().sum()
    }

    pub(crate) fn sanitizer_version(&self) -> u16 {
        self.sanitizer_version
    }
}

pub(crate) struct EvidenceSanitizer {
    marker_key: Zeroizing<Vec<u8>>,
    rules: Vec<RedactionRule>,
}

impl EvidenceSanitizer {
    pub(crate) fn new(installation_key: &[u8]) -> Result<Self, SanitizationError> {
        if installation_key.len() < MIN_INSTALLATION_KEY_BYTES {
            return Err(SanitizationError::WeakInstallationKey);
        }
        let rules = RULE_SPECS
            .iter()
            .map(|(class, pattern)| RedactionRule::compile(*class, pattern))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            marker_key: Zeroizing::new(installation_key.to_vec()),
            rules,
        })
    }

    pub(crate) fn sanitize(&self, input: &str) -> Result<SanitizationResult, SanitizationError> {
        if input.chars().count() > MAX_SANITIZER_INPUT_CHARS {
            return Err(SanitizationError::InputTooLong {
                max: MAX_SANITIZER_INPUT_CHARS,
            });
        }

        let mut text = input.to_string();
        let mut class_counts = BTreeMap::new();
        for rule in &self.rules {
            let count = self.apply_rule(&mut text, rule)?;
            if count > 0 {
                class_counts.insert(rule.class, count);
            }
        }
        Ok(SanitizationResult {
            text,
            class_counts,
            sanitizer_version: EVIDENCE_SANITIZER_V1,
        })
    }

    fn apply_rule(
        &self,
        text: &mut String,
        rule: &RedactionRule,
    ) -> Result<u32, SanitizationError> {
        let mut replacements = Vec::new();
        for captures in rule.regex.captures_iter(text) {
            let Some(value) = captures.get(1) else {
                continue;
            };
            if inside_existing_marker(text, value.start(), value.end()) {
                continue;
            }
            replacements.push((
                value.start(),
                value.end(),
                self.marker(rule.class, value.as_str())?,
            ));
        }
        let count = replacements.len() as u32;
        for (start, end, marker) in replacements.into_iter().rev() {
            text.replace_range(start..end, &marker);
        }
        Ok(count)
    }

    fn marker(&self, class: RedactionClass, value: &str) -> Result<String, SanitizationError> {
        let normalized = Zeroizing::new(normalize_for_marker(class, value));
        let mut mac = Hmac::<Sha256>::new_from_slice(self.marker_key.as_slice())
            .map_err(|_| SanitizationError::MarkerDerivationFailed)?;
        mac.update(class.as_str().as_bytes());
        mac.update(&[0]);
        mac.update(normalized.as_bytes());
        let bytes = mac.finalize().into_bytes();
        let token = bytes[..MARKER_TOKEN_BYTES]
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<String>();
        Ok(format!("<redacted:{}:{token}>", class.as_str()))
    }
}

struct RedactionRule {
    class: RedactionClass,
    regex: Regex,
}

impl RedactionRule {
    fn compile(class: RedactionClass, pattern: &str) -> Result<Self, SanitizationError> {
        Ok(Self {
            class,
            regex: Regex::new(pattern).map_err(|_| SanitizationError::RegistryUnavailable)?,
        })
    }
}

const RULE_SPECS: [(RedactionClass, &str); 12] = [
    (
        RedactionClass::PrivateKey,
        r"(?s)(-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----.*?-----END [A-Z0-9 ]*PRIVATE KEY-----)",
    ),
    (
        RedactionClass::Token,
        r#"(?i)\b(?:api[_-]?key|access[_-]?token|refresh[_-]?token|session[_-]?token)\s*[:=]\s*["']?([A-Za-z0-9._~+/=-]{8,})["']?"#,
    ),
    (
        RedactionClass::Authorization,
        r"(?i)\b(?:authorization\s*:\s*(?:bearer|basic)\s+|cookie\s*:\s*)([^\s,;]+)",
    ),
    (
        RedactionClass::CredentialAssignment,
        r#"(?i)\b(?:password|passwd|pwd|secret|credential)\s*[:=]\s*(["'][^"'\r\n]{4,}["']|[A-Za-z0-9_+/=-]{8,})"#,
    ),
    (
        RedactionClass::CredentialUrl,
        r"(?i)\b[a-z][a-z0-9+.-]*://([^/\s:@]+:[^/\s@]+)@",
    ),
    (
        RedactionClass::ConnectionString,
        r"(?i)\b((?:mongodb(?:\+srv)?|postgres(?:ql)?|mysql|redis)://[^\s]+)",
    ),
    (
        RedactionClass::SecretEnvironment,
        r"(?i)\b(?:AWS_SECRET_ACCESS_KEY|DATABASE_URL|PRIVATE_TOKEN|CLIENT_SECRET)\s*=\s*([A-Za-z0-9_+/=.-]{8,})",
    ),
    (
        RedactionClass::UserHomePath,
        r"(?i)([A-Z]:\\Users\\[^\\/\s]+(?:\\[^\s,;]+)*|/(?:home|Users)/[^/\s]+(?:/[^\s,;]+)*)",
    ),
    (
        RedactionClass::Email,
        r"(?i)\b([A-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[A-Z0-9](?:[A-Z0-9-]{0,61}[A-Z0-9])?(?:\.[A-Z0-9](?:[A-Z0-9-]{0,61}[A-Z0-9])?)+)\b",
    ),
    (RedactionClass::Phone, r"(?x)(\+?\d[\d\x20()-]{7,}\d)"),
    (
        RedactionClass::NetworkIdentifier,
        r"(?i)\b((?:\d{1,3}\.){3}\d{1,3}|[a-z0-9][a-z0-9.-]*\.(?:internal|local|corp))\b",
    ),
    (
        RedactionClass::CloudAccount,
        r"(?i)\b(?:tenant[_-]?id|subscription[_-]?id|account[_-]?id|project[_-]?id)\s*[:=]\s*([A-Za-z0-9][A-Za-z0-9_-]{5,})",
    ),
];

fn normalize_for_marker(class: RedactionClass, value: &str) -> String {
    let trimmed = value.trim().trim_matches(['\'', '"']);
    match class {
        RedactionClass::Email
        | RedactionClass::NetworkIdentifier
        | RedactionClass::UserHomePath
        | RedactionClass::CloudAccount => trimmed.to_lowercase(),
        _ => trimmed.to_string(),
    }
}

fn inside_existing_marker(text: &str, start: usize, end: usize) -> bool {
    let prefix = &text[..start];
    let suffix = &text[end..];
    prefix.rfind("<redacted:").is_some_and(|marker_start| {
        !prefix[marker_start..].contains('>') && suffix.find('>').is_some()
    })
}
