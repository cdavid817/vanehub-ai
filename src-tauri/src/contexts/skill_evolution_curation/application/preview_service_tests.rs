use super::*;
use crate::contexts::skill_evolution_curation::domain::*;

struct FakeStore {
    binding: CuratorPreviewBinding,
    persisted: Vec<CuratorPreview>,
    invalidations: Vec<CuratorPreviewInvalidation>,
}

impl CuratorPreviewStore for FakeStore {
    fn preview_binding(
        &mut self,
        _: &str,
    ) -> Result<CuratorPreviewBinding, CuratorPreviewStoreError> {
        Ok(self.binding.clone())
    }

    fn persist_preview(
        &mut self,
        preview: &CuratorPreview,
    ) -> Result<u64, CuratorPreviewStoreError> {
        self.persisted.push(preview.clone());
        Ok(preview.candidate_revision)
    }

    fn invalidate_preview(
        &mut self,
        invalidation: &CuratorPreviewInvalidation,
    ) -> Result<u64, CuratorPreviewStoreError> {
        self.invalidations.push(invalidation.clone());
        Ok(invalidation.expected_candidate_revision + 1)
    }
}

struct FakeOverlay {
    result: Result<CuratorOverlayPreviewReceipt, CuratorOverlayPreviewError>,
}

impl CuratorOverlayPreviewPort for FakeOverlay {
    fn preview(
        &self,
        _: &CuratorPreviewBinding,
    ) -> Result<CuratorOverlayPreviewReceipt, CuratorOverlayPreviewError> {
        self.result.clone()
    }
}

fn binding() -> CuratorPreviewBinding {
    CuratorPreviewBinding {
        candidate_id: "candidate-1".into(),
        candidate_revision: 5,
        candidate_hash: "candidate-hash".into(),
        policy_hash: "policy-hash".into(),
        state: CuratorCandidateState::ReadyForReview,
        workspace_id: "workspace:one".into(),
        target_skill_id: "code-review".into(),
        target_revision: "effective-hash".into(),
        overlay_scope: "project".into(),
        draft_id: "draft-1".into(),
        draft_revision: 2,
        draft_hash: "draft-hash".into(),
        mutation: CuratorDraftMutationInput::LearnedGuidance {
            guidance: "Prefer bounded output.".into(),
        },
        base_instruction_hash: "base-hash".into(),
        base_package_hash: "base-package-hash".into(),
        current_effective_hash: "effective-hash".into(),
        overlay_revision: Some(4),
        pin_witness: "pin-v1:false".into(),
        trust_witness: "trust-v1:4:trusted:applied".into(),
        conflict_witness: "conflict-v1:0:false:false".into(),
        assessment_id: "assessment-1".into(),
        assessment_hash: "assessment-hash".into(),
    }
}

fn diff(from_hash: &str, to_hash: &str, labels: &[&str]) -> CuratorDiffProjection {
    CuratorDiffProjection {
        from_hash: from_hash.into(),
        to_hash: to_hash.into(),
        added_characters: labels.len(),
        removed_characters: 0,
        hunks: labels
            .iter()
            .map(|label| CuratorDiffHunk {
                label: (*label).into(),
                before: CuratorDiffText {
                    content: String::new(),
                    total_characters: 0,
                    truncated: false,
                },
                after: CuratorDiffText {
                    content: (*label).into(),
                    total_characters: label.chars().count(),
                    truncated: false,
                },
            })
            .collect(),
        complete: true,
    }
}

fn receipt() -> CuratorOverlayPreviewReceipt {
    let binding = binding();
    let proposed = "proposed-hash";
    CuratorOverlayPreviewReceipt {
        witnesses: CuratorPreviewWitnesses {
            candidate_hash: binding.candidate_hash,
            draft_hash: binding.draft_hash,
            assessment_hash: binding.assessment_hash,
            target_revision: binding.target_revision,
            base_instruction_hash: binding.base_instruction_hash,
            base_package_hash: binding.base_package_hash,
            current_effective_hash: binding.current_effective_hash,
            proposed_effective_hash: proposed.into(),
            overlay_revision: binding.overlay_revision,
            pin_witness: binding.pin_witness,
            trust_witness: binding.trust_witness,
            conflict_witness: binding.conflict_witness,
            scanner_version: "overlay-text-v1".into(),
            policy_hash: binding.policy_hash,
        },
        diffs: CuratorPreviewDiffs {
            base_to_current: diff("base-hash", "effective-hash", &["current"]),
            current_to_proposed: diff("effective-hash", proposed, &["proposal"]),
            base_to_proposed: diff("base-hash", proposed, &["combined"]),
        },
        validation: CuratorPreviewValidation {
            scan_passed: true,
            can_commit: true,
            pinned: false,
            trusted: true,
            conflict_count: 0,
            conflicts_complete: true,
            safe_rule_ids: vec!["overlay.safe".into()],
            rules_complete: true,
        },
    }
}

fn request() -> CuratorPreviewRequest<'static> {
    CuratorPreviewRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 5,
        expected_draft_revision: 2,
        expected_assessment_id: "assessment-1",
    }
}

#[test]
fn preview_binds_three_exact_diffs_and_all_review_witnesses() {
    let mut store = FakeStore {
        binding: binding(),
        persisted: vec![],
        invalidations: vec![],
    };
    let preview = CuratorPreviewService::new(
        &mut store,
        &FakeOverlay {
            result: Ok(receipt()),
        },
    )
    .create(request(), 1_000)
    .expect("preview");

    assert_eq!(preview.candidate_revision, 6);
    assert_eq!(preview.expires_at_ms, 1_000 + CURATOR_PREVIEW_TTL_MS);
    assert_eq!(preview.diffs.base_to_current.from_hash, "base-hash");
    assert_eq!(
        preview.diffs.current_to_proposed.from_hash,
        "effective-hash"
    );
    assert_eq!(preview.diffs.base_to_proposed.to_hash, "proposed-hash");
    assert_eq!(preview.witnesses.assessment_hash, "assessment-hash");
    assert_eq!(preview.witnesses.scanner_version, "overlay-text-v1");
    assert_eq!(store.persisted, vec![preview]);
}

#[test]
fn relevant_drift_invalidates_existing_preview_before_returning_error() {
    let cases = [
        ("base", CuratorStalenessReason::BaseChanged),
        ("overlay", CuratorStalenessReason::OverlayChanged),
        ("pin", CuratorStalenessReason::PinChanged),
        ("trust", CuratorStalenessReason::TrustChanged),
        ("conflict", CuratorStalenessReason::ConflictChanged),
    ];
    for (reason_code, reason) in cases {
        let mut store = FakeStore {
            binding: binding(),
            persisted: vec![],
            invalidations: vec![],
        };
        let error = CuratorPreviewService::new(
            &mut store,
            &FakeOverlay {
                result: Err(CuratorOverlayPreviewError {
                    reason_code: reason_code.into(),
                    staleness: Some(reason),
                }),
            },
        )
        .create(request(), 2_000)
        .expect_err("drift must fail");

        assert!(matches!(error, CuratorPreviewServiceError::Overlay(_)));
        assert_eq!(store.invalidations.len(), 1);
        assert_eq!(store.invalidations[0].reason, reason);
        assert!(store.persisted.is_empty());
    }
}

#[test]
fn size_and_patch_ambiguity_fail_without_inventing_staleness() {
    for reason_code in ["preview.size-limit", "preview.patch-ambiguity-or-invalid"] {
        let mut store = FakeStore {
            binding: binding(),
            persisted: vec![],
            invalidations: vec![],
        };
        let result = CuratorPreviewService::new(
            &mut store,
            &FakeOverlay {
                result: Err(CuratorOverlayPreviewError {
                    reason_code: reason_code.into(),
                    staleness: None,
                }),
            },
        )
        .create(request(), 2_000);

        assert!(matches!(
            result,
            Err(CuratorPreviewServiceError::Overlay(_))
        ));
        assert!(store.invalidations.is_empty());
    }
}

#[test]
fn pagination_and_expiry_are_explicit_and_boundary_safe() {
    let projection = diff("from", "to", &["one", "two", "three"]);
    let first = page_diff(&projection, None, 2).expect("first page");
    assert_eq!(first.hunks.len(), 2);
    assert_eq!(first.next_cursor, Some(2));
    assert!(!first.complete);
    let last = page_diff(&projection, first.next_cursor, 2).expect("last page");
    assert_eq!(last.hunks.len(), 1);
    assert_eq!(last.next_cursor, None);
    assert!(last.complete);
    assert_eq!(
        page_diff(&projection, None, 0),
        Err("preview_page_limit_invalid")
    );
    assert_eq!(
        page_diff(&projection, Some(4), 2),
        Err("preview_page_cursor_invalid")
    );

    let mut preview = CuratorPreviewService::new(
        &mut FakeStore {
            binding: binding(),
            persisted: vec![],
            invalidations: vec![],
        },
        &FakeOverlay {
            result: Ok(receipt()),
        },
    )
    .create(request(), 3_000)
    .expect("preview");
    assert!(preview.is_current(3_000));
    assert!(preview.is_current(preview.expires_at_ms - 1));
    assert!(!preview.is_current(preview.expires_at_ms));
    preview.invalidated_at_ms = Some(3_001);
    assert!(!preview.is_current(3_001));
}

#[test]
fn mismatched_witness_or_diff_endpoint_is_never_persisted() {
    let mut invalid = receipt();
    invalid.witnesses.policy_hash = "changed-policy".into();
    invalid.diffs.base_to_current.from_hash = "wrong-base".into();
    let mut store = FakeStore {
        binding: binding(),
        persisted: vec![],
        invalidations: vec![],
    };
    let result = CuratorPreviewService::new(
        &mut store,
        &FakeOverlay {
            result: Ok(invalid),
        },
    )
    .create(request(), 4_000);

    assert_eq!(
        result,
        Err(CuratorPreviewServiceError::InvalidReceipt(
            "preview_witness_mismatch"
        ))
    );
    assert!(store.persisted.is_empty());
}
