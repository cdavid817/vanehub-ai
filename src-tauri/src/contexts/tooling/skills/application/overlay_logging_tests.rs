use super::{
    OverlayValidationDiagnostic, OverlayValidationReason, OverlayValidationTarget,
    SkillApplicationError, SkillLogAction, SkillLogEvent, SkillLoggingPort,
};
use crate::contexts::tooling::skills::domain::{OverlayTextRuleId, OVERLAY_TEXT_SCANNER_VERSION};
use std::sync::Mutex;

#[derive(Default)]
struct CapturingLogging {
    events: Mutex<Vec<SkillLogEvent>>,
}

impl SkillLoggingPort for CapturingLogging {
    fn record(&self, event: &SkillLogEvent) -> Result<(), SkillApplicationError> {
        self.events.lock().expect("events").push(event.clone());
        Ok(())
    }
}

#[test]
fn overlay_refusal_logs_only_safe_hashes_sizes_and_rule_ids() {
    let raw_identity = "private-skill-id";
    let raw_path = "references/customer-secret.md";
    let raw_content = b"password=customer-secret-value";
    let diagnostic = OverlayValidationDiagnostic::refused(
        OverlayValidationTarget::SupportingFile,
        raw_identity,
        Some(raw_path),
        Some(raw_content),
        OverlayValidationReason::TextRule,
        &[OverlayTextRuleId::CredentialStructure],
        "2026-08-11T00:00:00Z",
    );
    let logging = CapturingLogging::default();

    logging
        .record_overlay_validation(&diagnostic)
        .expect("safe diagnostic");

    let events = logging.events.lock().expect("events");
    let event = events.first().expect("captured event");
    assert_eq!(event.action, SkillLogAction::OverlayValidation);
    assert_eq!(event.skill_id, None);
    assert_eq!(event.message, "Overlay validation refused");
    assert_eq!(
        event.context.get("scannerVersion").map(String::as_str),
        Some(OVERLAY_TEXT_SCANNER_VERSION)
    );
    assert_eq!(
        event.context.get("ruleIds").map(String::as_str),
        Some("overlay.credential-structure")
    );
    assert_eq!(
        event.context.get("sizeBytes"),
        Some(&raw_content.len().to_string())
    );
    for key in ["identityHash", "pathHash", "contentHash"] {
        let value = event.context.get(key).expect("safe hash");
        assert_eq!(value.len(), 64);
        assert!(value.chars().all(|character| character.is_ascii_hexdigit()));
    }

    let serialized = format!("{event:?}");
    assert!(!serialized.contains(raw_identity));
    assert!(!serialized.contains(raw_path));
    assert!(!serialized.contains("customer-secret-value"));
}

#[test]
fn overlay_diagnostic_omits_absent_path_content_and_rules() {
    let diagnostic = OverlayValidationDiagnostic::refused(
        OverlayValidationTarget::ExactPatch,
        "skill-id",
        None,
        None,
        OverlayValidationReason::Pinned,
        &[],
        "2026-08-11T00:00:00Z",
    );
    let event = diagnostic.to_log_event();

    assert_eq!(
        event.context.get("reason").map(String::as_str),
        Some("pinned")
    );
    assert!(!event.context.contains_key("pathHash"));
    assert!(!event.context.contains_key("contentHash"));
    assert!(!event.context.contains_key("sizeBytes"));
    assert!(!event.context.contains_key("ruleIds"));
}
