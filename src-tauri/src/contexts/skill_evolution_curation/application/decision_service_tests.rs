use super::*;
use crate::contexts::skill_evolution_curation::domain::*;

struct FakeStore {
    binding: CuratorDecisionBinding,
    existing: Option<CuratorDecisionOutcome>,
    mutations: Vec<StoredMutation>,
    invalidations: Vec<CuratorPreviewInvalidation>,
}

#[derive(Clone)]
struct StoredMutation {
    decision: CuratorDecision,
    review_after_ms: Option<i64>,
}

impl CuratorDecisionStore for FakeStore {
    fn existing_decision(
        &mut self,
        _: &str,
        _: CuratorDecisionKind,
        _: &str,
    ) -> Result<Option<CuratorDecisionOutcome>, CuratorDecisionStoreError> {
        Ok(self.existing.clone())
    }

    fn decision_binding(
        &mut self,
        _: &str,
    ) -> Result<CuratorDecisionBinding, CuratorDecisionStoreError> {
        Ok(self.binding.clone())
    }

    fn persist_decision_mutation(
        &mut self,
        mutation: &CuratorDecisionMutation<'_>,
    ) -> Result<CuratorDecisionOutcome, CuratorDecisionStoreError> {
        let state = transition_candidate(mutation.expected_state, mutation.transition)
            .map_err(|_| CuratorDecisionStoreError::Conflict)?;
        self.mutations.push(StoredMutation {
            decision: mutation.decision.clone(),
            review_after_ms: mutation.review_after_ms,
        });
        Ok(CuratorDecisionOutcome {
            decision_id: mutation.decision.decision_id.clone(),
            candidate_revision: mutation.decision.candidate_revision + 1,
            state,
            duplicate: false,
        })
    }
}

impl CuratorPreviewStore for FakeStore {
    fn preview_binding(
        &mut self,
        _: &str,
    ) -> Result<CuratorPreviewBinding, CuratorPreviewStoreError> {
        Err(CuratorPreviewStoreError::NotFound)
    }

    fn persist_preview(&mut self, _: &CuratorPreview) -> Result<u64, CuratorPreviewStoreError> {
        Err(CuratorPreviewStoreError::InvalidInput)
    }

    fn invalidate_preview(
        &mut self,
        invalidation: &CuratorPreviewInvalidation,
    ) -> Result<u64, CuratorPreviewStoreError> {
        self.invalidations.push(invalidation.clone());
        Ok(invalidation.expected_candidate_revision + 1)
    }
}

fn binding(state: CuratorCandidateState) -> CuratorDecisionBinding {
    CuratorDecisionBinding {
        candidate_id: "candidate-1".into(),
        candidate_revision: 6,
        candidate_hash: "candidate-hash".into(),
        policy_hash: "policy-hash".into(),
        maximum_defer_days: 180,
        state,
        staleness: vec![],
        ready_draft: Some(CuratorReadyDraftWitness {
            draft_revision: 2,
            assessment_id: "assessment-1".into(),
        }),
        current_preview: Some(CuratorApprovalPreviewWitness {
            preview_id: "preview-1".into(),
            witness_hash: "preview-hash".into(),
            effective_diff_hash: "diff-hash".into(),
            draft_revision: 2,
            assessment_id: "assessment-1".into(),
            issued_at_ms: 1_000,
            expires_at_ms: 1_000 + CURATOR_PREVIEW_TTL_MS,
            diffs_complete: true,
            validation_complete: true,
        }),
    }
}

fn store(state: CuratorCandidateState) -> FakeStore {
    FakeStore {
        binding: binding(state),
        existing: None,
        mutations: vec![],
        invalidations: vec![],
    }
}

#[test]
fn trusted_actor_is_derived_outside_requests_and_system_cannot_decide() {
    let mut repository = store(CuratorCandidateState::ReadyForReview);
    let result = CuratorDecisionService::new(&mut repository, CuratorTrustedActor::system(2_000))
        .reject(CuratorRejectRequest {
            candidate_id: "candidate-1",
            expected_candidate_revision: 6,
            idempotency_key: "reject-1",
            reason: CuratorRejectionReason::TooRisky,
            note: None,
        });

    assert_eq!(result, Err(CuratorDecisionServiceError::Unauthorized));
    assert!(repository.mutations.is_empty());
}

#[test]
fn rejection_is_terminal_and_persists_only_a_sanitized_note_hash() {
    let mut repository = store(CuratorCandidateState::ReadyForReview);
    let result = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(2_000),
    )
    .reject(CuratorRejectRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        idempotency_key: "reject-1",
        reason: CuratorRejectionReason::IncorrectTarget,
        note: Some("  Private reviewer note  "),
    })
    .expect("reject");

    assert_eq!(result.state, CuratorCandidateState::Rejected);
    assert_eq!(repository.mutations.len(), 1);
    let stored = &repository.mutations[0].decision;
    assert_eq!(stored.actor_class, CuratorActorClass::LocalInteractiveUser);
    assert_eq!(stored.decided_at_ms, 2_000);
    assert_eq!(stored.reason_code, "incorrect_target");
    assert!(stored.note_hash.as_deref().is_some_and(|hash| {
        hash.starts_with("sha256:") && !hash.contains("Private reviewer note")
    }));

    repository.binding.state = CuratorCandidateState::Rejected;
    repository.existing = None;
    let repeated = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(2_001),
    )
    .reject(CuratorRejectRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        idempotency_key: "reject-another",
        reason: CuratorRejectionReason::Other,
        note: None,
    });
    assert_eq!(repeated, Err(CuratorDecisionServiceError::InvalidState));
}

#[test]
fn defer_bounds_are_enforced_and_resume_requires_exact_current_witnesses() {
    let now = 2_000;
    let mut repository = store(CuratorCandidateState::ReadyForReview);
    let too_soon = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(now),
    )
    .defer(CuratorDeferRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        idempotency_key: "defer-early",
        reason: CuratorDeferReason::WaitingForChange,
        note: None,
        review_after_ms: Some(now + CURATOR_MIN_DEFER_MS - 1),
    });
    assert_eq!(
        too_soon,
        Err(CuratorDecisionServiceError::InvalidInput(
            "defer_time_out_of_range"
        ))
    );

    repository.binding.maximum_defer_days = 30;
    let beyond_policy = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(now),
    )
    .defer(CuratorDeferRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        idempotency_key: "defer-beyond-policy",
        reason: CuratorDeferReason::WaitingForChange,
        note: None,
        review_after_ms: Some(now + 31 * CURATOR_MIN_DEFER_MS),
    });
    assert_eq!(
        beyond_policy,
        Err(CuratorDecisionServiceError::InvalidInput(
            "defer_time_out_of_range"
        ))
    );
    repository.binding.maximum_defer_days = 180;

    let result = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(now),
    )
    .defer(CuratorDeferRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        idempotency_key: "defer-1",
        reason: CuratorDeferReason::WaitingForChange,
        note: None,
        review_after_ms: Some(now + CURATOR_MAX_DEFER_MS),
    })
    .expect("defer");
    assert_eq!(result.state, CuratorCandidateState::Deferred);
    assert_eq!(
        repository.mutations[0].review_after_ms,
        Some(now + CURATOR_MAX_DEFER_MS)
    );

    repository.binding.state = CuratorCandidateState::Deferred;
    let stale = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(now + 1),
    )
    .resume(CuratorResumeRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        expected_candidate_hash: "forged-hash",
        expected_policy_hash: "policy-hash",
        expected_draft_revision: Some(2),
        expected_assessment_id: Some("assessment-1"),
        idempotency_key: "resume-stale",
    });
    assert_eq!(stale, Err(CuratorDecisionServiceError::Conflict));

    let resumed = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(now + 1),
    )
    .resume(CuratorResumeRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        expected_candidate_hash: "candidate-hash",
        expected_policy_hash: "policy-hash",
        expected_draft_revision: Some(2),
        expected_assessment_id: Some("assessment-1"),
        idempotency_key: "resume-1",
    })
    .expect("resume");
    assert_eq!(resumed.state, CuratorCandidateState::ReadyForReview);
}

#[test]
fn resume_without_current_ready_draft_returns_to_awaiting_draft() {
    let mut repository = store(CuratorCandidateState::Deferred);
    repository.binding.ready_draft = None;
    let resumed = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(2_000),
    )
    .resume(CuratorResumeRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        expected_candidate_hash: "candidate-hash",
        expected_policy_hash: "policy-hash",
        expected_draft_revision: None,
        expected_assessment_id: None,
        idempotency_key: "resume-1",
    })
    .expect("resume");
    assert_eq!(resumed.state, CuratorCandidateState::AwaitingDraft);
}

#[test]
fn approval_requires_exact_complete_current_preview_and_marks_web_mock_non_native() {
    let request = CuratorApprovalRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        confirmed_preview_hash: "preview-hash",
        confirmed_effective_diff_hash: "diff-hash",
        idempotency_key: "approve-1",
    };
    let mut repository = store(CuratorCandidateState::ReadyForReview);
    let native = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(2_000),
    )
    .authorize_approval(request)
    .expect("native approval");
    assert!(native.native_application_allowed);

    let mock = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::web_mock_interactive_user(2_000),
    )
    .authorize_approval(request)
    .expect("mock approval");
    assert_eq!(mock.actor_class, CuratorActorClass::WebMockInteractiveUser);
    assert!(!mock.native_application_allowed);

    repository
        .binding
        .current_preview
        .as_mut()
        .expect("preview")
        .diffs_complete = false;
    let incomplete = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(2_000),
    )
    .authorize_approval(request);
    assert_eq!(
        incomplete,
        Err(CuratorDecisionServiceError::PreviewIncomplete)
    );
}

#[test]
fn expired_preview_is_invalidated_and_mismatch_never_authorizes() {
    let mut repository = store(CuratorCandidateState::ReadyForReview);
    let mismatch = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(2_000),
    )
    .authorize_approval(CuratorApprovalRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        confirmed_preview_hash: "wrong",
        confirmed_effective_diff_hash: "diff-hash",
        idempotency_key: "approve-mismatch",
    });
    assert_eq!(mismatch, Err(CuratorDecisionServiceError::PreviewMismatch));

    let expires_at = repository
        .binding
        .current_preview
        .as_ref()
        .expect("preview")
        .expires_at_ms;
    let expired = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(expires_at),
    )
    .authorize_approval(CuratorApprovalRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        confirmed_preview_hash: "preview-hash",
        confirmed_effective_diff_hash: "diff-hash",
        idempotency_key: "approve-expired",
    });
    assert_eq!(expired, Err(CuratorDecisionServiceError::PreviewExpired));
    assert_eq!(repository.invalidations.len(), 1);
    assert_eq!(
        repository.invalidations[0].reason,
        CuratorStalenessReason::PreviewExpired
    );
}

#[test]
fn approval_rejects_missing_and_stale_current_witnesses() {
    let request = CuratorApprovalRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        confirmed_preview_hash: "preview-hash",
        confirmed_effective_diff_hash: "diff-hash",
        idempotency_key: "approve-1",
    };
    let mut repository = store(CuratorCandidateState::ReadyForReview);
    repository.binding.current_preview = None;
    let missing = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(2_000),
    )
    .authorize_approval(request);
    assert_eq!(missing, Err(CuratorDecisionServiceError::PreviewMissing));

    repository.binding.current_preview =
        binding(CuratorCandidateState::ReadyForReview).current_preview;
    repository.binding.staleness = vec![CuratorStalenessReason::PolicyChanged];
    let stale = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(2_000),
    )
    .authorize_approval(request);
    assert_eq!(stale, Err(CuratorDecisionServiceError::Conflict));
}

#[test]
fn duplicate_action_returns_original_outcome_even_after_terminal_transition() {
    let expected = CuratorDecisionOutcome {
        decision_id: "decision-existing".into(),
        candidate_revision: 7,
        state: CuratorCandidateState::Rejected,
        duplicate: true,
    };
    let mut repository = store(CuratorCandidateState::Rejected);
    repository.existing = Some(expected.clone());
    let result = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(2_000),
    )
    .reject(CuratorRejectRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        idempotency_key: "reject-1",
        reason: CuratorRejectionReason::TooRisky,
        note: None,
    })
    .expect("idempotent retry");
    assert_eq!(result, expected);
    assert!(repository.mutations.is_empty());
}
