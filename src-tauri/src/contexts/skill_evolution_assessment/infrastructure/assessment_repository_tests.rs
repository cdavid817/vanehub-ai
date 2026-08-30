use super::*;
use crate::contexts::skill_evolution_assessment::domain::*;
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

#[test]
fn completed_assessment_is_transactional_normalized_and_prompt_free() {
    let (_directory, database) = database_with_seed("assessment-repository");
    let repository = SqliteAssessmentRepository::new(database.clone());
    let witness = witness();
    let output = output();
    let routing = routing();
    let model_calls = vec![AssessmentModelCallRecord {
        model_call_id: "call-1".to_string(),
        stage: "quality_judge".to_string(),
        request_projection_hash: "request-hash".to_string(),
        profile_id: Some("profile-1".to_string()),
        provider_protocol: Some("openai-compatible".to_string()),
        model_id: Some("model-1".to_string()),
        template_version: "template-v1".to_string(),
        response_schema_version: "schema-v1".to_string(),
        outcome_category: "valid".to_string(),
        sanitized_response_json: Some("{\"risk\":\"low\"}".to_string()),
        input_tokens: Some(10),
        output_tokens: Some(5),
        latency_ms: 20,
    }];
    let record = PersistCompletedAssessment {
        witness: &witness,
        output: &output,
        routing: &routing,
        model_calls: &model_calls,
        model_evaluation_allowed: true,
        created_at_ms: 10,
        completed_at_ms: 20,
    };

    assert_eq!(
        repository.persist_completed(&record),
        Ok(PersistAssessmentOutcome::Inserted {
            attempt_id: "attempt-1".to_string()
        })
    );
    assert_eq!(
        repository.persist_completed(&record),
        Ok(PersistAssessmentOutcome::Coalesced {
            attempt_id: "attempt-1".to_string()
        })
    );
    let connection = database
        .connection()
        .unwrap_or_else(|error| panic!("connection: {error}"));
    for (table, expected) in [
        ("evolution_assessment_attempts", 1),
        ("evolution_assessment_targets", 1),
        ("evolution_assessment_score_components", 5),
        ("evolution_assessment_checks", 9),
        ("evolution_assessment_evidence_links", 1),
        ("evolution_assessment_model_calls", 1),
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|error| panic!("count {table}: {error}"));
        assert_eq!(count, expected, "table {table}");
    }
    let columns = table_columns(&connection, "evolution_assessment_model_calls");
    assert!(!columns.iter().any(|column| column.contains("prompt")));
    assert!(!columns.iter().any(|column| column.contains("payload")));
}

#[test]
fn concurrent_identical_witnesses_coalesce_to_one_attempt() {
    use std::sync::{Arc, Barrier};

    let (_directory, database) = database_with_seed("assessment-repository-concurrent");
    let barrier = Arc::new(Barrier::new(2));
    let handles = ["attempt-a", "attempt-b"].map(|attempt_id| {
        let repository = SqliteAssessmentRepository::new(database.clone());
        let barrier = Arc::clone(&barrier);
        std::thread::spawn(move || {
            let witness = witness();
            let mut output = output();
            output.attempt_id = attempt_id.to_string();
            let routing = routing();
            let record = PersistCompletedAssessment {
                witness: &witness,
                output: &output,
                routing: &routing,
                model_calls: &[],
                model_evaluation_allowed: false,
                created_at_ms: 10,
                completed_at_ms: 20,
            };
            barrier.wait();
            repository.persist_completed(&record)
        })
    });
    let outcomes = handles.map(|handle| handle.join().expect("assessment thread"));
    assert!(outcomes.iter().all(Result::is_ok));

    let connection = database.connection().expect("connection");
    let attempts: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_assessment_attempts",
            [],
            |row| row.get(0),
        )
        .expect("attempt count");
    assert_eq!(attempts, 1);
    let persisted_id: String = connection
        .query_row(
            "SELECT attempt_id FROM evolution_assessment_attempts",
            [],
            |row| row.get(0),
        )
        .expect("persisted attempt");
    assert!(outcomes.into_iter().any(|outcome| {
        outcome
            == Ok(PersistAssessmentOutcome::Coalesced {
                attempt_id: persisted_id.clone(),
            })
    }));
}

#[test]
fn attempt_leases_heartbeat_recover_and_keep_completed_attempts_immutable() {
    let (_directory, database) = database_with_seed("assessment-attempt-leases");
    let lease_repository = SqliteAttemptLeaseRepository::new(database.clone());
    let witness = witness();
    let pending = PendingAssessmentAttempt {
        attempt_id: "attempt-pending",
        witness: &witness,
        model_evaluation_allowed: false,
        created_at_ms: 10,
    };
    assert_eq!(
        lease_repository.create_pending(&pending),
        Ok(PendingAttemptOutcome::Created {
            attempt_id: "attempt-pending".to_string()
        })
    );
    assert_eq!(
        lease_repository.create_pending(&pending),
        Ok(PendingAttemptOutcome::Coalesced {
            attempt_id: "attempt-pending".to_string()
        })
    );
    assert_eq!(
        lease_repository.claim("attempt-pending", "worker-a", 20, 100),
        Ok(AttemptLease {
            attempt_id: "attempt-pending".to_string(),
            owner: "worker-a".to_string(),
            expires_at_ms: 120,
        })
    );
    assert_eq!(
        lease_repository.heartbeat("attempt-pending", "worker-b", 30, 100),
        Err(AttemptLeaseError::LeaseUnavailable)
    );
    assert_eq!(
        lease_repository.heartbeat("attempt-pending", "worker-a", 30, 100),
        Ok(AttemptLease {
            attempt_id: "attempt-pending".to_string(),
            owner: "worker-a".to_string(),
            expires_at_ms: 130,
        })
    );
    assert_eq!(lease_repository.recover_expired(129), Ok(0));
    assert_eq!(lease_repository.recover_expired(130), Ok(1));
    assert_eq!(
        lease_repository.claim("attempt-pending", "worker-b", 131, 100),
        Ok(AttemptLease {
            attempt_id: "attempt-pending".to_string(),
            owner: "worker-b".to_string(),
            expires_at_ms: 231,
        })
    );

    let repository = SqliteAssessmentRepository::new(database.clone());
    let mut completed_output = output();
    completed_output.attempt_id = "attempt-pending".to_string();
    let routing = routing();
    let completed = PersistCompletedAssessment {
        witness: &witness,
        output: &completed_output,
        routing: &routing,
        model_calls: &[],
        model_evaluation_allowed: false,
        created_at_ms: 10,
        completed_at_ms: 140,
    };
    assert_eq!(
        repository.complete_leased(&completed, "worker-a"),
        Err(AssessmentRepositoryError::LeaseUnavailable)
    );
    assert_eq!(
        repository.complete_leased(&completed, "worker-b"),
        Ok(PersistAssessmentOutcome::Inserted {
            attempt_id: "attempt-pending".to_string()
        })
    );
    assert_eq!(
        repository.complete_leased(&completed, "worker-b"),
        Ok(PersistAssessmentOutcome::Coalesced {
            attempt_id: "attempt-pending".to_string()
        })
    );
    assert_eq!(
        lease_repository.claim("attempt-pending", "worker-c", 150, 100),
        Err(AttemptLeaseError::Immutable)
    );
    assert_eq!(
        lease_repository.heartbeat("attempt-pending", "worker-b", 150, 100),
        Err(AttemptLeaseError::Immutable)
    );
    assert_eq!(lease_repository.recover_expired(1_000), Ok(0));
}

#[test]
fn stale_witness_supersedes_attempt_and_queues_one_replacement() {
    use super::supersession_repository::stale_reason;

    let (_directory, database) = database_with_seed("assessment-supersession");
    let lease_repository = SqliteAttemptLeaseRepository::new(database.clone());
    let original = witness();
    lease_repository
        .create_pending(&PendingAssessmentAttempt {
            attempt_id: "attempt-original",
            witness: &original,
            model_evaluation_allowed: true,
            created_at_ms: 10,
        })
        .expect("pending attempt");
    let mut current = original.clone();
    current.targets[0].lifecycle = TargetLifecycle::Archived;
    let request = WitnessRecheck {
        prior_attempt_id: "attempt-original",
        replacement_attempt_id: "attempt-replacement",
        original_witness: &original,
        current_witness: &current,
        model_evaluation_allowed: false,
        checked_at_ms: 20,
    };
    let expected = WitnessRecheckOutcome::Superseded {
        replacement_attempt_id: "attempt-replacement".to_string(),
        reason_code: "target_lifecycle_changed".to_string(),
    };
    let repository = SqliteSupersessionRepository::new(database.clone());
    assert_eq!(repository.recheck(&request), Ok(expected.clone()));
    assert_eq!(repository.recheck(&request), Ok(expected));

    let connection = database.connection().expect("connection");
    let attempts: Vec<(String, String, i64)> = connection
        .prepare(
            "SELECT attempt_id,status,is_current FROM evolution_assessment_attempts ORDER BY attempt_id",
        )
        .expect("prepare attempts")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .expect("query attempts")
        .collect::<Result<_, _>>()
        .expect("collect attempts");
    assert_eq!(
        attempts,
        vec![
            ("attempt-original".to_string(), "superseded".to_string(), 0),
            ("attempt-replacement".to_string(), "pending".to_string(), 1),
        ]
    );
    let links: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_assessment_supersessions",
            [],
            |row| row.get(0),
        )
        .expect("link count");
    assert_eq!(links, 1);

    let mut changed = original.clone();
    changed.input.seed_revision = "revision-2".to_string();
    assert_eq!(stale_reason(&original, &changed), "seed_witness_changed");
    changed = original.clone();
    changed.targets[0].revision_hash = "r2".to_string();
    assert_eq!(stale_reason(&original, &changed), "target_revision_changed");
    changed = original.clone();
    changed.consent_version = "assessment-disclosure-v2".to_string();
    assert_eq!(stale_reason(&original, &changed), "consent_changed");
    changed = original.clone();
    changed.gate_policy_version = "gates-v2".to_string();
    assert_eq!(stale_reason(&original, &changed), "policy_changed");
}

#[test]
fn evidence_seed_retention_cascades_assessment_and_cannot_resurrect_lineage() {
    let (_directory, database) = database_with_seed("assessment-retention-cascade");
    let witness = witness();
    let output = output();
    let routing = routing();
    let calls = vec![AssessmentModelCallRecord {
        model_call_id: "retention-call".to_string(),
        stage: "quality_judge".to_string(),
        request_projection_hash: "projection".to_string(),
        profile_id: None,
        provider_protocol: None,
        model_id: None,
        template_version: "template-v1".to_string(),
        response_schema_version: "schema-v1".to_string(),
        outcome_category: "fallback".to_string(),
        sanitized_response_json: None,
        input_tokens: None,
        output_tokens: None,
        latency_ms: 1,
    }];
    SqliteAssessmentRepository::new(database.clone())
        .persist_completed(&PersistCompletedAssessment {
            witness: &witness,
            output: &output,
            routing: &routing,
            model_calls: &calls,
            model_evaluation_allowed: true,
            created_at_ms: 10,
            completed_at_ms: 20,
        })
        .expect("completed assessment");
    let connection = database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO evolution_assessment_queue_state \
             (queue_id,seed_id,witness_hash,lane,status,priority,available_at_ms,created_at_ms,updated_at_ms) \
             VALUES ('queue-1','seed-1','witness','deterministic','queued',1,10,10,10)",
            [],
        )
        .expect("queue row");
    connection
        .execute(
            "DELETE FROM evolution_candidate_seeds WHERE seed_id='seed-1'",
            [],
        )
        .expect("evidence retention delete");
    for table in [
        "evolution_assessment_attempts",
        "evolution_assessment_targets",
        "evolution_assessment_score_components",
        "evolution_assessment_checks",
        "evolution_assessment_evidence_links",
        "evolution_assessment_model_calls",
        "evolution_assessment_supersessions",
        "evolution_assessment_queue_state",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("cascade count");
        assert_eq!(count, 0, "table {table}");
    }
    drop(connection);

    assert_eq!(
        SqliteAttemptLeaseRepository::new(database).create_pending(&PendingAssessmentAttempt {
            attempt_id: "resurrected-attempt",
            witness: &witness,
            model_evaluation_allowed: false,
            created_at_ms: 30,
        }),
        Err(AttemptLeaseError::LineageUnavailable)
    );
}

#[test]
fn database_failure_rolls_back_the_complete_assessment_graph() {
    let (_directory, database) = database_with_seed("assessment-database-failure");
    let connection = database.connection().expect("connection");
    connection
        .execute_batch(
            "CREATE TRIGGER fail_assessment_check BEFORE INSERT ON evolution_assessment_checks \
             BEGIN SELECT RAISE(ABORT, 'injected assessment storage failure'); END;",
        )
        .expect("failure trigger");
    drop(connection);
    let witness = witness();
    let output = output();
    let routing = routing();
    let result = SqliteAssessmentRepository::new(database.clone()).persist_completed(
        &PersistCompletedAssessment {
            witness: &witness,
            output: &output,
            routing: &routing,
            model_calls: &[],
            model_evaluation_allowed: false,
            created_at_ms: 10,
            completed_at_ms: 20,
        },
    );
    assert_eq!(result, Err(AssessmentRepositoryError::Storage));
    let connection = database.connection().expect("connection");
    for table in [
        "evolution_assessment_attempts",
        "evolution_assessment_targets",
        "evolution_assessment_score_components",
        "evolution_assessment_checks",
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("rollback count");
        assert_eq!(count, 0, "table {table}");
    }
}

#[test]
fn invalid_audit_is_rejected_without_partial_rows() {
    let (_directory, database) = database_with_seed("assessment-repository-rollback");
    let repository = SqliteAssessmentRepository::new(database.clone());
    let witness = witness();
    let mut output = output();
    output.checks.pop();
    let routing = routing();
    let record = PersistCompletedAssessment {
        witness: &witness,
        output: &output,
        routing: &routing,
        model_calls: &[],
        model_evaluation_allowed: false,
        created_at_ms: 10,
        completed_at_ms: 20,
    };

    assert_eq!(
        repository.persist_completed(&record),
        Err(AssessmentRepositoryError::InvalidInput)
    );
    let connection = database
        .connection()
        .unwrap_or_else(|error| panic!("connection: {error}"));
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_assessment_attempts",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|error| panic!("count: {error}"));
    assert_eq!(count, 0);
}

fn database_with_seed(name: &str) -> (TempDirectory, NativeDatabase) {
    let directory = TempDirectory::new(name);
    let database = NativeDatabase::new(directory.path().to_path_buf())
        .unwrap_or_else(|error| panic!("database: {error}"));
    let connection = database
        .connection()
        .unwrap_or_else(|error| panic!("connection: {error}"));
    connection.execute(
        "INSERT INTO evolution_candidate_seeds (seed_id,seed_version,grouping_key_hash,workspace,category,readiness,readiness_reason,safe_summary,first_occurred_at,last_occurred_at,independent_run_count,signal_set_revision,has_recovery,lineage_summary_json,lineage_signal_count,lineage_truncated_count,created_at) VALUES ('seed-1',1,'group','workspace','verification_outcome','ready','ready','safe','now','now',2,1,1,'{}',1,0,'now')",
        [],
    ).unwrap_or_else(|error| panic!("seed: {error}"));
    drop(connection);
    (directory, database)
}

fn witness() -> AssessmentWitness {
    AssessmentWitness {
        input: SanitizedAssessmentInput {
            schema_version: 1,
            seed_id: "seed-1".to_string(),
            seed_revision: "revision-1".to_string(),
            seed_fingerprint: "fingerprint".to_string(),
            lineage_hash: "lineage".to_string(),
            workspace_id: Some("workspace".to_string()),
            sanitizer_version: "sanitizer-v1".to_string(),
            evidence_ids: vec!["evidence-1".to_string()],
            attribution: EvidenceAttribution::Verified,
        },
        targets: vec![target_witness()],
        selector_policy_version: "selector-v1".to_string(),
        lexical_policy_version: "lexical-v1".to_string(),
        gate_policy_version: "gates-v1".to_string(),
        routing_policy_version: "routing-v1".to_string(),
        confidence_policy_version: "confidence-v1".to_string(),
        consent_version: "assessment-disclosure-v1".to_string(),
        evaluator_configuration: Some("evaluator-hash".to_string()),
    }
}

fn output() -> AssessmentOutput {
    AssessmentOutput {
        schema_version: 1,
        attempt_id: "attempt-1".to_string(),
        status: AssessmentAttemptStatus::Completed,
        classification: SelectionClassification::Selected,
        route: AssessmentRoute::Advance,
        confidence: AssessmentConfidence::High,
        risk: AssessmentRisk::Low,
        targets: vec![RankedTarget {
            witness: target_witness(),
            score: 90,
            attribution_score: 35,
            participation_score: 15,
            compatibility_score: 20,
            lexical_score: 15,
            locality_score: 5,
            matched_feature_classes: vec!["capability".to_string()],
            exclusions: Vec::new(),
            attribution_uncertain: false,
        }],
        selection_threshold: SelectionThresholdWitness {
            leading_score: 90,
            runner_up_score: None,
            margin: 90,
            selected_minimum: 60,
            ambiguous_minimum: 45,
            required_margin: 15,
        },
        attribution_uncertain: false,
        lesson_shape: LessonShape {
            trigger: Some("failure".to_string()),
            required_behavior: Some("inspect".to_string()),
            prohibited_behavior: None,
            verification: Some("test".to_string()),
            environment: Some("project".to_string()),
            content_kinds: vec!["guidance".to_string()],
        },
        checks: QUALITY_CHECK_ORDER_V1
            .iter()
            .map(|kind| QualityCheck {
                kind: *kind,
                result: QualityCheckResult::Pass,
                severity: AssessmentRisk::Low,
                reason_code: "pass".to_string(),
                evidence_ids: vec!["evidence-1".to_string()],
                route_constraints: Vec::new(),
            })
            .collect(),
        evaluator: EvaluatorResult {
            consulted: true,
            selected_target_id: Some("review".to_string()),
            confidence: Some(0.8),
            recommended_route: Some(AssessmentRoute::Advance),
            cited_evidence_ids: vec!["evidence-1".to_string()],
            fallback_reason: None,
        },
    }
}

fn routing() -> RoutingDecision {
    RoutingDecision {
        policy_version: "routing-v1".to_string(),
        route: AssessmentRoute::Advance,
        winning_rule: "high_confidence_low_risk".to_string(),
        route_constraints: Vec::new(),
        rules: vec![RoutingRuleWitness {
            rule_code: "high_confidence_low_risk".to_string(),
            route: AssessmentRoute::Advance,
            matched: true,
            reason_code: "condition_matched".to_string(),
        }],
    }
}

fn target_witness() -> EffectiveTargetWitness {
    EffectiveTargetWitness {
        skill_id: "review".to_string(),
        skill_type: "role".to_string(),
        revision_hash: "r1".to_string(),
        scope: TargetScope::Project,
        lifecycle: TargetLifecycle::Active,
        trust: TargetTrust::Trusted,
    }
}

fn table_columns(connection: &rusqlite::Connection, table: &str) -> Vec<String> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .unwrap_or_else(|error| panic!("columns: {error}"));
    statement
        .query_map([], |row| row.get(1))
        .unwrap_or_else(|error| panic!("query columns: {error}"))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|error| panic!("collect columns: {error}"))
}
