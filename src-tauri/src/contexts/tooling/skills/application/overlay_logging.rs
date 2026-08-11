#![allow(dead_code)]

use super::{SkillLogAction, SkillLogEvent, SkillLogLevel};
use crate::contexts::tooling::skills::domain::{OverlayTextRuleId, OVERLAY_TEXT_SCANNER_VERSION};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(crate) const OVERLAY_VALIDATION_DIAGNOSTIC_VERSION: &str = "overlay-validation-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayValidationTarget {
    ExactPatch,
    LearnedGuidance,
    SupportingFile,
    Import,
    TrustPromotion,
    Reconciliation,
    Replay,
}

impl OverlayValidationTarget {
    fn as_str(self) -> &'static str {
        match self {
            Self::ExactPatch => "exact-patch",
            Self::LearnedGuidance => "learned-guidance",
            Self::SupportingFile => "supporting-file",
            Self::Import => "import",
            Self::TrustPromotion => "trust-promotion",
            Self::Reconciliation => "reconciliation",
            Self::Replay => "replay",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayValidationReason {
    Path,
    Media,
    Size,
    TextRule,
    Pinned,
    StaleWitness,
    Trust,
    Integrity,
    ReplayConflict,
}

impl OverlayValidationReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Media => "media",
            Self::Size => "size",
            Self::TextRule => "text-rule",
            Self::Pinned => "pinned",
            Self::StaleWitness => "stale-witness",
            Self::Trust => "trust",
            Self::Integrity => "integrity",
            Self::ReplayConflict => "replay-conflict",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayValidationDiagnostic {
    target: OverlayValidationTarget,
    identity_hash: String,
    path_hash: Option<String>,
    content_hash: Option<String>,
    size_bytes: Option<usize>,
    reason: OverlayValidationReason,
    safe_rule_ids: Vec<&'static str>,
    timestamp: String,
}

impl OverlayValidationDiagnostic {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn refused(
        target: OverlayValidationTarget,
        canonical_skill_identity: &str,
        logical_path: Option<&str>,
        content: Option<&[u8]>,
        reason: OverlayValidationReason,
        rule_ids: &[OverlayTextRuleId],
        timestamp: &str,
    ) -> Self {
        let mut safe_rule_ids = rule_ids
            .iter()
            .map(|rule_id| rule_id.as_str())
            .collect::<Vec<_>>();
        safe_rule_ids.sort_unstable();
        safe_rule_ids.dedup();
        Self {
            target,
            identity_hash: domain_hash(b"skill-identity\0", canonical_skill_identity.as_bytes()),
            path_hash: logical_path
                .map(|path| domain_hash(b"overlay-logical-path\0", path.as_bytes())),
            content_hash: content.map(|value| domain_hash(b"overlay-content\0", value)),
            size_bytes: content.map(<[u8]>::len),
            reason,
            safe_rule_ids,
            timestamp: timestamp.to_string(),
        }
    }

    pub(crate) fn to_log_event(&self) -> SkillLogEvent {
        let mut context = BTreeMap::from([
            (
                "diagnosticVersion".to_string(),
                OVERLAY_VALIDATION_DIAGNOSTIC_VERSION.to_string(),
            ),
            ("identityHash".to_string(), self.identity_hash.clone()),
            ("reason".to_string(), self.reason.as_str().to_string()),
            ("target".to_string(), self.target.as_str().to_string()),
        ]);
        if let Some(path_hash) = &self.path_hash {
            context.insert("pathHash".to_string(), path_hash.clone());
        }
        if let Some(content_hash) = &self.content_hash {
            context.insert("contentHash".to_string(), content_hash.clone());
        }
        if let Some(size_bytes) = self.size_bytes {
            context.insert("sizeBytes".to_string(), size_bytes.to_string());
        }
        if !self.safe_rule_ids.is_empty() {
            context.insert(
                "scannerVersion".to_string(),
                OVERLAY_TEXT_SCANNER_VERSION.to_string(),
            );
            context.insert("ruleIds".to_string(), self.safe_rule_ids.join(","));
        }
        SkillLogEvent {
            action: SkillLogAction::OverlayValidation,
            level: SkillLogLevel::Warn,
            skill_id: None,
            message: "Overlay validation refused".to_string(),
            timestamp: self.timestamp.clone(),
            context,
        }
    }
}

fn domain_hash(domain: &[u8], value: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(value);
    let digest = hasher.finalize();
    let mut encoded = String::with_capacity(digest.len() * 2);
    for byte in digest {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}
