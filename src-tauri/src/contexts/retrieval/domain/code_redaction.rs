// Predates the production panic-shortcut gate; removing this attribute is the
// definition of done for this file, and it may be removed without ceremony.
// TODO(retire-production-panic-shortcuts): 6 pre-existing sites, in a domain
// module — the least defensible placement for a panic shortcut, so clear these first.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use regex::{Captures, Regex};
use std::sync::OnceLock;

pub(crate) const REDACTED_MARKER: &str = "[REDACTED]";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RedactedCode {
    pub(crate) text: String,
    pub(crate) count: u32,
}

pub(crate) fn redact_code(input: &str) -> RedactedCode {
    let mut text = input.to_string();
    let mut count = 0_u32;
    for expression in [
        private_key_expression(),
        quoted_assignment_expression(),
        bare_assignment_expression(),
        bearer_expression(),
        provider_token_expression(),
        internal_url_expression(),
    ] {
        text = expression
            .replace_all(&text, |captures: &Captures<'_>| {
                if captures
                    .name("value")
                    .is_some_and(|value| value.as_str() == REDACTED_MARKER)
                {
                    return captures[0].to_string();
                }
                count = count.saturating_add(1);
                replacement(captures)
            })
            .into_owned();
    }
    RedactedCode { text, count }
}

fn replacement(captures: &Captures<'_>) -> String {
    let prefix = captures.name("prefix").map_or("", |value| value.as_str());
    let quote = captures.name("quote").map_or("", |value| value.as_str());
    format!("{prefix}{quote}{REDACTED_MARKER}{quote}")
}

fn private_key_expression() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| {
        Regex::new(
            r"(?s)(?P<prefix>-----BEGIN [^-\r\n]*PRIVATE KEY-----).*?-----END [^-\r\n]*PRIVATE KEY-----",
        )
        .expect("private-key redaction regex")
    })
}

fn quoted_assignment_expression() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| {
        Regex::new(concat!(
            r"(?i)(?P<prefix>\b(?:api[_-]?key|access[_-]?key|client[_-]?secret|",
            r"password|passwd|token|auth(?:orization)?|credential)\b\s*[:=]\s*)",
            r#"(?P<quote>[\"'])(?P<value>(?:\\.|[^\"'\r\n])*)(?:[\"'])"#,
        ))
        .expect("quoted-assignment redaction regex")
    })
}

fn bare_assignment_expression() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| {
        Regex::new(concat!(
            r"(?i)(?P<prefix>\b(?:api[_-]?key|access[_-]?key|client[_-]?secret|",
            r"password|passwd|token|auth(?:orization)?|credential)\b\s*[:=]\s*)",
            r#"(?P<value>[^\s,;\}\]\[\"']+)"#,
        ))
        .expect("bare-assignment redaction regex")
    })
}

fn bearer_expression() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| {
        Regex::new(r"(?i)(?P<prefix>\bbearer\s+)[A-Za-z0-9._~+/=-]{6,}")
            .expect("bearer redaction regex")
    })
}

fn provider_token_expression() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| {
        Regex::new(r"\b(?:sk-[A-Za-z0-9_-]{8,}|ghp_[A-Za-z0-9_]{8,}|github_pat_[A-Za-z0-9_]{8,}|AKIA[A-Z0-9]{12,})\b")
            .expect("provider-token redaction regex")
    })
}

fn internal_url_expression() -> &'static Regex {
    static VALUE: OnceLock<Regex> = OnceLock::new();
    VALUE.get_or_init(|| {
        Regex::new(r#"(?i)https?://(?:localhost|127\.0\.0\.1|10\.\d+\.\d+\.\d+|192\.168\.\d+\.\d+|172\.(?:1[6-9]|2\d|3[01])\.\d+\.\d+)(?::\d+)?(?:/[^\s"']*)?"#)
            .expect("internal-url redaction regex")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_assignments_tokens_private_keys_and_internal_urls() {
        let input = r#"const api_key = "SENSITIVE-ASSIGNMENT";
password: 'SENSITIVE-PASSWORD'
let header = "Bearer SENSITIVE-BEARER";
let direct = "sk-SENSITIVEPROVIDERTOKEN";
let url = "http://10.20.30.40/internal";
-----BEGIN PRIVATE KEY-----
SENSITIVE-PRIVATE-KEY
-----END PRIVATE KEY-----"#;
        let redacted = redact_code(input);
        for sentinel in [
            "SENSITIVE-ASSIGNMENT",
            "SENSITIVE-PASSWORD",
            "SENSITIVE-BEARER",
            "SENSITIVEPROVIDERTOKEN",
            "10.20.30.40",
            "SENSITIVE-PRIVATE-KEY",
        ] {
            assert!(!redacted.text.contains(sentinel), "leaked {sentinel}");
        }
        assert_eq!(redacted.text.matches(REDACTED_MARKER).count(), 6);
        assert_eq!(redacted.count, 6);
    }

    #[test]
    fn leaves_normal_code_and_public_urls_unchanged() {
        let input = "const total = 42; const docs = \"https://example.com/docs\";";
        assert_eq!(
            redact_code(input),
            RedactedCode {
                text: input.to_string(),
                count: 0,
            }
        );
    }

    #[test]
    fn redaction_is_idempotent_for_already_redacted_assignments() {
        let once = redact_code("api_key = \"SENSITIVE-IDEMPOTENCE\"");
        let twice = redact_code(&once.text);
        assert_eq!(once.count, 1);
        assert_eq!(twice.count, 0);
        assert_eq!(twice.text, once.text);
    }
}
