use std::{cell::Cell, collections::BTreeSet};

use crate::contexts::skill_evolution_generation::domain::{
    GeneratedArtifactKind, GeneratedLessonShapeV1, GeneratedVerificationStepV1,
    GenerationCitationV1, GenerationValidationCheckV1, GenerationValidationStatus, MutationPlanV1,
    MutationTargetV1, StructuredDraftV1,
};

use super::{
    render_generation_artifact, validate_existing_skill_draft,
    validate_existing_skill_with_one_repair, ExistingSkillRepairPort, ExistingSkillRepairRequestV1,
    ExistingSkillValidationPort, ExistingSkillValidationRequestV1,
    GenerationOverlayPreviewReceiptV1, GenerationQualityReceiptV1, GenerationRenderRequestV1,
    GenerationSafetyReceiptV1, GENERATION_DRAFT_CHECK_ORDER_V1,
};

struct FakeValidationPort {
    privacy_passed: bool,
    duplicate: bool,
    preview_can_commit: bool,
    preview_anchor_matches: u16,
    preview_calls: Cell<u8>,
}

impl ExistingSkillValidationPort for FakeValidationPort {
    fn scan(
        &self,
        artifact: &crate::contexts::skill_evolution_generation::domain::RenderedGenerationArtifactV1,
    ) -> Result<GenerationSafetyReceiptV1, &'static str> {
        Ok(GenerationSafetyReceiptV1 {
            sanitizer_version: "sanitizer-v1".into(),
            content_hash: artifact.content_hash.clone(),
            privacy_passed: self.privacy_passed,
            injection_passed: true,
            prohibited_content_passed: true,
        })
    }

    fn is_duplicate(&self, _artifact_hash: &str) -> Result<bool, &'static str> {
        Ok(self.duplicate)
    }

    fn quality(
        &self,
        request: &ExistingSkillValidationRequestV1,
    ) -> Result<GenerationQualityReceiptV1, &'static str> {
        let vague = matches!(
            &request.draft,
            StructuredDraftV1::OverlayLearnBlock { guidance } if guidance == "vague"
        );
        let checks = GENERATION_DRAFT_CHECK_ORDER_V1
            .into_iter()
            .map(|code| GenerationValidationCheckV1 {
                code: code.into(),
                status: if vague && code == "guidance_specificity" {
                    GenerationValidationStatus::Failed
                } else {
                    GenerationValidationStatus::Passed
                },
                reason_code: (vague && code == "guidance_specificity")
                    .then(|| "vague_or_untestable_guidance".into()),
            })
            .collect();
        Ok(GenerationQualityReceiptV1 {
            artifact_hash: request.artifact.content_hash.clone(),
            checks,
            stricter_judge_passed: None,
        })
    }

    fn preview(
        &self,
        request: &ExistingSkillValidationRequestV1,
    ) -> Result<GenerationOverlayPreviewReceiptV1, &'static str> {
        self.preview_calls.set(self.preview_calls.get() + 1);
        Ok(GenerationOverlayPreviewReceiptV1 {
            artifact_hash: request.artifact.content_hash.clone(),
            target_revision: request.frozen_revision.clone(),
            overlay_witness_hash: request.frozen_overlay_witness_hash.clone(),
            exact_anchor_matches: self.preview_anchor_matches,
            unrelated_deletion: false,
            can_commit: self.preview_can_commit,
            preview_witness_hash: "sha256:preview".into(),
        })
    }
}

struct SpecificRepair;

impl ExistingSkillRepairPort for SpecificRepair {
    fn repair(
        &self,
        request: &ExistingSkillRepairRequestV1,
    ) -> Result<StructuredDraftV1, &'static str> {
        assert_eq!(request.safe_reason_codes, ["vague_or_untestable_guidance"]);
        assert!(matches!(
            request.prior_structured_draft,
            StructuredDraftV1::OverlayLearnBlock { .. }
        ));
        Ok(StructuredDraftV1::OverlayLearnBlock {
            guidance: "When a run fails, validate the bounded output before retrying.".into(),
        })
    }
}

struct FailedRepair;

impl ExistingSkillRepairPort for FailedRepair {
    fn repair(
        &self,
        _request: &ExistingSkillRepairRequestV1,
    ) -> Result<StructuredDraftV1, &'static str> {
        Err("generation_repair_provider_failed")
    }
}

#[test]
fn safe_learn_block_and_unique_contradictory_patch_pass_full_preview() {
    let port = passing_port();
    let learn = request(StructuredDraftV1::OverlayLearnBlock {
        guidance: "Validate the result before retrying.".into(),
    });
    let result = validate_existing_skill_draft(&port, &learn);
    assert_eq!(result.status, GenerationValidationStatus::Passed);
    assert_eq!(
        result.preview_witness_hash.as_deref(),
        Some("sha256:preview")
    );

    let patch = request(StructuredDraftV1::OverlayExactPatch {
        old_string: "Do not validate.".into(),
        new_string: "Validate before completing.".into(),
        replace_all: false,
    });
    let result = validate_existing_skill_draft(&port, &patch);
    assert_eq!(result.status, GenerationValidationStatus::Passed);
    assert!(result.safe_reason_codes.is_empty());
}

#[test]
fn pinned_stale_duplicate_and_hard_safety_fail_closed() {
    let mut pinned = request(StructuredDraftV1::OverlayLearnBlock {
        guidance: "Validate the result.".into(),
    });
    pinned.pinned = true;
    let pinned_result = validate_existing_skill_draft(&passing_port(), &pinned);
    assert_eq!(
        pinned_result.safe_reason_codes,
        ["generation_target_pinned"]
    );

    let mut stale = pinned.clone();
    stale.pinned = false;
    stale.current_overlay_witness_hash = "sha256:changed".into();
    let stale_port = passing_port();
    let stale_result = validate_existing_skill_draft(&stale_port, &stale);
    assert_eq!(stale_result.safe_reason_codes, ["generation_overlay_stale"]);
    assert_eq!(stale_port.preview_calls.get(), 0);

    let duplicate = FakeValidationPort {
        duplicate: true,
        ..passing_port()
    };
    let duplicate_result = validate_existing_skill_draft(&duplicate, &request_learn());
    assert_eq!(duplicate_result.safe_reason_codes, ["generation_duplicate"]);

    let unsafe_port = FakeValidationPort {
        privacy_passed: false,
        ..passing_port()
    };
    let unsafe_result = validate_existing_skill_draft(&unsafe_port, &request_learn());
    assert_eq!(
        unsafe_result.safe_reason_codes,
        ["generation_privacy_rejected"]
    );
    assert_eq!(unsafe_port.preview_calls.get(), 0);
}

#[test]
fn exact_patch_requires_one_frozen_and_preview_anchor() {
    let draft = StructuredDraftV1::OverlayExactPatch {
        old_string: "Do not validate.".into(),
        new_string: "Validate.".into(),
        replace_all: false,
    };
    let mut missing = request(draft.clone());
    missing.frozen_effective_content = "No matching instruction.".into();
    assert_eq!(
        validate_existing_skill_draft(&passing_port(), &missing).safe_reason_codes,
        ["generation_exact_anchor_invalid"]
    );

    let stale_preview = FakeValidationPort {
        preview_anchor_matches: 2,
        ..passing_port()
    };
    assert_eq!(
        validate_existing_skill_draft(&stale_preview, &request(draft)).safe_reason_codes,
        ["generation_preview_invalid"]
    );
}

#[test]
fn one_safe_repair_reruns_the_entire_pipeline_and_cannot_repeat() {
    let outcome = validate_existing_skill_with_one_repair(
        &passing_port(),
        &SpecificRepair,
        request(StructuredDraftV1::OverlayLearnBlock {
            guidance: "vague".into(),
        }),
        "artifact-repair",
        false,
    );
    assert!(outcome.repair_performed);
    assert_eq!(outcome.attempts.len(), 2);
    assert_eq!(
        outcome.attempts[0].status,
        GenerationValidationStatus::Failed
    );
    assert_eq!(
        outcome.attempts[1].status,
        GenerationValidationStatus::Passed
    );

    let exhausted = validate_existing_skill_with_one_repair(
        &passing_port(),
        &SpecificRepair,
        request(StructuredDraftV1::OverlayLearnBlock {
            guidance: "vague".into(),
        }),
        "unused",
        true,
    );
    assert!(!exhausted.repair_performed);
    assert_eq!(exhausted.attempts.len(), 1);
}

#[test]
fn failed_repair_preserves_the_original_attempt() {
    let outcome = validate_existing_skill_with_one_repair(
        &passing_port(),
        &FailedRepair,
        request(StructuredDraftV1::OverlayLearnBlock {
            guidance: "vague".into(),
        }),
        "artifact-repair",
        false,
    );
    assert!(!outcome.repair_performed);
    assert_eq!(outcome.attempts.len(), 1);
    assert_eq!(
        outcome.repair_failure_code.as_deref(),
        Some("generation_repair_provider_failed")
    );
}

fn passing_port() -> FakeValidationPort {
    FakeValidationPort {
        privacy_passed: true,
        duplicate: false,
        preview_can_commit: true,
        preview_anchor_matches: 1,
        preview_calls: Cell::new(0),
    }
}

fn request_learn() -> ExistingSkillValidationRequestV1 {
    request(StructuredDraftV1::OverlayLearnBlock {
        guidance: "Validate the result.".into(),
    })
}

fn request(draft: StructuredDraftV1) -> ExistingSkillValidationRequestV1 {
    let kind = match &draft {
        StructuredDraftV1::OverlayLearnBlock { .. } => GeneratedArtifactKind::OverlayLearnBlock,
        StructuredDraftV1::OverlayExactPatch { .. } => GeneratedArtifactKind::OverlayExactPatch,
        StructuredDraftV1::NewSkill { .. } => GeneratedArtifactKind::NewSkill,
    };
    let plan = plan(kind);
    let artifact = render_generation_artifact(&GenerationRenderRequestV1 {
        artifact_id: "artifact-one",
        expected_kind: kind,
        draft: &draft,
        allowed_built_in_tools: &BTreeSet::new(),
    })
    .expect("render");
    ExistingSkillValidationRequestV1 {
        validation_id: "validation-one".into(),
        plan,
        draft,
        artifact,
        frozen_skill_id: "review-skill".into(),
        frozen_revision: "revision-one".into(),
        overlay_scope: "project".into(),
        frozen_effective_content: "First. Do not validate. Last.".into(),
        frozen_overlay_witness_hash: "sha256:overlay".into(),
        current_overlay_witness_hash: "sha256:overlay".into(),
        registered_citations: BTreeSet::from([(
            "source_signal_inventory".into(),
            "signal-one".into(),
        )]),
        estimated_tokens: 100,
        maximum_tokens: 500,
        pinned: false,
    }
}

fn plan(kind: GeneratedArtifactKind) -> MutationPlanV1 {
    MutationPlanV1 {
        schema_version: 1,
        plan_id: "plan-one".into(),
        artifact_kind: kind,
        target: MutationTargetV1::ExistingSkill {
            skill_id: "review-skill".into(),
            effective_revision: "revision-one".into(),
            overlay_scope: "project".into(),
        },
        rationale: "Evidence supports this bounded change.".into(),
        lesson: GeneratedLessonShapeV1 {
            trigger: "When the operation completes".into(),
            action: "Validate the result".into(),
            verification: "Confirm the expected invariant".into(),
        },
        evidence_citations: [
            "lesson.trigger",
            "lesson.action",
            "lesson.verification",
            "expected_behavior",
            "verify-one",
        ]
        .into_iter()
        .map(|claim_id| GenerationCitationV1 {
            claim_id: claim_id.into(),
            dossier_section: "source_signal_inventory".into(),
            source_id: "signal-one".into(),
        })
        .collect(),
        expected_behavior: "The operation is verified.".into(),
        verification_steps: vec![GeneratedVerificationStepV1 {
            step_id: "verify-one".into(),
            action_code: "validate".into(),
            expected_code: "verified".into(),
            citation_ids: vec!["signal-one".into()],
        }],
        content_hash: "sha256:plan".into(),
    }
}
