use crate::contexts::tooling::skills::api::{OverlayError, SkillError};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayCommandError {
    kind: &'static str,
    code: Box<str>,
    message: Box<str>,
    expected_revision: Option<u64>,
    current_revision: Option<u64>,
    maximum: Option<u64>,
    actual: Option<u64>,
    base_changed: Option<bool>,
    payload_changed: Option<bool>,
    pin_changed: Option<bool>,
}

impl OverlayCommandError {
    pub(crate) fn validation(message: impl Into<String>) -> Self {
        Self::new("validation", "invalid-request", message.into())
    }

    fn new(kind: &'static str, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            kind,
            code: code.into().into_boxed_str(),
            message: message.into().into_boxed_str(),
            expected_revision: None,
            current_revision: None,
            maximum: None,
            actual: None,
            base_changed: None,
            payload_changed: None,
            pin_changed: None,
        }
    }
}

impl From<SkillError> for OverlayCommandError {
    fn from(error: SkillError) -> Self {
        match error {
            SkillError::Overlay(overlay) => Self::from(overlay),
            SkillError::NotFound(skill_id) => Self::new(
                "notFound",
                "skill-not-found",
                format!("Skill not found: {skill_id}"),
            ),
            SkillError::Conflict(value) => Self::new(
                "conflict",
                "overlay-already-exists",
                format!("Overlay already exists: {value}"),
            ),
            SkillError::ConcurrentModification(value) => Self::new(
                "stale",
                "concurrent-modification",
                format!("Overlay state changed while updating {value}"),
            ),
            SkillError::Validation(message) => Self::validation(message),
            SkillError::Domain(error) => Self::validation(error.to_string()),
            SkillError::ImmutablePackage(skill_id) => Self::new(
                "conflict",
                "immutable-package",
                format!("System Skill package is immutable: {skill_id}"),
            ),
            SkillError::InvalidResourceUri => Self::validation("Invalid Skill resource URI"),
            SkillError::ResourceEscape => Self::validation("Skill resource escapes its package"),
            SkillError::BinaryResource => Self::validation("Skill resource is binary"),
            SkillError::OversizedResource => Self::validation("Skill resource exceeds its limit"),
            SkillError::MountRootExternalLink(path) => Self::new(
                "conflict",
                "mount-root-external-link",
                format!("Skill mount root is an external link: {path}"),
            ),
            SkillError::MountRootBrokenLink(path) => Self::new(
                "conflict",
                "mount-root-broken-link",
                format!("Skill mount root is a broken link: {path}"),
            ),
            SkillError::MountRootNotDirectory(path) => Self::new(
                "conflict",
                "mount-root-not-directory",
                format!("Skill mount root is not a directory: {path}"),
            ),
            SkillError::Repository(message)
            | SkillError::Filesystem(message)
            | SkillError::Selection(message)
            | SkillError::Logging(message) => {
                Self::new("infrastructure", "storage-failure", message)
            }
        }
    }
}

impl From<OverlayError> for OverlayCommandError {
    fn from(error: OverlayError) -> Self {
        let message = error.to_string();
        match error {
            OverlayError::InvalidRequest { code } => Self::new("validation", code, message),
            OverlayError::PinnedRefusal { .. } => Self::new("pinned", "skill-pinned", message),
            OverlayError::StaleWitnesses {
                expected_revision,
                current_revision,
                base_changed,
                payload_changed,
                pin_changed,
            } => {
                let mut value = Self::new("stale", "stale-witnesses", message);
                value.expected_revision = expected_revision;
                value.current_revision = current_revision;
                value.base_changed = Some(base_changed);
                value.payload_changed = Some(payload_changed);
                value.pin_changed = Some(pin_changed);
                value
            }
            OverlayError::LimitExceeded {
                kind,
                maximum,
                actual,
            } => {
                let mut value = Self::new("limit", kind.as_str(), message);
                value.maximum = Some(maximum);
                value.actual = Some(actual);
                value
            }
            OverlayError::Integrity { code } => Self::new("integrity", code.as_str(), message),
            OverlayError::TrustRequired { revision } => {
                let mut value = Self::new("trust", "trust-required", message);
                value.current_revision = Some(revision);
                value
            }
            OverlayError::ImportRejected { code } => Self::new("import", code, message),
            OverlayError::PromotionWitnessMismatch {
                reviewed_revision,
                current_revision,
                document_hash_changed,
                scan_changed,
            } => {
                let mut value = Self::new("stale", "promotion-witness-mismatch", message);
                value.expected_revision = Some(reviewed_revision);
                value.current_revision = Some(current_revision);
                value.base_changed = Some(document_hash_changed);
                value.payload_changed = Some(scan_changed);
                value
            }
            OverlayError::NeedsReconciliation { .. } => {
                Self::new("conflict", "needs-reconciliation", message)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_overlay_error_serializes_machine_readable_witness_fields() {
        let error = OverlayCommandError::from(OverlayError::StaleWitnesses {
            expected_revision: Some(4),
            current_revision: Some(5),
            base_changed: true,
            payload_changed: false,
            pin_changed: true,
        });
        let value = serde_json::to_value(error).expect("structured error");
        assert_eq!(value["kind"], "stale");
        assert_eq!(value["code"], "stale-witnesses");
        assert_eq!(value["expectedRevision"], 4);
        assert_eq!(value["currentRevision"], 5);
        assert_eq!(value["baseChanged"], true);
        assert_eq!(value["pinChanged"], true);
    }

    #[test]
    fn pinned_overlay_error_uses_stable_kind_and_code() {
        let error = OverlayCommandError::from(OverlayError::PinnedRefusal {
            skill_id: "developer".to_string(),
        });
        let value = serde_json::to_value(error).expect("structured error");
        assert_eq!(value["kind"], "pinned");
        assert_eq!(value["code"], "skill-pinned");
    }
}
