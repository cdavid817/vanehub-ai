use crate::platform::database::DatabaseError;
use rusqlite::Connection;

pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(include_str!("schema.sql"))?;
    Ok(())
}

pub(crate) fn apply_policy_payload_schema(connection: &Connection) -> Result<(), DatabaseError> {
    if crate::platform::database::table_has_column(
        connection,
        "evolution_generation_policy",
        "policy_json",
    )? {
        return Ok(());
    }
    connection.execute_batch(
        "ALTER TABLE evolution_generation_policy ADD COLUMN policy_json TEXT NOT NULL DEFAULT '{}';",
    )?;
    Ok(())
}

pub(crate) fn apply_tool_receipt_names_schema(
    connection: &Connection,
) -> Result<(), DatabaseError> {
    let sql: String = connection.query_row(
        "SELECT sql FROM sqlite_master WHERE type='table' AND name='evolution_generation_tool_receipts'",
        [],
        |row| row.get(0),
    )?;
    if sql.contains("read_dossier_section") {
        return Ok(());
    }
    connection.execute_batch(
        "ALTER TABLE evolution_generation_tool_receipts RENAME TO evolution_generation_tool_receipts_legacy;
         CREATE TABLE evolution_generation_tool_receipts (
           receipt_id TEXT PRIMARY KEY,
           stage_attempt_id TEXT NOT NULL REFERENCES evolution_generation_stage_attempts(attempt_id) ON DELETE CASCADE,
           tool_name TEXT NOT NULL CHECK (tool_name IN ('read_dossier_section','read_skill_excerpt','find_exact_anchor','validate_draft_structure','simulate_local_preview','get_assessment','get_evidence_dossier_section','get_effective_skill','preview_overlay','preview_skill_creation')),
           argument_hash TEXT NOT NULL, source_witness_hash TEXT NOT NULL,
           outcome TEXT NOT NULL CHECK (outcome IN ('succeeded','stale_witness','invalid_argument','result_too_large','budget_exceeded','policy_denied','failed')),
           result_hash TEXT, safe_failure_code TEXT,
           duration_ms INTEGER NOT NULL CHECK (duration_ms >= 0), created_at_ms INTEGER NOT NULL
         );
         INSERT INTO evolution_generation_tool_receipts SELECT * FROM evolution_generation_tool_receipts_legacy;
         DROP TABLE evolution_generation_tool_receipts_legacy;
         CREATE INDEX IF NOT EXISTS idx_evolution_generation_tool_receipts_attempt
           ON evolution_generation_tool_receipts(stage_attempt_id, created_at_ms);",
    )?;
    Ok(())
}

pub(crate) fn apply_governance_tombstone_schema(
    connection: &Connection,
) -> Result<(), DatabaseError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS evolution_generation_governance_tombstones (
           tombstone_id TEXT PRIMARY KEY,
           job_id TEXT NOT NULL,
           package_hash TEXT NOT NULL,
           artifact_hash TEXT NOT NULL,
           validation_report_hash TEXT NOT NULL,
           curator_candidate_id TEXT,
           final_status TEXT NOT NULL,
           source_purge_witness_hash TEXT NOT NULL,
           created_at_ms INTEGER NOT NULL,
           UNIQUE (job_id, package_hash)
         );",
    )?;
    Ok(())
}
