use super::{
    OverlayBaseWitness, OverlayConflict, OverlayConflictState, OverlayDocument, OverlayFile,
    OverlayLearnBlock, OverlayMutationState, OverlayOrigin, OverlayPatch, OverlayScope,
    OverlayTrust, OverlayTrustState, SkillDomainError, SkillId,
};

fn witness() -> OverlayBaseWitness {
    OverlayBaseWitness::new("system:developer@1", "instruction-hash", "package-hash")
        .expect("valid witness")
}

#[test]
fn overlay_document_validates_scope_witnesses_and_initial_revision() {
    assert_eq!(OverlayScope::System.as_str(), "system");
    assert_eq!(OverlayScope::parse("user"), Some(OverlayScope::User));
    assert_eq!(OverlayScope::parse("project"), Some(OverlayScope::Project));
    assert_eq!(OverlayScope::parse("unknown"), None);

    let document = OverlayDocument::new(
        SkillId::parse("developer").expect("skill id"),
        OverlayScope::Project,
        Some("D:/code/app"),
        witness(),
        OverlayTrust::trusted_local(1),
        "2026-08-10T10:00:00Z",
    )
    .expect("valid document");

    assert_eq!(document.scope(), OverlayScope::Project);
    assert_eq!(document.workspace_identity(), Some("D:/code/app"));
    assert_eq!(document.revision(), 1);
    assert_eq!(document.prior_revision_hash(), None);
    assert!(document.trust().is_trusted_for_revision(1));

    assert!(matches!(
        OverlayDocument::new(
            SkillId::parse("developer").expect("skill id"),
            OverlayScope::Project,
            None,
            witness(),
            OverlayTrust::trusted_local(1),
            "2026-08-10T10:00:00Z",
        ),
        Err(SkillDomainError::InvalidOverlayValue(_))
    ));
}

#[test]
fn mutation_state_transitions_are_bounded_and_terminal_after_revert() {
    assert_eq!(
        OverlayMutationState::Active.disable().expect("disable"),
        OverlayMutationState::Disabled
    );
    assert_eq!(
        OverlayMutationState::Disabled.revert().expect("revert"),
        OverlayMutationState::Reverted
    );
    assert!(matches!(
        OverlayMutationState::Reverted.disable(),
        Err(SkillDomainError::InvalidOverlayTransition(_))
    ));

    let mut patch = OverlayPatch::new(
        "patch-1",
        "old",
        "new",
        false,
        "instruction-hash",
        "2026-08-10T10:00:00Z",
    )
    .expect("patch");
    let mut guidance =
        OverlayLearnBlock::new("learn-1", "Prefer focused tests.", "2026-08-10T10:00:00Z")
            .expect("guidance");
    let mut file = OverlayFile::new(
        "file-1",
        "references/team.md",
        "text/markdown",
        12,
        "content-hash",
        "sha256/content-hash",
        "2026-08-10T10:00:00Z",
    )
    .expect("file");

    assert_eq!(patch.state(), OverlayMutationState::Active);
    assert_eq!(guidance.state(), OverlayMutationState::Active);
    assert_eq!(file.state(), OverlayMutationState::Active);

    patch
        .disable("2026-08-10T11:00:00Z")
        .expect("disable patch");
    guidance
        .revert("2026-08-10T11:00:00Z")
        .expect("revert guidance");
    file.disable("2026-08-10T11:00:00Z").expect("disable file");
    file.revert("2026-08-10T12:00:00Z").expect("revert file");

    assert_eq!(patch.state(), OverlayMutationState::Disabled);
    assert_eq!(guidance.state(), OverlayMutationState::Reverted);
    assert_eq!(file.state(), OverlayMutationState::Reverted);
    assert!(guidance.disable("2026-08-10T12:00:00Z").is_err());
    assert!(patch.revert("2026-08-10T12:00:00Z").is_ok());
}

#[test]
fn trust_is_bound_to_the_exact_reviewed_revision() {
    let mut imported = OverlayTrust::untrusted_imported(Some("local archive".to_string()));
    assert_eq!(imported.state(), OverlayTrustState::Untrusted);
    assert_eq!(imported.origin(), OverlayOrigin::Imported);
    assert!(!imported.is_trusted_for_revision(3));

    imported
        .promote(3, "document-hash")
        .expect("promote exact revision");
    assert!(imported.is_trusted_for_revision(3));
    assert!(!imported.is_trusted_for_revision(4));
    assert_eq!(imported.reviewed_content_hash(), Some("document-hash"));
}

#[test]
fn advancing_revision_is_monotonic_and_records_the_prior_hash() {
    let mut document = OverlayDocument::new(
        SkillId::parse("developer").expect("skill id"),
        OverlayScope::User,
        None,
        witness(),
        OverlayTrust::trusted_local(1),
        "2026-08-10T10:00:00Z",
    )
    .expect("document");

    document
        .advance_revision("revision-one-hash", "2026-08-10T11:00:00Z")
        .expect("advance");

    assert_eq!(document.revision(), 2);
    assert_eq!(document.prior_revision_hash(), Some("revision-one-hash"));
    assert!(document.trust().is_trusted_for_revision(2));

    let mut imported = OverlayDocument::new(
        SkillId::parse("developer").expect("skill id"),
        OverlayScope::System,
        None,
        witness(),
        OverlayTrust::untrusted_imported(Some("archive".to_string())),
        "2026-08-10T10:00:00Z",
    )
    .expect("imported document");
    imported
        .advance_revision("imported-revision-one", "2026-08-10T11:00:00Z")
        .expect("advance imported document");
    assert_eq!(imported.trust().state(), OverlayTrustState::Untrusted);
    assert!(!imported.trust().is_trusted_for_revision(2));
}

#[test]
fn conflict_resolution_retains_audit_identity() {
    let mut resolved =
        OverlayConflict::new("conflict-1", "patch-1", "match-count", "instruction-hash")
            .expect("conflict");
    resolved.resolve(2).expect("resolve");

    assert_eq!(resolved.id(), "conflict-1");
    assert_eq!(resolved.mutation_id(), "patch-1");
    assert_eq!(resolved.state(), OverlayConflictState::Resolved);
    assert_eq!(resolved.resolution_revision(), Some(2));
    assert!(matches!(
        resolved.ignore(3),
        Err(SkillDomainError::InvalidOverlayTransition(_))
    ));

    let mut ignored =
        OverlayConflict::new("conflict-2", "file-1", "resource-changed", "package-hash")
            .expect("conflict");
    ignored.ignore(4).expect("ignore");
    assert_eq!(ignored.state(), OverlayConflictState::Ignored);
    assert_eq!(ignored.resolution_revision(), Some(4));
}
