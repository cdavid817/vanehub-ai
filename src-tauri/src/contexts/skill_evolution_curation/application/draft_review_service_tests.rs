use super::*;
use crate::contexts::skill_evolution_curation::domain::*;
use std::{future::Future, pin::Pin};

#[tokio::test]
async fn safe_refinement_projects_only_a_bounded_lesson_and_becomes_approvable() {
    let mut store = FakeStore::new(binding(2, 1));
    let reviewer = FakeReviewer::new(receipt(CuratorCheckResult::Pass, None));
    let result = CuratorDraftReviewService::new(&mut store, &reviewer)
        .review_current(request(2, 1), 30)
        .await
        .unwrap_or_else(|error| panic!("safe review: {error}"));

    assert!(result.approvable);
    assert_eq!(store.persisted.len(), 1);
    let projected = reviewer
        .last_input
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
        .unwrap_or_else(|| panic!("review input"));
    assert_eq!(projected.lesson_shape.content_kinds, ["guidance"]);
    assert!(!projected
        .lesson_shape
        .required_behavior
        .contains("oldString"));
    assert!(projected.lesson_shape.required_behavior.chars().count() <= 512);
}

#[tokio::test]
async fn materially_changed_guidance_and_target_mismatch_are_blocked() {
    let mut store = FakeStore::new(binding(2, 1));
    let changed = FakeReviewer::new(receipt(
        CuratorCheckResult::Fail,
        Some(("evidence_consistency", "draft_materially_changes_lesson")),
    ));
    let result = CuratorDraftReviewService::new(&mut store, &changed)
        .review_current(request(2, 1), 30)
        .await
        .unwrap_or_else(|error| panic!("blocked review persists: {error}"));
    assert!(!result.approvable);

    let mut store = FakeStore::new(binding(2, 1));
    let mut mismatch = receipt(CuratorCheckResult::Pass, None);
    mismatch.target_skill_id = "different-skill".to_string();
    let error = CuratorDraftReviewService::new(&mut store, &FakeReviewer::new(mismatch))
        .review_current(request(2, 1), 31)
        .await
        .expect_err("target witness mismatch");
    assert_eq!(
        error,
        CuratorDraftReviewError::InvalidReceipt("draft_quality_witness_mismatch")
    );
    assert!(store.persisted.is_empty());
}

#[tokio::test]
async fn deterministic_fallback_does_not_invent_or_override_a_result() {
    let mut store = FakeStore::new(binding(2, 1));
    let mut fallback = receipt(CuratorCheckResult::Pass, None);
    fallback.model_evaluation_allowed = false;
    fallback.model_consulted = false;
    fallback.model_fallback_reason = Some("disabled_consent".to_string());
    let result = CuratorDraftReviewService::new(&mut store, &FakeReviewer::new(fallback))
        .review_current(request(2, 1), 30)
        .await
        .unwrap_or_else(|error| panic!("deterministic fallback: {error}"));
    assert!(result.approvable);
    assert!(!result.model_consulted);
    assert_eq!(
        result.model_fallback_reason.as_deref(),
        Some("disabled_consent")
    );
}

#[tokio::test]
async fn edited_draft_requires_a_new_exact_hash_assessment() {
    let reviewer = FakeReviewer::new(receipt(CuratorCheckResult::Pass, None));
    let mut first_store = FakeStore::new(binding(2, 1));
    let first = CuratorDraftReviewService::new(&mut first_store, &reviewer)
        .review_current(request(2, 1), 30)
        .await
        .unwrap_or_else(|error| panic!("first review: {error}"));

    let reviewer = FakeReviewer::new(receipt_for("draft-hash-2"));
    let mut edited_store = FakeStore::new(binding(4, 2));
    let edited = CuratorDraftReviewService::new(&mut edited_store, &reviewer)
        .review_current(request(4, 2), 40)
        .await
        .unwrap_or_else(|error| panic!("edited review: {error}"));
    assert_ne!(first.draft_hash, edited.draft_hash);
    assert_ne!(first.witness_hash, edited.witness_hash);
    assert_eq!(edited.draft_revision, 2);
}

fn request(candidate_revision: u64, draft_revision: u64) -> CuratorDraftReviewRequest<'static> {
    CuratorDraftReviewRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: candidate_revision,
        expected_draft_revision: draft_revision,
    }
}

fn binding(candidate_revision: u64, draft_revision: u64) -> CuratorDraftReviewBinding {
    CuratorDraftReviewBinding {
        candidate_id: "candidate-1".to_string(),
        candidate_revision,
        candidate_witness_hash: "candidate-witness".to_string(),
        state: CuratorCandidateState::AwaitingDraft,
        assessment_attempt_id: "attempt-1".to_string(),
        assessment_revision: "assessment-witness".to_string(),
        target_skill_id: "skill-1".to_string(),
        target_revision: "target-revision".to_string(),
        draft_id: "draft-1".to_string(),
        draft_revision,
        draft_hash: format!("draft-hash-{draft_revision}"),
        draft_kind: "learn_block".to_string(),
        rationale: "verification failure".repeat(60),
        expected_effective_change: "validate_before_action with a bounded preflight".to_string(),
        evidence_ids: vec!["evidence-1".to_string()],
        original_checks: checks(CuratorCheckResult::Pass),
        original_lesson_shape: CuratorDraftLessonShape {
            trigger: "verification_failure".to_string(),
            required_behavior: "validate_before_action".to_string(),
            prohibited_behavior: "skip_validation".to_string(),
            verification: "test_passes".to_string(),
            environment: "project".to_string(),
            content_kinds: vec!["guidance".to_string()],
        },
    }
}

fn receipt(
    result: CuratorCheckResult,
    override_check: Option<(&str, &str)>,
) -> CuratorDraftQualityReceipt {
    let mut checks = checks(CuratorCheckResult::Pass);
    if let Some((code, reason)) = override_check {
        let check = checks
            .iter_mut()
            .find(|check| check.code == code)
            .unwrap_or_else(|| panic!("known check"));
        check.result = result;
        check.reason_code = reason.to_string();
    }
    CuratorDraftQualityReceipt {
        candidate_witness_hash: "candidate-witness".to_string(),
        target_skill_id: "skill-1".to_string(),
        target_revision: "target-revision".to_string(),
        draft_hash: "draft-hash-1".to_string(),
        checks,
        deterministic_approvable: result != CuratorCheckResult::Fail,
        model_evaluation_allowed: false,
        model_consulted: false,
        model_fallback_reason: Some("disabled_consent".to_string()),
    }
}

fn receipt_for(hash: &str) -> CuratorDraftQualityReceipt {
    CuratorDraftQualityReceipt {
        draft_hash: hash.to_string(),
        ..receipt(CuratorCheckResult::Pass, None)
    }
}

fn checks(result: CuratorCheckResult) -> Vec<CuratorQualityCheck> {
    CURATOR_DRAFT_CHECK_ORDER_V1
        .iter()
        .map(|code| CuratorQualityCheck {
            code: (*code).to_string(),
            result,
            reason_code: "fixture".to_string(),
        })
        .collect()
}

struct FakeStore {
    binding: CuratorDraftReviewBinding,
    persisted: Vec<CuratorDraftAssessment>,
}

impl FakeStore {
    fn new(binding: CuratorDraftReviewBinding) -> Self {
        Self {
            binding,
            persisted: Vec::new(),
        }
    }
}

impl CuratorDraftReviewStore for FakeStore {
    fn review_binding(
        &mut self,
        _candidate_id: &str,
    ) -> Result<CuratorDraftReviewBinding, CuratorDraftReviewStoreError> {
        Ok(self.binding.clone())
    }

    fn persist_draft_assessment(
        &mut self,
        assessment: &CuratorDraftAssessment,
        _occurred_at_ms: i64,
    ) -> Result<u64, CuratorDraftReviewStoreError> {
        self.persisted.push(assessment.clone());
        Ok(assessment.candidate_revision + 1)
    }
}

struct FakeReviewer {
    receipt: CuratorDraftQualityReceipt,
    last_input: std::sync::Mutex<Option<CuratorDraftQualityInput>>,
}

impl FakeReviewer {
    fn new(receipt: CuratorDraftQualityReceipt) -> Self {
        Self {
            receipt,
            last_input: std::sync::Mutex::new(None),
        }
    }
}

impl CuratorDraftQualityPort for FakeReviewer {
    fn review<'a>(
        &'a self,
        input: &'a CuratorDraftQualityInput,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<CuratorDraftQualityReceipt, CuratorDraftQualityError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            *self
                .last_input
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = Some(input.clone());
            Ok(self.receipt.clone())
        })
    }
}
