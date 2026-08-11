use super::*;
use crate::contexts::tooling::skills::domain::{
    OverlayBaseWitness, OverlayDocument, OverlayScope, OverlayTrust, SkillId, SkillLayer,
    OVERLAY_TEXT_SCANNER_VERSION,
};

#[test]
fn promotion_accepts_only_the_exact_reviewed_revision_document_hash_and_scan() {
    let current = imported_snapshot();
    let base = base_snapshot();
    let scan = passed_scan();
    let request = promotion_request(&current, &scan);

    let result = validate(&request, &current, &base, &scan, false);

    assert!(result.is_ok());
}

#[test]
fn promotion_rejects_an_import_whose_revision_changed_after_review() {
    let mut current = imported_snapshot();
    let base = base_snapshot();
    let scan = passed_scan();
    let request = promotion_request(&current, &scan);
    current
        .document
        .advance_revision("prior-document-hash", "2026-08-11T13:00:00Z")
        .expect("advance imported revision");

    let error =
        validate(&request, &current, &base, &scan, false).expect_err("changed revision must fail");

    assert!(matches!(
        error,
        SkillApplicationError::Overlay(OverlayApplicationError::PromotionWitnessMismatch {
            reviewed_revision: 1,
            current_revision: 2,
            ..
        })
    ));
}

#[test]
fn promotion_rejects_an_import_whose_document_hash_changed_after_review() {
    let mut current = imported_snapshot();
    let base = base_snapshot();
    let scan = passed_scan();
    let request = promotion_request(&current, &scan);
    current.document_hash = "changed-document-hash".to_string();

    let error = validate(&request, &current, &base, &scan, false)
        .expect_err("changed document hash must fail");

    assert!(matches!(
        error,
        SkillApplicationError::Overlay(OverlayApplicationError::PromotionWitnessMismatch {
            document_hash_changed: true,
            ..
        })
    ));
}

#[test]
fn promotion_rejects_a_base_that_changed_after_review() {
    let current = imported_snapshot();
    let mut base = base_snapshot();
    let scan = passed_scan();
    let request = promotion_request(&current, &scan);
    base.instruction_hash = "changed-instruction-hash".to_string();

    let error =
        validate(&request, &current, &base, &scan, false).expect_err("changed base must fail");

    assert!(matches!(
        error,
        SkillApplicationError::Overlay(OverlayApplicationError::StaleWitnesses {
            base_changed: true,
            ..
        })
    ));
}

#[test]
fn promotion_rejects_a_scan_result_that_changed_after_review() {
    let current = imported_snapshot();
    let base = base_snapshot();
    let reviewed_scan = passed_scan();
    let live_scan = OverlayScanResult {
        scanner_version: "overlay-text-v2".to_string(),
        passed: true,
        safe_rule_ids: Vec::new(),
        rule_ids_truncated: false,
    };
    let request = promotion_request(&current, &reviewed_scan);

    let error =
        validate(&request, &current, &base, &live_scan, false).expect_err("changed scan must fail");

    assert!(matches!(
        error,
        SkillApplicationError::Overlay(OverlayApplicationError::PromotionWitnessMismatch {
            scan_changed: true,
            ..
        })
    ));
}

#[test]
fn promotion_hard_denies_even_when_the_failed_scan_is_claimed_as_reviewed() {
    let current = imported_snapshot();
    let base = base_snapshot();
    let failed_scan = OverlayScanResult {
        scanner_version: OVERLAY_TEXT_SCANNER_VERSION.to_string(),
        passed: false,
        safe_rule_ids: vec!["overlay.prompt-authority-override".to_string()],
        rule_ids_truncated: false,
    };
    let request = promotion_request(&current, &failed_scan);

    let error = validate(&request, &current, &base, &failed_scan, false)
        .expect_err("reviewing a failed scan must not authorize promotion");

    assert!(matches!(
        error,
        SkillApplicationError::Overlay(OverlayApplicationError::ImportRejected { code })
            if code == "overlay-promotion-hard-deny-scan"
    ));
}

#[test]
fn overlay_operation_contracts_expose_no_hard_deny_bypass_flag() {
    for request_name in [
        "OverlayMutationRequest",
        "OverlayImportRequest",
        "OverlayPromotionRequest",
        "OverlayReconciliationRequest",
    ] {
        let source = request_struct_source(request_name).to_ascii_lowercase();
        for forbidden in [
            "force",
            "trust_anyway",
            "trustanyway",
            "bypass",
            "skip_scan",
            "skipscan",
            "allow_unsafe",
            "allowunsafe",
        ] {
            assert!(
                !source.contains(forbidden),
                "{request_name} must not expose `{forbidden}`"
            );
        }
    }
}

#[test]
fn promotion_refuses_a_pinned_target_before_changing_trust() {
    let current = imported_snapshot();
    let base = base_snapshot();
    let scan = passed_scan();
    let request = promotion_request(&current, &scan);

    let error =
        validate(&request, &current, &base, &scan, true).expect_err("pinned target must fail");

    assert!(matches!(
        error,
        SkillApplicationError::Overlay(OverlayApplicationError::PinnedRefusal { .. })
    ));
    assert_eq!(
        current.document.trust().state(),
        crate::contexts::tooling::skills::domain::OverlayTrustState::Untrusted
    );
}

fn request_struct_source(name: &str) -> &'static str {
    let source = include_str!("overlay_models.rs");
    let marker = format!("pub(crate) struct {name}");
    let start = source.find(&marker).expect("request struct declaration");
    let declaration = &source[start..];
    let end = declaration
        .find("\n}")
        .expect("request struct closing brace");
    &declaration[..end + 2]
}

fn validate(
    request: &OverlayPromotionRequest,
    current: &OverlayManifestSnapshot,
    base: &OverlayEffectivePackageSnapshot,
    live_scan: &OverlayScanResult,
    pinned: bool,
) -> Result<(), SkillApplicationError> {
    let pin = OverlayPinSnapshot {
        pinned,
        revision_witness: if pinned { "pin-2" } else { "pin-1" }.to_string(),
    };
    validate_overlay_promotion(&OverlayPromotionValidationInput {
        request,
        current,
        base,
        pin: &pin,
        live_scan,
    })
}

fn promotion_request(
    current: &OverlayManifestSnapshot,
    scan: &OverlayScanResult,
) -> OverlayPromotionRequest {
    OverlayPromotionRequest {
        canonical_skill_id: current.document.canonical_skill_id.clone(),
        scope: current.document.scope(),
        workspace_identity: None,
        reviewed_revision: current.document.revision(),
        reviewed_document_hash: current.document_hash.clone(),
        reviewed_scan: scan.clone(),
        witnesses: OverlayWitnesses {
            expected_overlay_revision: Some(current.document.revision()),
            expected_base_instruction_hash: "instruction-hash".to_string(),
            expected_base_package_hash: "package-hash".to_string(),
            expected_payload_hash: None,
            expected_pinned: false,
        },
    }
}

fn passed_scan() -> OverlayScanResult {
    OverlayScanResult {
        scanner_version: OVERLAY_TEXT_SCANNER_VERSION.to_string(),
        passed: true,
        safe_rule_ids: Vec::new(),
        rule_ids_truncated: false,
    }
}

fn imported_snapshot() -> OverlayManifestSnapshot {
    let document = OverlayDocument::new(
        SkillId::parse("promotion-skill").expect("skill id"),
        OverlayScope::User,
        None,
        OverlayBaseWitness::new("system:promotion-skill", "instruction-hash", "package-hash")
            .expect("base witness"),
        OverlayTrust::untrusted_imported(Some("reviewed-import.zip".to_string())),
        "2026-08-11T12:00:00Z",
    )
    .expect("imported overlay");
    OverlayManifestSnapshot {
        document,
        document_hash: "reviewed-document-hash".to_string(),
    }
}

fn base_snapshot() -> OverlayEffectivePackageSnapshot {
    OverlayEffectivePackageSnapshot {
        canonical_skill_id: SkillId::parse("promotion-skill").expect("skill id"),
        base_identity: "system:promotion-skill".to_string(),
        base_layer: SkillLayer::System,
        instructions: "Build safely.".to_string(),
        resources: Vec::new(),
        instruction_hash: "instruction-hash".to_string(),
        package_hash: "package-hash".to_string(),
    }
}
