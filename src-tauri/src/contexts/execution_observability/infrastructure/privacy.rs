use crate::contexts::execution_observability::domain::{
    CapturePolicy, SafeAttributeValue, SafeAttributes,
};
use crate::platform::logging::redact_text;

const CONTENT_MARKER: &str = "[REDACTED_CONTENT]";

pub(super) fn sanitize_attributes(
    policy: CapturePolicy,
    attributes: &SafeAttributes,
) -> SafeAttributes {
    let entries = attributes.entries().iter().filter_map(|(key, value)| {
        if is_sensitive_key(key) {
            return match policy {
                CapturePolicy::MetadataOnly => None,
                CapturePolicy::RedactedContent => Some((
                    key.clone(),
                    SafeAttributeValue::String(CONTENT_MARKER.to_string()),
                )),
            };
        }
        let value = match value {
            SafeAttributeValue::String(value) => SafeAttributeValue::String(redact_text(value)),
            value => value.clone(),
        };
        Some((key.clone(), value))
    });
    SafeAttributes::try_from_entries(entries).unwrap_or_default()
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    [
        "prompt",
        "output",
        "content",
        "payload",
        "body",
        "header",
        "authorization",
        "credential",
        "secret",
        "token",
        "environment",
        "env.",
        "path",
        "argument",
    ]
    .iter()
    .any(|fragment| normalized.contains(fragment))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_only_drops_content_and_redacts_secret_shaped_values() {
        let attributes = SafeAttributes::try_from_entries([
            (
                "mcp.request.body".to_string(),
                SafeAttributeValue::String("private prompt".to_string()),
            ),
            (
                "safe.detail".to_string(),
                SafeAttributeValue::String(
                    "Bearer private-token C:\\Users\\developer\\private.json".to_string(),
                ),
            ),
        ])
        .expect("attributes");
        let sanitized = sanitize_attributes(CapturePolicy::MetadataOnly, &attributes);
        assert!(!sanitized.entries().contains_key("mcp.request.body"));
        assert_eq!(
            sanitized.entries().get("safe.detail"),
            Some(&SafeAttributeValue::String(
                "Bearer [REDACTED] [REDACTED_PATH]".to_string()
            ))
        );
    }

    #[test]
    fn redacted_content_never_retains_raw_content() {
        let attributes = SafeAttributes::try_from_entries([(
            "gen_ai.prompt".to_string(),
            SafeAttributeValue::String("private prompt".to_string()),
        )])
        .expect("attributes");
        let sanitized = sanitize_attributes(CapturePolicy::RedactedContent, &attributes);
        assert_eq!(
            sanitized.entries().get("gen_ai.prompt"),
            Some(&SafeAttributeValue::String(CONTENT_MARKER.to_string()))
        );
    }
}
