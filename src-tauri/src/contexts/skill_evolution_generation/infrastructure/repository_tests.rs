use std::{
    sync::{Arc, Barrier},
    thread,
    time::Duration,
};

use crate::{
    contexts::skill_evolution_generation::{
        application::{
            retention_cutoffs, GenerationRetentionPolicyV1, GenerationToolReceiptPort,
            NewSkillCreationPreviewV1, PreparedQuarantinedSkillV1,
        },
        domain::{
            FrozenGenerationInputV1, GenerationBudgetV1, GenerationEvidenceWitnessV1,
            GenerationJobStatus, GenerationJobV1, GenerationModelCallRecordV1,
            GenerationModelOutcome, GenerationQuarantineStatus, GenerationStageAttemptV1,
            GenerationStageKind, GenerationStageStatus, GenerationToolOutcome,
            GenerationToolReceiptV1, GenerationUsageV1, QuarantinedSkillProposalV1,
            GENERATION_SCHEMA_VERSION_V1,
        },
    },
    test_support::TempDirectory,
};
use rusqlite::Connection;
use serde_json::json;

use super::{
    apply_schema, canonical_hash, canonical_json, GenerationJobRepository,
    GenerationModelCallRepository, GenerationPersistenceError, GenerationQuarantineRepository,
    GenerationRetentionRepository, GenerationStageRepository, GenerationToolReceiptRepository,
    JobTransition, PersistGenerationOutcome, StageAttemptCompletion,
};

#[test]
fn canonical_serialization_sorts_keys_and_hashes_reproducibly() {
    let left = json!({"z": 1, "a": {"y": 2, "b": 3}});
    let right: serde_json::Value =
        serde_json::from_str(r#"{"a":{"b":3,"y":2},"z":1}"#).expect("json");
    assert_eq!(canonical_json(&left), canonical_json(&right));
    assert_eq!(canonical_hash(&left), canonical_hash(&right));
}

#[test]
fn request_idempotency_and_optimistic_revision_are_enforced() {
    let connection = setup_connection();
    let input = input("request-one");
    let job = job("job-one", &input);
    let repository = GenerationJobRepository::new(&connection);
    assert!(matches!(
        repository.persist_job(&job, &input),
        Ok(PersistGenerationOutcome::Inserted { .. })
    ));
    assert_eq!(
        repository.persist_job(&job, &input),
        Ok(PersistGenerationOutcome::Coalesced {
            id: "job-one".into()
        })
    );

    let usage = canonical_json(&GenerationUsageV1::default()).expect("usage");
    let transition = JobTransition {
        job_id: "job-one",
        expected_revision: 1,
        status: GenerationJobStatus::Running,
        current_stage: Some(GenerationStageKind::FreezeInput),
        usage_json: &usage,
        safe_failure_code: None,
        updated_at_ms: 2,
    };
    assert_eq!(repository.transition_job(&transition), Ok(2));
    assert_eq!(
        repository.transition_job(&transition),
        Err(GenerationPersistenceError::Conflict)
    );
}

#[test]
fn terminal_stage_attempt_is_immutable_and_malformed_json_is_rejected() {
    let connection = setup_connection();
    let input = input("request-two");
    let job = job("job-two", &input);
    let repository = GenerationJobRepository::new(&connection);
    repository.persist_job(&job, &input).expect("job");
    let attempt = GenerationStageAttemptV1 {
        attempt_id: "attempt-one".into(),
        job_id: job.job_id,
        stage: GenerationStageKind::FreezeInput,
        attempt: 1,
        status: GenerationStageStatus::Succeeded,
        input_hash: "sha256:input".into(),
        output_hash: Some("sha256:output".into()),
        usage: GenerationUsageV1::default(),
        safe_failure_code: None,
        started_at_ms: 1,
        completed_at_ms: Some(2),
        superseded_by_attempt_id: None,
    };
    GenerationStageRepository::new(&connection)
        .persist_attempt(&attempt)
        .expect("attempt");
    assert!(connection.execute("UPDATE evolution_generation_stage_attempts SET output_hash='changed' WHERE attempt_id='attempt-one'", []).is_err());
    assert!(connection.execute(
        "INSERT INTO evolution_generation_policy (workspace_id,schema_version,consent_state,
         disclosure_version,job_budget_json,daily_budget_json,retention_json,policy_hash,updated_at_ms)
         VALUES ('workspace',1,'disabled','v1','not-json','{}','{}','sha256:policy',1)", [],
    ).is_err());
}

#[test]
fn running_stage_attempt_completes_once_and_then_coalesces_or_rejects_changes() {
    let connection = setup_connection();
    let input = input("request-stage");
    let job = job("job-stage", &input);
    GenerationJobRepository::new(&connection)
        .persist_job(&job, &input)
        .expect("job");
    let repository = GenerationStageRepository::new(&connection);
    repository
        .persist_attempt(&GenerationStageAttemptV1 {
            attempt_id: "attempt-stage".into(),
            job_id: job.job_id,
            stage: GenerationStageKind::FreezeInput,
            attempt: 1,
            status: GenerationStageStatus::Running,
            input_hash: "sha256:input".into(),
            output_hash: None,
            usage: GenerationUsageV1::default(),
            safe_failure_code: None,
            started_at_ms: 1,
            completed_at_ms: None,
            superseded_by_attempt_id: None,
        })
        .expect("running");
    let usage = GenerationUsageV1 {
        elapsed_ms: 5,
        ..Default::default()
    };
    let completion = StageAttemptCompletion {
        attempt_id: "attempt-stage",
        status: GenerationStageStatus::Succeeded,
        expected_input_hash: "sha256:input",
        output_hash: Some("sha256:output"),
        usage: &usage,
        safe_failure_code: None,
        completed_at_ms: 6,
        superseded_by_attempt_id: None,
    };
    assert!(matches!(
        repository.complete_attempt(&completion),
        Ok(PersistGenerationOutcome::Inserted { .. })
    ));
    assert!(matches!(
        repository.complete_attempt(&completion),
        Ok(PersistGenerationOutcome::Coalesced { .. })
    ));
    let changed = StageAttemptCompletion {
        output_hash: Some("sha256:changed"),
        ..completion
    };
    assert_eq!(
        repository.complete_attempt(&changed),
        Err(GenerationPersistenceError::Immutable)
    );
}

#[test]
fn model_provenance_persists_hashes_and_counts_without_raw_payload_columns() {
    let connection = setup_connection();
    let input = input("request-model");
    let job = job("job-model", &input);
    GenerationJobRepository::new(&connection)
        .persist_job(&job, &input)
        .expect("job");
    GenerationStageRepository::new(&connection)
        .persist_attempt(&GenerationStageAttemptV1 {
            attempt_id: "attempt-model".into(),
            job_id: job.job_id,
            stage: GenerationStageKind::PlanMutation,
            attempt: 1,
            status: GenerationStageStatus::Running,
            input_hash: "sha256:input".into(),
            output_hash: None,
            usage: GenerationUsageV1::default(),
            safe_failure_code: None,
            started_at_ms: 1,
            completed_at_ms: None,
            superseded_by_attempt_id: None,
        })
        .expect("attempt");
    let record = GenerationModelCallRecordV1 {
        model_call_id: "model-call".into(),
        stage_attempt_id: "attempt-model".into(),
        purpose: "skill_evolution_generation".into(),
        provider_protocol: Some("openai-compatible".into()),
        provider_profile_id: Some("profile".into()),
        model_id: Some("model".into()),
        prompt_template_version: "skill-generation-control-v1".into(),
        response_schema_version: "mutation-plan-v1".into(),
        outcome: GenerationModelOutcome::Valid,
        input_tokens: 120,
        output_tokens: 40,
        latency_ms: 50,
        structured_response_hash: Some("sha256:response".into()),
        safe_failure_code: None,
        created_at_ms: 2,
    };
    let repository = GenerationModelCallRepository::new(&connection);
    assert!(matches!(
        repository.persist(&record),
        Ok(PersistGenerationOutcome::Inserted { .. })
    ));
    assert!(matches!(
        repository.persist(&record),
        Ok(PersistGenerationOutcome::Coalesced { .. })
    ));
    let columns: Vec<String> = connection
        .prepare("PRAGMA table_info(evolution_generation_model_calls)")
        .expect("columns")
        .query_map([], |row| row.get(1))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("collect");
    assert!(!columns.iter().any(|column| matches!(
        column.as_str(),
        "raw_prompt" | "raw_request" | "raw_response" | "provider_payload"
    )));
}

#[test]
fn tool_repository_persists_safe_success_and_failure_receipts() {
    let connection = setup_connection();
    let input = input("request-tool");
    let job = job("job-tool", &input);
    GenerationJobRepository::new(&connection)
        .persist_job(&job, &input)
        .expect("job");
    GenerationStageRepository::new(&connection)
        .persist_attempt(&GenerationStageAttemptV1 {
            attempt_id: "attempt-tool".into(),
            job_id: job.job_id,
            stage: GenerationStageKind::InspectTarget,
            attempt: 1,
            status: GenerationStageStatus::Running,
            input_hash: "sha256:input".into(),
            output_hash: None,
            usage: GenerationUsageV1::default(),
            safe_failure_code: None,
            started_at_ms: 1,
            completed_at_ms: None,
            superseded_by_attempt_id: None,
        })
        .expect("attempt");
    let repository = GenerationToolReceiptRepository::new(&connection);
    repository
        .persist_receipt(&GenerationToolReceiptV1 {
            receipt_id: "receipt-success".into(),
            stage_attempt_id: "attempt-tool".into(),
            tool_name: "read_skill_excerpt".into(),
            argument_hash: "sha256:argument".into(),
            source_witness_hash: "sha256:input".into(),
            outcome: GenerationToolOutcome::Succeeded,
            result_hash: Some("sha256:result".into()),
            safe_failure_code: None,
            duration_ms: 3,
            created_at_ms: 4,
        })
        .expect("success receipt");
    repository
        .persist_receipt(&GenerationToolReceiptV1 {
            receipt_id: "receipt-failure".into(),
            stage_attempt_id: "attempt-tool".into(),
            tool_name: "find_exact_anchor".into(),
            argument_hash: "sha256:argument-two".into(),
            source_witness_hash: "sha256:input".into(),
            outcome: GenerationToolOutcome::InvalidArgument,
            result_hash: None,
            safe_failure_code: Some("generation_tool_invalid_argument".into()),
            duration_ms: 2,
            created_at_ms: 5,
        })
        .expect("failure receipt");
    let rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_generation_tool_receipts",
            [],
            |row| row.get(0),
        )
        .expect("receipt count");
    assert_eq!(rows, 2);
}

#[test]
fn quarantined_skill_bytes_are_idempotent_and_never_enter_skill_discovery() {
    let connection = setup_connection();
    let input = input("request-quarantine");
    let job = job("job-quarantine", &input);
    GenerationJobRepository::new(&connection)
        .persist_job(&job, &input)
        .expect("job");
    let rendered = "---\nid: focused-review\n---\n\nReview safely.\n";
    let artifact_hash = super::sha256_bytes(rendered.as_bytes());
    let prepared = PreparedQuarantinedSkillV1 {
        proposal: QuarantinedSkillProposalV1 {
            proposal_id: "proposal-quarantine".into(),
            job_id: job.job_id,
            status: GenerationQuarantineStatus::Quarantined,
            candidate_id: "focused-review".into(),
            scope: "project".into(),
            workspace_id: Some("workspace-one".into()),
            artifact_hash: artifact_hash.clone(),
            catalog_witness_hash: "sha256:catalog".into(),
            revision: 1,
        },
        rendered_skill_md: rendered.into(),
        preview: NewSkillCreationPreviewV1 {
            candidate_id: "focused-review".into(),
            scope: "project".into(),
            workspace_id: Some("workspace-one".into()),
            skill_type: "utility".into(),
            frontmatter: "id: focused-review".into(),
            instructions: "Review safely.".into(),
            estimated_tokens: 10,
            built_in_tools: Vec::new(),
            collision_free: true,
            catalog_witness_hash: "sha256:catalog".into(),
            artifact_hash,
        },
        created_at_ms: 2,
    };
    let repository = GenerationQuarantineRepository::new(&connection);
    assert!(matches!(
        repository.persist(&prepared),
        Ok(PersistGenerationOutcome::Inserted { .. })
    ));
    assert!(matches!(
        repository.persist(&prepared),
        Ok(PersistGenerationOutcome::Coalesced { .. })
    ));
    assert_eq!(
        repository.rendered_skill_md("proposal-quarantine"),
        Ok(Some(rendered.into()))
    );
    let discovered: i64 = connection
        .query_row("SELECT COUNT(*) FROM skills", [], |row| row.get(0))
        .expect("skills count");
    assert_eq!(discovered, 0);
}

#[test]
fn evidence_purge_removes_details_keeps_tombstone_and_discloses_external_exports() {
    let connection = setup_connection();
    let input = input("request-purge");
    let job = job("job-purge", &input);
    GenerationJobRepository::new(&connection)
        .persist_job(&job, &input)
        .expect("job");
    connection
        .execute_batch(
            "UPDATE evolution_generation_jobs SET status='completed' WHERE job_id='job-purge';
         INSERT INTO evolution_generation_job_sources VALUES
           ('job-purge','evidence','evidence-one','revision-one','source-hash',NULL);
         INSERT INTO evolution_evidence_dossiers
           (dossier_id,schema_version,revision,input_witness_hash,builder_version,sanitizer_version,
            canonical_size_bytes,content_hash,created_at_ms)
           VALUES ('dossier-purge',1,1,'input','builder','sanitizer',1,'dossier-hash',1);
         INSERT INTO evolution_evidence_dossier_links VALUES
           ('dossier-purge','evidence','evidence-one','revision-one','source-hash');
         INSERT INTO evolution_evidence_dossier_links VALUES
           ('dossier-purge','job','job-purge','1','job-hash');
         INSERT INTO evolution_generation_exports VALUES
           ('export-one','dossier-purge','json',1,1,'redaction','export-hash',10,2);
         INSERT INTO evolution_generated_drafts
           (draft_id,job_id,generation_attempt,artifact_kind,renderer_version,media_type,
            rendered_content,size_bytes,content_hash,permanently_manual,created_at_ms)
           VALUES ('draft-purge','job-purge',1,'overlay_learn_block','renderer','text/markdown',
                   'content',7,'artifact-hash',1,2);
         INSERT INTO evolution_generation_validations
           (validation_id,job_id,draft_id,draft_attempt,validator_version,status,checks_json,
            preview_witness_hash,report_hash,repair_attempt,created_at_ms)
           VALUES ('validation-purge','job-purge','draft-purge',1,'validator','passed','[]',
                   'preview-hash','validation-hash',0,3);
         INSERT INTO evolution_generation_handoffs
           (handoff_id,job_id,validation_id,curator_candidate_id,package_json,package_hash,status,
            safe_failure_code,created_at_ms,updated_at_ms)
           VALUES ('handoff-purge','job-purge','validation-purge',NULL,'{}','package-hash',
                   'delivered',NULL,4,4);",
        )
        .expect("purge fixture");

    let result = GenerationRetentionRepository::new(&connection)
        .purge_source_evidence("evidence-one", Some("revision-one"), "sha256:purge", 10)
        .expect("purge");
    assert_eq!(result.removed_jobs, 1);
    assert_eq!(result.removed_dossiers, 1);
    assert_eq!(result.retained_tombstones, 1);
    assert_eq!(result.removed_export_manifests, 1);
    assert!(result.exported_files_remain_user_managed);
    let tombstones: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_generation_governance_tombstones",
            [],
            |row| row.get(0),
        )
        .expect("tombstones");
    assert_eq!(tombstones, 1);
}

#[test]
fn bounded_retention_removes_expired_failures_but_not_recent_jobs() {
    let connection = setup_connection();
    for (job_id, request_id, updated_at_ms) in [
        ("job-expired", "request-expired", 1_i64),
        ("job-recent", "request-recent", 3_900_000_000_i64),
    ] {
        let input = input(request_id);
        let job = job(job_id, &input);
        GenerationJobRepository::new(&connection)
            .persist_job(&job, &input)
            .expect("job");
        connection
            .execute(
                "UPDATE evolution_generation_jobs SET status='failed',updated_at_ms=?2 WHERE job_id=?1",
                rusqlite::params![job_id, updated_at_ms],
            )
            .expect("status");
    }
    let policy = GenerationRetentionPolicyV1 {
        failed_cancelled_days: 30,
        completed_package_days: 180,
    };
    let cutoffs = retention_cutoffs(policy, 4_000_000_000).expect("cutoffs");
    let result = GenerationRetentionRepository::new(&connection)
        .apply_retention(cutoffs, 4_000_000_000)
        .expect("retention");
    assert_eq!(result.removed_jobs, 1);
    let remaining: String = connection
        .query_row("SELECT job_id FROM evolution_generation_jobs", [], |row| {
            row.get(0)
        })
        .expect("remaining job");
    assert_eq!(remaining, "job-recent");
    assert!(retention_cutoffs(
        GenerationRetentionPolicyV1 {
            failed_cancelled_days: 181,
            completed_package_days: 365,
        },
        4_000_000_000
    )
    .is_none());
}

#[test]
fn concurrent_identical_requests_coalesce_to_one_job() {
    let directory = TempDirectory::new("generation-concurrency");
    let path = directory.path().join("generation.sqlite");
    let connection = Connection::open(&path).expect("database");
    connection
        .busy_timeout(Duration::from_secs(5))
        .expect("busy timeout");
    install_dependencies(&connection);
    apply_schema(&connection).expect("schema");
    drop(connection);
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = ["job-concurrent-a", "job-concurrent-b"]
        .into_iter()
        .map(|job_id| {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                let connection = Connection::open(path).expect("database");
                connection
                    .busy_timeout(Duration::from_secs(5))
                    .expect("busy timeout");
                let input = input("request-concurrent");
                let job = job(job_id, &input);
                barrier.wait();
                GenerationJobRepository::new(&connection).persist_job(&job, &input)
            })
        })
        .collect();
    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("thread").expect("persist"))
        .collect();
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, PersistGenerationOutcome::Inserted { .. }))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, PersistGenerationOutcome::Coalesced { .. }))
            .count(),
        1
    );
}

fn setup_connection() -> Connection {
    let connection = Connection::open_in_memory().expect("database");
    install_dependencies(&connection);
    apply_schema(&connection).expect("schema");
    apply_schema(&connection).expect("idempotent schema");
    connection
}

fn install_dependencies(connection: &Connection) {
    connection
        .execute_batch(
            "PRAGMA foreign_keys=ON;
        CREATE TABLE evolution_candidate_seeds (seed_id TEXT PRIMARY KEY);
        CREATE TABLE evolution_assessment_attempts (attempt_id TEXT PRIMARY KEY);
        CREATE TABLE evolution_curator_candidates (candidate_id TEXT PRIMARY KEY);
        CREATE TABLE skills (id TEXT PRIMARY KEY);
        INSERT INTO evolution_candidate_seeds VALUES ('seed-one');
        INSERT INTO evolution_assessment_attempts VALUES ('assessment-one');",
        )
        .expect("dependencies");
}

fn input(request_id: &str) -> FrozenGenerationInputV1 {
    FrozenGenerationInputV1 {
        schema_version: GENERATION_SCHEMA_VERSION_V1,
        request_id: request_id.into(),
        workspace_id: Some("workspace-one".into()),
        seed_id: "seed-one".into(),
        seed_revision: "seed-r1".into(),
        assessment_attempt_id: "assessment-one".into(),
        assessment_revision: "assessment-r1".into(),
        assessment_route: "advance".into(),
        target: None,
        evidence: GenerationEvidenceWitnessV1::default(),
        effective_skill: None,
        curator: None,
        policy_revision: 1,
        policy_hash: "sha256:policy".into(),
        consent_revision: 1,
        consent_hash: "sha256:consent".into(),
        model_configuration_hash: "sha256:model".into(),
        dossier_builder_version: "v1".into(),
        renderer_version: "v1".into(),
        validator_version: "v1".into(),
        frozen_at_ms: 1,
    }
}

fn job(job_id: &str, input: &FrozenGenerationInputV1) -> GenerationJobV1 {
    GenerationJobV1 {
        schema_version: GENERATION_SCHEMA_VERSION_V1,
        job_id: job_id.into(),
        request_id: input.request_id.clone(),
        workspace_id: input.workspace_id.clone(),
        status: GenerationJobStatus::Requested,
        current_stage: None,
        input_witness_hash: canonical_hash(input).expect("hash"),
        current_attempt: 1,
        budget: GenerationBudgetV1 {
            wall_time_ms: 1_000,
            model_calls: 2,
            tool_calls: 5,
            input_tokens: 1_000,
            output_tokens: 500,
            validation_repairs: 1,
        },
        usage: GenerationUsageV1::default(),
        safe_failure_code: None,
        supersedes_job_id: None,
        created_at_ms: 1,
        updated_at_ms: 1,
    }
}
