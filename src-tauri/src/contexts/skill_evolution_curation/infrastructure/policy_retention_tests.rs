use super::*;
use crate::contexts::skill_evolution_curation::domain::*;
use rusqlite::{params, Connection};
use serde_json::json;

const DAY_MS: i64 = 24 * 60 * 60 * 1_000;

fn database() -> Connection {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .execute_batch(
            "PRAGMA foreign_keys=ON;
             CREATE TABLE evolution_assessment_attempts (attempt_id TEXT PRIMARY KEY);
             INSERT INTO evolution_assessment_attempts VALUES ('assessment-1');",
        )
        .expect("assessment schema");
    apply_schema(&connection).expect("curator schema");
    connection
}

fn snapshot(candidate_id: &str, created_at_ms: i64) -> CuratorCandidateSnapshot {
    CuratorCandidateSnapshot {
        schema_version: 1,
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
            lineage_hash: "lineage-sensitive-hash".into(),
        }],
        quality_checks: vec![],
        assessment_witness_hash: "assessment-hash".into(),
        policy_witness_hash: "original-policy-hash".into(),
        witness_hash: format!("candidate-hash-{candidate_id}"),
        state: CuratorCandidateState::Pending,
        staleness: vec![],
        revision: 1,
        created_at_ms,
        updated_at_ms: created_at_ms,
    }
}

fn valid_update() -> CuratorPolicyUpdateV1 {
    CuratorPolicyV1::manual_default("workspace:one".into()).into()
}

#[test]
fn policy_defaults_bounds_and_unknown_mutation_fields_preserve_manual_approval() {
    let default = CuratorPolicyV1::manual_default("workspace:one".into());
    assert_eq!(default.open_retention_days, 180);
    assert_eq!(default.terminal_retention_days, 365);
    valid_update().validate().expect("valid manual policy");

    let mut invalid_retention = valid_update();
    invalid_retention.open_retention_days = 181;
    assert_eq!(
        invalid_retention.validate(),
        Err(CuratorPolicyValidationError::RetentionBounds)
    );

    for prohibited in ["automaticApply", "approveAll", "mutationBypass"] {
        let mut value = serde_json::to_value(valid_update()).expect("policy json");
        value
            .as_object_mut()
            .expect("policy object")
            .insert(prohibited.into(), json!(true));
        assert!(serde_json::from_value::<CuratorPolicyUpdateV1>(value).is_err());
    }
}

#[test]
fn policy_update_is_conflict_safe_and_invalidates_preview_without_rewriting_history() {
    let mut connection = database();
    SqliteCuratorRepository::new(&mut connection)
        .insert_candidate(&snapshot("candidate-policy", 10))
        .expect("candidate");
    insert_preview_fixture(&connection, "candidate-policy");

    let result = SqliteCuratorRepository::new(&mut connection)
        .update_policy("workspace:one", 1, valid_update(), 50)
        .expect("policy update");
    assert_eq!(result.policy.revision, 2);
    assert_eq!(result.affected_candidates, 1);
    let state: (Option<String>, String, i64, String) = connection
        .query_row(
            "SELECT current_preview_id,policy_witness_hash,revision,staleness_json
             FROM evolution_curator_candidates WHERE candidate_id='candidate-policy'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("candidate state");
    assert_eq!(state.0, None);
    assert_eq!(state.1, result.policy_hash);
    assert_eq!(state.2, 3);
    assert!(state.3.contains("policy_changed"));
    let historical: (String, Option<i64>) = connection
        .query_row(
            "SELECT witnesses_json,invalidated_at_ms FROM evolution_curator_previews
             WHERE preview_id='preview-policy'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("historical preview");
    assert!(historical.0.contains("original-policy-hash"));
    assert_eq!(historical.1, Some(50));
    assert_eq!(
        SqliteCuratorRepository::new(&mut connection).update_policy(
            "workspace:one",
            1,
            valid_update(),
            51,
        ),
        Err(CuratorPolicyRetentionError::Conflict {
            current_revision: 2
        })
    );
}

#[test]
fn retention_expires_open_candidates_and_purges_terminal_detail() {
    let mut connection = database();
    {
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        repository
            .insert_candidate(&snapshot("candidate-open", 10))
            .expect("open candidate");
        repository
            .insert_candidate(&snapshot("candidate-terminal", 11))
            .expect("terminal candidate");
    }
    connection
        .execute(
            "UPDATE evolution_curator_candidates SET state='rejected' WHERE candidate_id='candidate-terminal'",
            [],
        )
        .expect("terminal state");
    let report = SqliteCuratorRepository::new(&mut connection)
        .run_retention("workspace:one", 366 * DAY_MS)
        .expect("retention");
    assert_eq!(report.expired_open_candidates, 1);
    assert_eq!(report.purged_terminal_details, 1);
    let open_state: String = connection
        .query_row(
            "SELECT state FROM evolution_curator_candidates WHERE candidate_id='candidate-open'",
            [],
            |row| row.get(0),
        )
        .expect("open state");
    assert_eq!(open_state, "superseded");
    assert_eq!(detail_count(&connection), 0);
}

#[test]
fn evidence_purge_redacts_open_content_and_preserves_applied_overlay_tombstone() {
    let mut connection = database();
    {
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        repository
            .insert_candidate(&snapshot("candidate-open", 10))
            .expect("open candidate");
        repository
            .insert_candidate(&snapshot("candidate-applied", 11))
            .expect("applied candidate");
    }
    insert_applied_tombstone(&connection, "candidate-applied");
    let report = SqliteCuratorRepository::new(&mut connection)
        .purge_evidence("evidence-1", 100)
        .expect("purge");
    assert_eq!(report.redacted_candidates, 2);
    assert_eq!(report.superseded_open_candidates, 1);
    assert_eq!(report.preserved_applied_tombstones, 1);
    assert_eq!(detail_count(&connection), 0);
    let repository = SqliteCuratorRepository::new(&mut connection);
    let tombstone = repository
        .applied_tombstone("candidate-applied")
        .expect("applied tombstone");
    assert_eq!(tombstone.overlay_revision, "overlay-revision-2");
    assert_eq!(tombstone.overlay_history_id, "overlay-history-2");
    let snapshot_json: String = connection
        .query_row(
            "SELECT snapshot_json FROM evolution_curator_candidates WHERE candidate_id='candidate-applied'",
            [],
            |row| row.get(0),
        )
        .expect("redacted snapshot");
    assert!(!snapshot_json.contains("lineage-sensitive-hash"));
}

fn insert_preview_fixture(connection: &Connection, candidate_id: &str) {
    connection.execute(
        "INSERT INTO evolution_curator_drafts
         (draft_id,candidate_id,revision,kind,target_skill_id,target_revision,overlay_scope,validated_body_json,
          body_hash,rationale,expected_effective_change,evidence_ids_json,scanner_version,base_hash,
          base_package_hash,effective_hash,pin_witness,trust_witness,conflict_witness,created_at_ms)
         VALUES ('draft-policy',?1,1,'learn_block','code-review','target-revision-1','project','{}',
          'draft-hash','','','[]','scanner-v1','base','package','effective','pin','trust','conflict',20)",
        [candidate_id],
    ).expect("draft");
    connection
        .execute(
            "INSERT INTO evolution_curator_draft_assessments
         (draft_assessment_id,candidate_id,candidate_revision,draft_id,draft_revision,draft_hash,
          candidate_witness_hash,target_skill_id,target_revision,checks_json,approvable,
          model_evaluation_allowed,model_consulted,witness_hash,created_at_ms)
         VALUES ('draft-assessment-policy',?1,2,'draft-policy',1,'draft-hash','candidate-hash',
          'code-review','target-revision-1','[]',1,0,0,'assessment-hash',20)",
            [candidate_id],
        )
        .expect("draft assessment");
    connection.execute(
        "INSERT INTO evolution_curator_previews
         (preview_id,candidate_id,candidate_revision,draft_id,draft_revision,draft_assessment_id,
          witness_hash,effective_diff_hash,witnesses_json,diff_projection_json,validation_json,issued_at_ms,expires_at_ms)
         VALUES ('preview-policy',?1,2,'draft-policy',1,'draft-assessment-policy','preview-hash','diff-hash',
          '{\"policyHash\":\"original-policy-hash\"}','{}','{}',20,900020)",
        [candidate_id],
    ).expect("preview");
    connection
        .execute(
            "UPDATE evolution_curator_candidates SET state='ready_for_review',revision=2,
         current_draft_id='draft-policy',current_preview_id='preview-policy' WHERE candidate_id=?1",
            [candidate_id],
        )
        .expect("ready candidate");
}

fn insert_applied_tombstone(connection: &Connection, candidate_id: &str) {
    connection
        .execute(
            "UPDATE evolution_curator_candidates SET state='applied' WHERE candidate_id=?1",
            [candidate_id],
        )
        .expect("applied state");
    connection
        .execute(
            "INSERT INTO evolution_curator_decisions
         (decision_id,candidate_id,candidate_revision,decision_kind,actor_class,reason_code,
          preview_hash,idempotency_key,decided_at_ms)
         VALUES ('decision-applied',?1,1,'approve','local_interactive_user','approved',
          'preview-hash','approval-key',20)",
            [candidate_id],
        )
        .expect("decision");
    connection
        .execute(
            "INSERT INTO evolution_curator_applications
         (application_id,candidate_id,decision_id,status,approved_witness_hash,overlay_revision,
          overlay_history_id,revision,created_at_ms,updated_at_ms)
         VALUES ('application-applied',?1,'decision-applied','applied','preview-hash',
          'overlay-revision-2','overlay-history-2',2,20,30)",
            [candidate_id],
        )
        .expect("application");
}

fn detail_count(connection: &Connection) -> i64 {
    connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM evolution_curator_candidate_sources) +
                (SELECT COUNT(*) FROM evolution_curator_drafts)",
            [],
            |row| row.get(0),
        )
        .expect("detail count")
}
