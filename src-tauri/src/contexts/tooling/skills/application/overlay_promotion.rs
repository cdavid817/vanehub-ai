#![cfg_attr(not(test), allow(dead_code))]

use super::{
    OverlayApplicationError, OverlayContentScannerPort, OverlayEffectivePackageSnapshot,
    OverlayLimitKind, OverlayManifestSnapshot, OverlayPayloadRepository, OverlayPinSnapshot,
    OverlayPromotionRequest, OverlayScanResult, SkillApplicationError,
};
use crate::contexts::tooling::skills::domain::{
    OverlayContentKind, OverlayDocument, OverlayOrigin, OverlayTrustState, DEFAULT_OVERLAY_LIMITS,
};
use std::collections::BTreeSet;

const MAXIMUM_SCAN_RULE_IDS: usize = 32;

pub(crate) struct OverlayPromotionValidationInput<'a> {
    pub(crate) request: &'a OverlayPromotionRequest,
    pub(crate) current: &'a OverlayManifestSnapshot,
    pub(crate) base: &'a OverlayEffectivePackageSnapshot,
    pub(crate) pin: &'a OverlayPinSnapshot,
    pub(crate) live_scan: &'a OverlayScanResult,
}

pub(crate) fn validate_overlay_promotion(
    input: &OverlayPromotionValidationInput<'_>,
) -> Result<(), SkillApplicationError> {
    let request = input.request;
    let document = &input.current.document;
    if document.canonical_skill_id != request.canonical_skill_id {
        return Err(OverlayApplicationError::InvalidRequest {
            code: "overlay-promotion-skill-mismatch".to_string(),
        }
        .into());
    }
    if input.pin.pinned {
        return Err(OverlayApplicationError::PinnedRefusal {
            skill_id: request.canonical_skill_id.as_str().to_string(),
        }
        .into());
    }
    if document.trust().origin() != OverlayOrigin::Imported
        || document.trust().state() != OverlayTrustState::Untrusted
    {
        return Err(OverlayApplicationError::TrustRequired {
            revision: document.revision(),
        }
        .into());
    }
    if !input.live_scan.passed {
        return Err(OverlayApplicationError::ImportRejected {
            code: "overlay-promotion-hard-deny-scan".to_string(),
        }
        .into());
    }

    let document_hash_changed = input.current.document_hash != request.reviewed_document_hash;
    let scan_changed = input.live_scan != &request.reviewed_scan;
    if document.revision() != request.reviewed_revision
        || request.witnesses.expected_overlay_revision != Some(document.revision())
        || document_hash_changed
        || scan_changed
    {
        return Err(OverlayApplicationError::PromotionWitnessMismatch {
            reviewed_revision: request.reviewed_revision,
            current_revision: document.revision(),
            document_hash_changed,
            scan_changed,
        }
        .into());
    }

    let base_changed = document.base_witness.base_identity != input.base.base_identity
        || document.base_witness.instruction_hash != input.base.instruction_hash
        || document.base_witness.package_hash != input.base.package_hash
        || request.witnesses.expected_base_instruction_hash != input.base.instruction_hash
        || request.witnesses.expected_base_package_hash != input.base.package_hash;
    if base_changed || request.witnesses.expected_pinned != input.pin.pinned {
        return Err(OverlayApplicationError::StaleWitnesses {
            expected_revision: request.witnesses.expected_overlay_revision,
            current_revision: Some(document.revision()),
            base_changed,
            payload_changed: false,
            pin_changed: request.witnesses.expected_pinned != input.pin.pinned,
        }
        .into());
    }
    Ok(())
}

pub(crate) fn rescan_overlay_for_promotion(
    document: &OverlayDocument,
    key: &super::OverlayKey,
    payloads: &dyn OverlayPayloadRepository,
    scanner: &dyn OverlayContentScannerPort,
) -> Result<OverlayScanResult, SkillApplicationError> {
    validate_document_limits(document)?;
    let mut scans = vec![scanner.scan_text("")];
    for patch in &document.patches {
        scans.push(scanner.scan_text(&patch.old_string));
        scans.push(scanner.scan_text(&patch.new_string));
    }
    for block in &document.learn_blocks {
        scans.push(scanner.scan_text(&block.guidance));
    }
    for file in &document.files {
        let content = payloads.read_verified(key, &file.content_hash)?;
        let validated = scanner.validate_file(&file.logical_path, &file.media_type, &content)?;
        let metadata_matches = validated.logical_path == file.logical_path
            && validated.media_type == file.media_type
            && validated.size_bytes == file.size
            && validated.content_hash == file.content_hash
            && file.payload_ref == format!("sha256/{}", file.content_hash);
        if !metadata_matches {
            return Err(OverlayApplicationError::Integrity {
                code: super::OverlayIntegrityCode::PayloadHashMismatch,
            }
            .into());
        }
        if validated.content_kind == OverlayContentKind::Utf8Text {
            let text = std::str::from_utf8(&content).map_err(|_| {
                OverlayApplicationError::ImportRejected {
                    code: "overlay-promotion-invalid-utf8".to_string(),
                }
            })?;
            scans.push(scanner.scan_text(text));
        }
    }
    let scanner_version = scans
        .first()
        .map(|scan| scan.scanner_version().to_string())
        .unwrap_or_default();
    if scanner_version.is_empty()
        || scans
            .iter()
            .any(|scan| scan.scanner_version() != scanner_version)
    {
        return Err(OverlayApplicationError::ImportRejected {
            code: "overlay-promotion-inconsistent-scanner-version".to_string(),
        }
        .into());
    }
    let safe_rule_ids = scans
        .iter()
        .flat_map(|scan| scan.safe_rule_ids())
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let rule_ids_truncated = safe_rule_ids.len() > MAXIMUM_SCAN_RULE_IDS;
    Ok(OverlayScanResult {
        scanner_version,
        passed: safe_rule_ids.is_empty(),
        safe_rule_ids: safe_rule_ids
            .into_iter()
            .take(MAXIMUM_SCAN_RULE_IDS)
            .collect(),
        rule_ids_truncated,
    })
}

fn validate_document_limits(document: &OverlayDocument) -> Result<(), SkillApplicationError> {
    let mutation_count =
        document.patches.len() + document.learn_blocks.len() + document.files.len();
    if mutation_count > DEFAULT_OVERLAY_LIMITS.maximum_mutations {
        return Err(OverlayApplicationError::LimitExceeded {
            kind: OverlayLimitKind::MutationCount,
            maximum: DEFAULT_OVERLAY_LIMITS.maximum_mutations as u64,
            actual: mutation_count as u64,
        }
        .into());
    }
    let instruction_characters = document
        .patches
        .iter()
        .map(|patch| patch.old_string.chars().count() + patch.new_string.chars().count())
        .sum::<usize>()
        + document
            .learn_blocks
            .iter()
            .map(|block| block.guidance.chars().count())
            .sum::<usize>();
    if instruction_characters > DEFAULT_OVERLAY_LIMITS.maximum_instruction_characters {
        return Err(OverlayApplicationError::LimitExceeded {
            kind: OverlayLimitKind::InstructionCharacters,
            maximum: DEFAULT_OVERLAY_LIMITS.maximum_instruction_characters as u64,
            actual: instruction_characters as u64,
        }
        .into());
    }
    Ok(())
}
