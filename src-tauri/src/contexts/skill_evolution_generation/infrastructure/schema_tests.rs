use super::{apply_schema, apply_tool_receipt_names_schema};
use rusqlite::Connection;

#[test]
fn schema_creates_all_generation_tables() {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
         CREATE TABLE evolution_candidate_seeds (seed_id TEXT PRIMARY KEY);
         CREATE TABLE evolution_assessment_attempts (attempt_id TEXT PRIMARY KEY);
         CREATE TABLE evolution_curator_candidates (candidate_id TEXT PRIMARY KEY);",
        )
        .expect("dependency schemas");

    apply_schema(&connection).expect("generation schema");

    let expected = [
        "evolution_generation_policy",
        "evolution_generation_jobs",
        "evolution_generation_job_sources",
        "evolution_evidence_dossiers",
        "evolution_evidence_dossier_sections",
        "evolution_evidence_dossier_links",
        "evolution_generation_stage_attempts",
        "evolution_generation_model_calls",
        "evolution_generation_tool_receipts",
        "evolution_generation_structured_results",
        "evolution_generated_drafts",
        "evolution_generation_validations",
        "evolution_generation_handoffs",
        "evolution_generated_skill_quarantine",
        "evolution_generation_exports",
        "evolution_generation_governance_tombstones",
    ];
    for table in expected {
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("table lookup");
        assert_eq!(count, 1, "missing table {table}");
    }
}

#[test]
fn schema_rejects_a_section_kind_at_the_wrong_ordinal() {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE evolution_candidate_seeds (seed_id TEXT PRIMARY KEY);
             CREATE TABLE evolution_assessment_attempts (attempt_id TEXT PRIMARY KEY);
             CREATE TABLE evolution_curator_candidates (candidate_id TEXT PRIMARY KEY);",
        )
        .expect("dependency schemas");
    apply_schema(&connection).expect("generation schema");
    connection
        .execute(
            "INSERT INTO evolution_evidence_dossiers
         (dossier_id,schema_version,revision,input_witness_hash,builder_version,sanitizer_version,
          canonical_size_bytes,content_hash,created_at_ms)
         VALUES ('dossier',1,1,'input','builder','sanitizer',0,'dossier-hash',1)",
            [],
        )
        .expect("dossier");
    let result = connection.execute(
        "INSERT INTO evolution_evidence_dossier_sections
         (dossier_id,ordinal,section_kind,status,source_witnesses_json,records_json,
          truncation_json,section_hash)
         VALUES ('dossier',0,'executive_summary','complete','[]','[]','{}','section-hash')",
        [],
    );
    assert!(result.is_err());
}

#[test]
fn tool_receipt_migration_preserves_legacy_rows_and_accepts_the_exact_runtime_names() {
    let connection = Connection::open_in_memory().expect("database");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE evolution_generation_stage_attempts (attempt_id TEXT PRIMARY KEY);
             INSERT INTO evolution_generation_stage_attempts VALUES ('attempt');
             CREATE TABLE evolution_generation_tool_receipts (
               receipt_id TEXT PRIMARY KEY,
               stage_attempt_id TEXT NOT NULL REFERENCES evolution_generation_stage_attempts(attempt_id),
               tool_name TEXT NOT NULL CHECK (tool_name IN ('get_assessment')),
               argument_hash TEXT NOT NULL,
               source_witness_hash TEXT NOT NULL,
               outcome TEXT NOT NULL CHECK (outcome IN ('succeeded','stale_witness','invalid_argument','result_too_large','budget_exceeded','policy_denied','failed')),
               result_hash TEXT,
               safe_failure_code TEXT,
               duration_ms INTEGER NOT NULL CHECK (duration_ms >= 0),
               created_at_ms INTEGER NOT NULL
             );
             INSERT INTO evolution_generation_tool_receipts VALUES
               ('legacy','attempt','get_assessment','argument','witness','succeeded','result',NULL,1,2);",
        )
        .expect("legacy schema");

    apply_tool_receipt_names_schema(&connection).expect("receipt migration");
    apply_tool_receipt_names_schema(&connection).expect("idempotent receipt migration");
    let legacy_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM evolution_generation_tool_receipts WHERE receipt_id='legacy'",
            [],
            |row| row.get(0),
        )
        .expect("legacy row");
    assert_eq!(legacy_rows, 1);
    connection
        .execute(
            "INSERT INTO evolution_generation_tool_receipts VALUES
             ('current','attempt','read_dossier_section','argument','witness','succeeded','result',NULL,1,3)",
            [],
        )
        .expect("current tool name");
    assert!(connection
        .execute(
            "INSERT INTO evolution_generation_tool_receipts VALUES
             ('forbidden','attempt','shell','argument','witness','policy_denied',NULL,'denied',1,4)",
            [],
        )
        .is_err());
}
