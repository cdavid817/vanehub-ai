use super::*;
use crate::contexts::skill_evolution_curation::application::{
    CuratorApplicationStore, CuratorDecisionService, CuratorDecisionStore, CuratorDraftReviewStore,
    CuratorPreviewStore,
};
use crate::contexts::skill_evolution_curation::domain::*;
use rusqlite::Connection;
use serde_json::json;
use std::time::Duration;

fn database() -> Connection {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .expect("foreign keys");
    connection
        .execute_batch(
            "CREATE TABLE evolution_assessment_attempts (attempt_id TEXT PRIMARY KEY);
             INSERT INTO evolution_assessment_attempts VALUES ('assessment-1');",
        )
        .expect("assessment fixture");
    apply_schema(&connection).expect("curator schema");
    connection
}

fn snapshot(candidate_id: &str) -> CuratorCandidateSnapshot {
    CuratorCandidateSnapshot {
        schema_version: CURATOR_SCHEMA_VERSION_V1,
        candidate_id: candidate_id.into(),
        workspace_id: "workspace:one".into(),
        seed_id: "seed-1".into(),
        seed_revision: "seed-revision-1".into(),
        assessment_attempt_id: "assessment-1".into(),
        assessment_revision: "assessment-revision-1".into(),
        target_skill_id: "code-review".into(),
        target_revision: "target-revision-1".into(),
        overlay_scope: "project".into(),
        route: CuratorRoute::Advance,
        risk: CuratorRisk::Low,
        confidence: CuratorConfidence::High,
        evidence_ids: vec!["evidence-1".into()],
        evidence_sources: vec![CuratorEvidenceSource {
            evidence_id: "evidence-1".into(),
            evidence_revision: "evidence-revision-1".into(),
            lineage_hash: "lineage-1".into(),
        }],
        quality_checks: vec![],
        assessment_witness_hash: "sha256:assessment-one".into(),
        policy_witness_hash: "sha256:policy-one".into(),
        witness_hash: "sha256:witness-one".into(),
        state: CuratorCandidateState::Pending,
        staleness: vec![],
        revision: 1,
        created_at_ms: 10,
        updated_at_ms: 10,
    }
}

fn transition<'a>(candidate_id: &'a str, expected_revision: u64) -> CandidateTransitionRequest<'a> {
    CandidateTransitionRequest {
        candidate_id,
        expected_revision,
        transition: CuratorTransition::IntakeValidatedWithoutDraft,
        event_kind: CuratorEventKind::Intake,
        reason_code: Some("intake_validated"),
        audit: TrustedAuditContext::system(20),
    }
}

#[test]
fn candidate_cas_and_decision_idempotency_are_transactional() {
    let mut connection = database();
    let mut repository = SqliteCuratorRepository::new(&mut connection);
    assert_eq!(
        repository.insert_candidate(&snapshot("candidate-1")),
        Ok(PersistCandidateOutcome::Inserted)
    );
    assert_eq!(
        repository.transition_with_audit(&transition("candidate-1", 1)),
        Ok(2)
    );
    assert_eq!(
        repository.transition_with_audit(&transition("candidate-1", 1)),
        Err(CuratorRepositoryError::Conflict(CandidateConflict {
            current_revision: 2,
            current_state: CuratorCandidateState::AwaitingDraft,
        }))
    );

    let decision = CuratorDecision {
        decision_id: "decision-1".into(),
        candidate_id: "candidate-1".into(),
        candidate_revision: 2,
        kind: CuratorDecisionKind::Defer,
        actor_class: CuratorActorClass::LocalInteractiveUser,
        reason_code: "need_more_evidence".into(),
        note_hash: None,
        preview_hash: None,
        decided_at_ms: 30,
    };
    let input = DecisionPersistence {
        decision: &decision,
        idempotency_key: "defer-click-1",
        review_after_ms: Some(40),
    };
    assert_eq!(
        repository.persist_decision(&input),
        Ok(PersistDecisionOutcome::Inserted)
    );
    assert_eq!(
        repository.persist_decision(&input),
        Ok(PersistDecisionOutcome::Existing {
            decision_id: "decision-1".into()
        })
    );
}

#[test]
fn audit_chain_is_ordered_verifiable_and_detects_corruption() {
    let mut connection = database();
    {
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        repository
            .insert_candidate(&snapshot("candidate-1"))
            .expect("candidate");
        repository
            .transition_with_audit(&transition("candidate-1", 1))
            .expect("intake");
        repository
            .transition_with_audit(&CandidateTransitionRequest {
                candidate_id: "candidate-1",
                expected_revision: 2,
                transition: CuratorTransition::DraftBecameReady,
                event_kind: CuratorEventKind::DraftAssessed,
                reason_code: Some("checks_passed"),
                audit: TrustedAuditContext::local_interactive_user(30),
            })
            .expect("draft ready");
    }
    verify_audit_chain(&connection, "candidate-1").expect("valid chain");
    connection
        .execute_batch(
            "DROP TRIGGER evolution_curator_events_immutable;
         UPDATE evolution_curator_events SET reason_code='tampered'
         WHERE candidate_id='candidate-1' AND sequence=1;",
        )
        .expect("corrupt fixture");
    assert_eq!(
        verify_audit_chain(&connection, "candidate-1"),
        Err(AuditChainError::Corrupt)
    );
}

#[test]
fn human_decisions_are_atomic_explicit_and_idempotent() {
    let mut connection = database();
    let mut repository = SqliteCuratorRepository::new(&mut connection);
    repository
        .insert_candidate(&snapshot("candidate-1"))
        .expect("candidate");
    repository
        .transition_with_audit(&transition("candidate-1", 1))
        .expect("intake");
    let deferred = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(30),
    )
    .defer(CuratorDeferRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 2,
        idempotency_key: "defer-1",
        reason: CuratorDeferReason::NeedMoreEvidence,
        note: Some("Wait for a second bounded observation."),
        review_after_ms: Some(30 + CURATOR_MIN_DEFER_MS),
    })
    .expect("defer");
    assert_eq!(deferred.state, CuratorCandidateState::Deferred);
    let state_after_review_time: String = repository
        .connection
        .query_row(
            "SELECT state FROM evolution_curator_candidates WHERE candidate_id='candidate-1'",
            [],
            |row| row.get(0),
        )
        .expect("deferred state");
    assert_eq!(state_after_review_time, "deferred");

    let resumed = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(31 + CURATOR_MIN_DEFER_MS),
    )
    .resume(CuratorResumeRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 3,
        expected_candidate_hash: "sha256:witness-one",
        expected_policy_hash: "sha256:policy-one",
        expected_draft_revision: None,
        expected_assessment_id: None,
        idempotency_key: "resume-1",
    })
    .expect("resume");
    assert_eq!(resumed.state, CuratorCandidateState::AwaitingDraft);

    let duplicate = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(40 + CURATOR_MIN_DEFER_MS),
    )
    .defer(CuratorDeferRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 2,
        idempotency_key: "defer-1",
        reason: CuratorDeferReason::NeedMoreEvidence,
        note: None,
        review_after_ms: None,
    })
    .expect("duplicate defer");
    assert!(duplicate.duplicate);
    assert_eq!(duplicate.candidate_revision, 3);
    assert_eq!(duplicate.state, CuratorCandidateState::Deferred);
    let decision_count: i64 = repository
        .connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_curator_decisions WHERE candidate_id='candidate-1'",
            [],
            |row| row.get(0),
        )
        .expect("decision count");
    assert_eq!(decision_count, 2);
    verify_audit_chain(repository.connection, "candidate-1").expect("decision audit chain");
}

#[test]
fn unsafe_documents_never_reach_draft_storage() {
    assert_eq!(
        ValidatedDraftDocument::from_validated_value(
            CuratorDraftKind::LearnBlock,
            &json!({"guidance": "safe", "providerPayload": "secret"}),
            "scanner-v1",
        ),
        Err(SafeDocumentError::UnsafeShape)
    );
    let document = ValidatedDraftDocument::from_validated_value(
        CuratorDraftKind::LearnBlock,
        &json!({"guidance": "Verify the bounded repository result."}),
        "scanner-v1",
    )
    .expect("validated document");
    let mut connection = database();
    let mut repository = SqliteCuratorRepository::new(&mut connection);
    repository
        .insert_candidate(&snapshot("candidate-1"))
        .expect("candidate");
    repository
        .transition_with_audit(&transition("candidate-1", 1))
        .expect("intake");
    let draft = CuratorDraftRevision {
        draft_id: "draft-1".into(),
        candidate_id: "candidate-1".into(),
        revision: 1,
        kind: CuratorDraftKind::LearnBlock,
        target_skill_id: "code-review".into(),
        target_revision: "target-revision-1".into(),
        overlay_scope: "project".into(),
        body_hash: document.body_hash().into(),
        evidence_ids: vec!["evidence-1".into()],
        rationale: "Bound the repository behavior.".into(),
        expected_effective_change: "Adds bounded repository guidance.".into(),
        base_hash: "base-1".into(),
        base_package_hash: "base-package-1".into(),
        effective_hash: "effective-1".into(),
        overlay_revision: Some(1),
        pin_witness: "pin-v1:false".into(),
        trust_witness: "trust-v1:1:trusted:applied".into(),
        conflict_witness: "conflict-v1:0:false:false".into(),
        created_at_ms: 40,
    };
    assert_eq!(
        repository.persist_validated_draft(&DraftPersistence {
            draft: &draft,
            document: &document,
            expected_candidate_revision: 2,
            occurred_at_ms: 40,
        }),
        Ok(3)
    );
    let stored: String = repository
        .connection
        .query_row(
            "SELECT validated_body_json FROM evolution_curator_drafts WHERE draft_id='draft-1'",
            [],
            |row| row.get(0),
        )
        .expect("stored body");
    assert!(stored.contains("bounded repository"));
    assert!(!stored.to_ascii_lowercase().contains("providerpayload"));
    let assessment = draft_assessment(&draft);
    assert_eq!(repository.persist_draft_assessment(&assessment, 41), Ok(4));
    let preview_binding = repository
        .preview_binding("candidate-1")
        .expect("preview binding");
    let preview = preview(&preview_binding, 42);
    assert_eq!(repository.persist_preview(&preview), Ok(5));
    let decision_binding = repository
        .decision_binding("candidate-1")
        .expect("decision binding");
    let approval_preview = decision_binding.current_preview.expect("approval preview");
    assert_eq!(approval_preview.witness_hash, "preview-witness");
    assert_eq!(approval_preview.effective_diff_hash, "diff-hash");
    assert!(approval_preview.diffs_complete);
    assert!(approval_preview.validation_complete);
    let deferred = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(43),
    )
    .defer(CuratorDeferRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 5,
        idempotency_key: "ready-defer-1",
        reason: CuratorDeferReason::LowerPriority,
        note: None,
        review_after_ms: None,
    })
    .expect("defer ready candidate");
    assert_eq!(deferred.state, CuratorCandidateState::Deferred);
    let resumed = CuratorDecisionService::new(
        &mut repository,
        CuratorTrustedActor::local_interactive_user(44),
    )
    .resume(CuratorResumeRequest {
        candidate_id: "candidate-1",
        expected_candidate_revision: 6,
        expected_candidate_hash: "sha256:witness-one",
        expected_policy_hash: "sha256:policy-one",
        expected_draft_revision: Some(1),
        expected_assessment_id: Some("draft-assessment-1"),
        idempotency_key: "ready-resume-1",
    })
    .expect("resume ready candidate");
    assert_eq!(resumed.state, CuratorCandidateState::ReadyForReview);
    let revised_document = ValidatedDraftDocument::from_validated_value(
        CuratorDraftKind::LearnBlock,
        &json!({"guidance": "Verify every revised bounded result."}),
        "scanner-v1",
    )
    .expect("revised document");
    let revised = CuratorDraftRevision {
        revision: 2,
        body_hash: revised_document.body_hash().into(),
        rationale: "Refine the bounded verification behavior.".into(),
        expected_effective_change: "Replaces the current draft guidance.".into(),
        base_hash: "base-2".into(),
        base_package_hash: "base-package-2".into(),
        effective_hash: "effective-2".into(),
        overlay_revision: Some(2),
        pin_witness: "pin-v1:false".into(),
        trust_witness: "trust-v1:2:trusted:applied".into(),
        conflict_witness: "conflict-v1:0:false:false".into(),
        created_at_ms: 50,
        ..draft.clone()
    };
    assert_eq!(
        repository.persist_validated_draft(&DraftPersistence {
            draft: &revised,
            document: &revised_document,
            expected_candidate_revision: 7,
            occurred_at_ms: 50,
        }),
        Ok(8)
    );
    let (drafts, assessment_invalidated, preview_invalidated, state, revision):
        (i64, Option<i64>, Option<i64>, String, i64) = (
        repository.connection.query_row("SELECT COUNT(*) FROM evolution_curator_drafts", [], |row| row.get(0)).expect("draft count"),
        repository.connection.query_row("SELECT invalidated_at_ms FROM evolution_curator_draft_assessments", [], |row| row.get(0)).expect("assessment invalidation"),
        repository.connection.query_row("SELECT invalidated_at_ms FROM evolution_curator_previews", [], |row| row.get(0)).expect("preview invalidation"),
        repository.connection.query_row("SELECT state FROM evolution_curator_candidates WHERE candidate_id='candidate-1'", [], |row| row.get(0)).expect("candidate state"),
        repository.connection.query_row("SELECT revision FROM evolution_curator_candidates WHERE candidate_id='candidate-1'", [], |row| row.get(0)).expect("candidate revision"),
    );
    assert_eq!(
        (
            drafts,
            assessment_invalidated,
            preview_invalidated,
            state.as_str(),
            revision
        ),
        (2, Some(50), Some(43), "awaiting_draft", 8)
    );
    verify_audit_chain(repository.connection, "candidate-1").expect("draft audit chain");
}

#[test]
fn application_intent_is_atomic_idempotent_and_audit_failure_rolls_back() {
    let mut connection = database();
    prepare_ready_candidate(&mut connection);
    connection
        .execute_batch(
            "CREATE TRIGGER fail_curator_application_audit BEFORE INSERT ON evolution_curator_events
             WHEN NEW.event_kind='approved' BEGIN SELECT RAISE(ABORT, 'audit failed'); END;",
        )
        .expect("audit failure trigger");
    let intent = application_intent();
    {
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        assert_eq!(
            repository.prepare_application_intent(&intent),
            Err(crate::contexts::skill_evolution_curation::application::CuratorApplicationStoreError::Storage)
        );
    }
    let rolled_back: (String, i64, i64, i64, Option<i64>) = connection
        .query_row(
            "SELECT c.state,c.revision,
                    (SELECT COUNT(*) FROM evolution_curator_applications),
                    (SELECT COUNT(*) FROM evolution_curator_outbox),p.invalidated_at_ms
             FROM evolution_curator_candidates c
             JOIN evolution_curator_previews p ON p.preview_id=c.current_preview_id
             WHERE c.candidate_id='candidate-1'",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .expect("rolled back state");
    assert_eq!(rolled_back, ("ready_for_review".into(), 5, 0, 0, None));

    connection
        .execute_batch("DROP TRIGGER fail_curator_application_audit;")
        .expect("drop trigger");
    let mut repository = SqliteCuratorRepository::new(&mut connection);
    let prepared = repository
        .prepare_application_intent(&intent)
        .expect("prepare intent");
    assert!(!prepared.duplicate);
    assert_eq!(
        prepared.application.status,
        CuratorApplicationStatus::IntentRecorded
    );
    let existing = repository
        .existing_application(
            &intent.application_id,
            "candidate-1",
            5,
            "preview-witness",
            "diff-hash",
            None,
        )
        .expect("existing application")
        .expect("idempotent application");
    assert!(existing.duplicate);
    assert_eq!(
        repository.existing_application(
            &intent.application_id,
            "candidate-1",
            5,
            "preview-witness",
            "different-diff-hash",
            None,
        ),
        Err(crate::contexts::skill_evolution_curation::application::CuratorApplicationStoreError::Conflict)
    );
    let duplicate = repository
        .prepare_application_intent(&intent)
        .expect("duplicate intent");
    assert!(duplicate.duplicate);
    let persisted: (String, i64, i64, i64) = repository
        .connection
        .query_row(
            "SELECT c.state,c.revision,
                    (SELECT COUNT(*) FROM evolution_curator_applications),
                    (SELECT COUNT(*) FROM evolution_curator_outbox)
             FROM evolution_curator_candidates c WHERE c.candidate_id='candidate-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("persisted intent");
    assert_eq!(persisted, ("applying".into(), 6, 1, 1));
    verify_audit_chain(repository.connection, "candidate-1").expect("application audit chain");
}

#[test]
fn system_policy_authorization_is_atomic_and_part_of_idempotency_identity() {
    let mut connection = database();
    prepare_ready_candidate(&mut connection);
    let mut intent = application_intent();
    intent.application_id = "system-application-1".into();
    intent.outbox_id = "outbox-system-application-1".into();
    intent.decision.decision_id = "system-decision-1".into();
    intent.decision.actor_class = CuratorActorClass::System;
    intent.decision.reason_code = "system_policy_authorized".into();
    intent.idempotency_key = "system-reservation-1".into();
    intent.system_policy_authorization = Some(CuratorSystemPolicyAuthorizationV1 {
        run_id: "run-1".into(),
        eligibility_id: "eligibility-1".into(),
        eligibility_proof_hash: "eligibility-proof".into(),
        preflight_witness_hash: "preflight-proof".into(),
        policy_witness_hash: "policy-proof".into(),
        rate_reservation_id: "reservation-1".into(),
        authorized_at_ms: 45,
    });

    connection
        .execute_batch(
            "CREATE TRIGGER fail_system_outbox BEFORE INSERT ON evolution_curator_outbox
             BEGIN SELECT RAISE(ABORT, 'outbox failed'); END;",
        )
        .expect("failure trigger");
    {
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        assert_eq!(
            repository.prepare_application_intent(&intent),
            Err(crate::contexts::skill_evolution_curation::application::CuratorApplicationStoreError::Storage)
        );
    }
    let rolled_back: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_curator_system_policy_authorizations",
            [],
            |row| row.get(0),
        )
        .expect("authorization count");
    assert_eq!(rolled_back, 0);

    connection
        .execute_batch("DROP TRIGGER fail_system_outbox;")
        .expect("drop trigger");
    let authorization = intent
        .system_policy_authorization
        .as_ref()
        .expect("authorization")
        .clone();
    let mut repository = SqliteCuratorRepository::new(&mut connection);
    repository
        .prepare_application_intent(&intent)
        .expect("system intent");
    let actor: String = repository
        .connection
        .query_row(
            "SELECT actor FROM evolution_curator_system_policy_authorizations
             WHERE application_id='system-application-1'",
            [],
            |row| row.get(0),
        )
        .expect("stored actor");
    assert_eq!(actor, "system_policy");
    assert!(repository
        .existing_application(
            &intent.application_id,
            "candidate-1",
            5,
            "preview-witness",
            "diff-hash",
            Some(&authorization),
        )
        .expect("matching authorization")
        .is_some());
    let mut changed = authorization;
    changed.policy_witness_hash = "changed-policy-proof".into();
    assert_eq!(
        repository.existing_application(
            &intent.application_id,
            "candidate-1",
            5,
            "preview-witness",
            "diff-hash",
            Some(&changed),
        ),
        Err(crate::contexts::skill_evolution_curation::application::CuratorApplicationStoreError::Conflict)
    );
}

#[test]
fn application_finalization_records_overlay_refs_and_retry_requires_fresh_approval() {
    let mut connection = database();
    prepare_ready_candidate(&mut connection);
    let intent = application_intent();
    let mut repository = SqliteCuratorRepository::new(&mut connection);
    let prepared = repository
        .prepare_application_intent(&intent)
        .expect("prepare intent");
    let receipt =
        crate::contexts::skill_evolution_curation::domain::CuratorOverlayApplicationReceipt {
            overlay_revision: "overlay-revision-2".into(),
            overlay_history_id: "overlay-history-2".into(),
            effective_diff_hash: "proposed-effective-1".into(),
            duplicate: false,
        };
    let applied = repository
        .finalize_application(
            &intent.application_id,
            prepared.application.revision,
            Ok(&receipt),
            50,
        )
        .expect("finalize applied");
    assert_eq!(applied.status, CuratorApplicationStatus::Applied);
    assert_eq!(
        applied.overlay_revision.as_deref(),
        Some("overlay-revision-2")
    );
    assert_eq!(
        applied.overlay_history_id.as_deref(),
        Some("overlay-history-2")
    );

    let mut second_connection = database();
    prepare_ready_candidate(&mut second_connection);
    let mut second_repository = SqliteCuratorRepository::new(&mut second_connection);
    let failed_prepared = second_repository
        .prepare_application_intent(&intent)
        .expect("prepare failed intent");
    let failed = second_repository
        .finalize_application(
            &intent.application_id,
            failed_prepared.application.revision,
            Err(CuratorApplicationFailure::Pinned),
            50,
        )
        .expect("finalize failed");
    assert_eq!(failed.failure_code.as_deref(), Some("overlay_pinned"));
    let retry_revision = second_repository
        .prepare_failed_retry("candidate-1", 7, 60)
        .expect("prepare retry");
    assert_eq!(retry_revision, 8);
    let retry_state: (String, Option<String>, i64) = second_repository
        .connection
        .query_row(
            "SELECT state,current_preview_id,
                    (SELECT COUNT(*) FROM evolution_curator_previews
                     WHERE candidate_id='candidate-1' AND invalidated_at_ms IS NULL)
             FROM evolution_curator_candidates WHERE candidate_id='candidate-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("retry state");
    assert_eq!(retry_state, ("ready_for_review".into(), None, 0));
}

fn prepare_ready_candidate(connection: &mut Connection) {
    let mut repository = SqliteCuratorRepository::new(connection);
    repository
        .insert_candidate(&snapshot("candidate-1"))
        .expect("candidate");
    repository
        .transition_with_audit(&transition("candidate-1", 1))
        .expect("intake");
    let document = ValidatedDraftDocument::from_validated_value(
        CuratorDraftKind::LearnBlock,
        &json!({"guidance": "Verify the bounded repository result."}),
        "scanner-v1",
    )
    .expect("validated document");
    let draft = CuratorDraftRevision {
        draft_id: "draft-1".into(),
        candidate_id: "candidate-1".into(),
        revision: 1,
        kind: CuratorDraftKind::LearnBlock,
        target_skill_id: "code-review".into(),
        target_revision: "target-revision-1".into(),
        overlay_scope: "project".into(),
        body_hash: document.body_hash().into(),
        evidence_ids: vec!["evidence-1".into()],
        rationale: "Bound the repository behavior.".into(),
        expected_effective_change: "Adds bounded repository guidance.".into(),
        base_hash: "base-1".into(),
        base_package_hash: "base-package-1".into(),
        effective_hash: "effective-1".into(),
        overlay_revision: Some(1),
        pin_witness: "pin-v1:false".into(),
        trust_witness: "trust-v1:1:trusted:applied".into(),
        conflict_witness: "conflict-v1:0:false:false".into(),
        created_at_ms: 40,
    };
    repository
        .persist_validated_draft(&DraftPersistence {
            draft: &draft,
            document: &document,
            expected_candidate_revision: 2,
            occurred_at_ms: 40,
        })
        .expect("draft");
    repository
        .persist_draft_assessment(&draft_assessment(&draft), 41)
        .expect("assessment");
    let binding = repository
        .preview_binding("candidate-1")
        .expect("preview binding");
    repository
        .persist_preview(&preview(&binding, 42))
        .expect("preview");
}

fn application_intent() -> CuratorApplicationIntent {
    CuratorApplicationIntent {
        application_id: "application-1".into(),
        outbox_id: "outbox-application-1".into(),
        decision: CuratorDecision {
            decision_id: "approval-decision-1".into(),
            candidate_id: "candidate-1".into(),
            candidate_revision: 5,
            kind: CuratorDecisionKind::Approve,
            actor_class: CuratorActorClass::LocalInteractiveUser,
            reason_code: "explicit_preview_approval".into(),
            note_hash: None,
            preview_hash: Some("preview-witness".into()),
            decided_at_ms: 45,
        },
        idempotency_key: "approval-click-1".into(),
        approved_witness_hash: "preview-witness".into(),
        approved_diff_hash: "diff-hash".into(),
        expected_effective_hash: "proposed-effective-1".into(),
        expected_state: CuratorCandidateState::ReadyForReview,
        system_policy_authorization: None,
    }
}

fn preview(binding: &CuratorPreviewBinding, issued_at_ms: i64) -> CuratorPreview {
    let proposed_hash = "proposed-effective-1";
    CuratorPreview {
        preview_id: "preview-1".into(),
        candidate_id: binding.candidate_id.clone(),
        candidate_revision: binding.candidate_revision + 1,
        draft_id: binding.draft_id.clone(),
        draft_revision: binding.draft_revision,
        assessment_id: binding.assessment_id.clone(),
        witness_hash: "preview-witness".into(),
        effective_diff_hash: "diff-hash".into(),
        witnesses: CuratorPreviewWitnesses {
            candidate_hash: binding.candidate_hash.clone(),
            draft_hash: binding.draft_hash.clone(),
            assessment_hash: binding.assessment_hash.clone(),
            target_revision: binding.target_revision.clone(),
            base_instruction_hash: binding.base_instruction_hash.clone(),
            base_package_hash: binding.base_package_hash.clone(),
            current_effective_hash: binding.current_effective_hash.clone(),
            proposed_effective_hash: proposed_hash.into(),
            overlay_revision: binding.overlay_revision,
            pin_witness: binding.pin_witness.clone(),
            trust_witness: binding.trust_witness.clone(),
            conflict_witness: binding.conflict_witness.clone(),
            scanner_version: "overlay-text-v1".into(),
            policy_hash: binding.policy_hash.clone(),
        },
        diffs: CuratorPreviewDiffs {
            base_to_current: empty_diff(
                &binding.base_instruction_hash,
                &binding.current_effective_hash,
            ),
            current_to_proposed: empty_diff(&binding.current_effective_hash, proposed_hash),
            base_to_proposed: empty_diff(&binding.base_instruction_hash, proposed_hash),
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
        issued_at_ms,
        expires_at_ms: issued_at_ms + CURATOR_PREVIEW_TTL_MS,
        invalidated_at_ms: None,
    }
}

fn empty_diff(from_hash: &str, to_hash: &str) -> CuratorDiffProjection {
    CuratorDiffProjection {
        from_hash: from_hash.into(),
        to_hash: to_hash.into(),
        added_characters: 0,
        removed_characters: 0,
        hunks: vec![],
        complete: true,
    }
}

fn draft_assessment(draft: &CuratorDraftRevision) -> CuratorDraftAssessment {
    CuratorDraftAssessment {
        assessment_id: "draft-assessment-1".into(),
        candidate_id: draft.candidate_id.clone(),
        candidate_revision: 3,
        draft_id: draft.draft_id.clone(),
        draft_revision: draft.revision,
        draft_hash: draft.body_hash.clone(),
        candidate_witness_hash: "sha256:witness-one".into(),
        target_skill_id: draft.target_skill_id.clone(),
        target_revision: draft.target_revision.clone(),
        checks: CURATOR_DRAFT_CHECK_ORDER_V1
            .iter()
            .map(|code| CuratorQualityCheck {
                code: (*code).into(),
                result: CuratorCheckResult::Pass,
                reason_code: "fixture_pass".into(),
            })
            .collect(),
        approvable: true,
        model_evaluation_allowed: false,
        model_consulted: false,
        model_fallback_reason: Some("disabled_consent".into()),
        witness_hash: "draft-assessment-witness-1".into(),
    }
}

#[test]
fn candidate_and_sources_roll_back_together_on_failure() {
    let mut connection = database();
    let mut invalid = snapshot("candidate-rollback");
    invalid.evidence_ids = vec!["duplicate".into(), "duplicate".into()];
    invalid.evidence_sources = vec![
        CuratorEvidenceSource {
            evidence_id: "duplicate".into(),
            evidence_revision: "revision-1".into(),
            lineage_hash: "lineage-1".into(),
        },
        CuratorEvidenceSource {
            evidence_id: "duplicate".into(),
            evidence_revision: "revision-1".into(),
            lineage_hash: "lineage-1".into(),
        },
    ];
    let mut repository = SqliteCuratorRepository::new(&mut connection);
    assert_eq!(
        repository.insert_candidate(&invalid),
        Err(CuratorRepositoryError::Storage)
    );
    let count: i64 = connection.query_row(
        "SELECT COUNT(*) FROM evolution_curator_candidates WHERE candidate_id='candidate-rollback'",
        [], |row| row.get(0),
    ).expect("candidate count");
    assert_eq!(count, 0);
}

#[test]
fn audit_failure_rolls_back_the_candidate_transition() {
    let mut connection = database();
    {
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        repository
            .insert_candidate(&snapshot("candidate-1"))
            .expect("candidate");
    }
    connection
        .execute_batch(
            "CREATE TRIGGER fail_curator_audit BEFORE INSERT ON evolution_curator_events
         BEGIN SELECT RAISE(ABORT, 'fixture audit failure'); END;",
        )
        .expect("failure trigger");
    let outcome = SqliteCuratorRepository::new(&mut connection)
        .transition_with_audit(&transition("candidate-1", 1));
    assert_eq!(outcome, Err(CuratorRepositoryError::Storage));
    let state: (String, i64) = connection.query_row(
        "SELECT state,revision FROM evolution_curator_candidates WHERE candidate_id='candidate-1'",
        [], |row| Ok((row.get(0)?, row.get(1)?)),
    ).expect("candidate state");
    assert_eq!(state, ("pending".into(), 1));
}

#[test]
fn concurrent_candidate_mutations_allow_one_revision_winner() {
    let directory = crate::test_support::TempDirectory::new("curator-concurrent-cas");
    let path = directory.path().join("curator.sqlite");
    {
        let mut connection = Connection::open(&path).expect("database");
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .expect("foreign keys");
        connection
            .execute_batch(
                "CREATE TABLE evolution_assessment_attempts (attempt_id TEXT PRIMARY KEY);
             INSERT INTO evolution_assessment_attempts VALUES ('assessment-1');",
            )
            .expect("assessment fixture");
        apply_schema(&connection).expect("schema");
        SqliteCuratorRepository::new(&mut connection)
            .insert_candidate(&snapshot("candidate-1"))
            .expect("candidate");
    }
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles = [(), ()].map(|()| {
        let path = path.clone();
        let barrier = barrier.clone();
        std::thread::spawn(move || {
            let mut connection = Connection::open(path).expect("thread database");
            connection
                .busy_timeout(Duration::from_secs(5))
                .expect("busy timeout");
            connection
                .pragma_update(None, "foreign_keys", "ON")
                .expect("foreign keys");
            barrier.wait();
            SqliteCuratorRepository::new(&mut connection)
                .transition_with_audit(&transition("candidate-1", 1))
        })
    });
    let outcomes = handles.map(|handle| handle.join().expect("mutation thread"));
    assert_eq!(outcomes.iter().filter(|outcome| outcome.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Err(CuratorRepositoryError::Conflict(_))))
            .count(),
        1
    );
}
