use super::*;

fn input() -> AuthorizedCorrectionDraftInputV1 {
    AuthorizedCorrectionDraftInputV1 {
        workspace_id: "workspace:one".into(),
        target_skill_id: "skill-one".into(),
        target_revision: "revision-one".into(),
        authorization_id: "authorization-one".into(),
        authorization_witness_hash: "sha256:authorization".into(),
        assessment_id: "assessment-one".into(),
        sanitizer_version: 1,
        authorization_current: true,
        trigger: "When  the\r\ncheck fails".into(),
        guidance: "Use ＮＦＫＣ output.".into(),
        verification: "Run\tbounded tests.".into(),
        created_at_ms: 10,
    }
}

#[test]
fn canonical_output_is_byte_reproducible_across_equivalent_input() {
    let first = produce_authorized_correction_draft(&input()).expect("first draft");
    let mut equivalent = input();
    equivalent.trigger = "When the check fails".into();
    equivalent.guidance = "Use NFKC output.".into();
    equivalent.verification = "Run bounded tests.".into();
    equivalent.created_at_ms = 99;
    let second = produce_authorized_correction_draft(&equivalent).expect("second draft");

    assert_eq!(first.content.as_bytes(), second.content.as_bytes());
    assert_eq!(first.record.content_hash, second.record.content_hash);
    assert_eq!(first.record.draft_id, second.record.draft_id);
    assert_eq!(
        first.record.source_witness_hash,
        second.record.source_witness_hash
    );
    assert_eq!(
        first.record.content_size_bytes as usize,
        first.content.len()
    );
    assert_eq!(
        first.content,
        "### Verified correction guidance\n\n- Trigger: When the check fails\n- Guidance: Use NFKC output.\n- Verify: Run bounded tests.\n"
    );
}

#[test]
fn incomplete_unsafe_and_unauthorized_inputs_produce_no_draft() {
    let mut incomplete = input();
    incomplete.verification = "  \r\n".into();
    assert_eq!(
        produce_authorized_correction_draft(&incomplete),
        Err(CorrectionDraftError::IncompleteShape)
    );

    let mut unsafe_control = input();
    unsafe_control.guidance = "unsafe\u{0007}".into();
    assert_eq!(
        produce_authorized_correction_draft(&unsafe_control),
        Err(CorrectionDraftError::UnsafeControl)
    );

    let mut unauthorized = input();
    unauthorized.authorization_current = false;
    assert_eq!(
        produce_authorized_correction_draft(&unauthorized),
        Err(CorrectionDraftError::AuthorizationUnavailable)
    );
}

#[test]
fn excessive_fields_or_missing_sanitizer_are_rejected() {
    let mut too_large = input();
    too_large.guidance = "界".repeat(513);
    assert_eq!(
        produce_authorized_correction_draft(&too_large),
        Err(CorrectionDraftError::FieldLimit)
    );

    let mut unsanitized = input();
    unsanitized.sanitizer_version = 0;
    assert_eq!(
        produce_authorized_correction_draft(&unsanitized),
        Err(CorrectionDraftError::SanitizationUnavailable)
    );
}

#[test]
fn every_noncanonical_provenance_is_permanently_excluded() {
    assert!(AutomaticDraftProvenance::DeterministicAuthorizedCorrection.eligible());
    for provenance in [
        AutomaticDraftProvenance::UserAuthored,
        AutomaticDraftProvenance::Edited,
        AutomaticDraftProvenance::ModelGenerated,
        AutomaticDraftProvenance::Imported,
        AutomaticDraftProvenance::ExactPatch,
        AutomaticDraftProvenance::File,
        AutomaticDraftProvenance::Script,
        AutomaticDraftProvenance::Unknown,
    ] {
        assert!(!provenance.eligible(), "{provenance:?}");
    }
}
