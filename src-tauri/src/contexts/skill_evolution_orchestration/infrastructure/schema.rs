use crate::platform::database::DatabaseError;
use rusqlite::Connection;

pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(include_str!("schema.sql"))?;
    Ok(())
}

pub(crate) fn apply_preflight_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS evolution_auto_preflight_witnesses (
           witness_id TEXT PRIMARY KEY,
           run_id TEXT NOT NULL REFERENCES evolution_runs(run_id),
           eligibility_id TEXT NOT NULL REFERENCES evolution_auto_eligibility(eligibility_id),
           eligibility_proof_hash TEXT NOT NULL,
           reservation_id TEXT NOT NULL REFERENCES evolution_auto_rate_reservations(reservation_id),
           overlay_preview_hash TEXT NOT NULL,
           proof_hash TEXT NOT NULL UNIQUE,
           issued_at_ms INTEGER NOT NULL,
           expires_at_ms INTEGER NOT NULL CHECK (expires_at_ms = issued_at_ms + 5000),
           consumed_at_ms INTEGER,
           status TEXT NOT NULL CHECK (status IN ('active','consumed','expired')),
           revision INTEGER NOT NULL CHECK (revision >= 0)
         );
         CREATE INDEX IF NOT EXISTS idx_evolution_preflight_active
           ON evolution_auto_preflight_witnesses(expires_at_ms) WHERE status = 'active';",
    )?;
    Ok(())
}

pub(crate) fn apply_breaker_failure_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS evolution_auto_application_failures (
           failure_id TEXT PRIMARY KEY,
           workspace_id TEXT NOT NULL,
           source_run_id TEXT,
           source_application_id TEXT,
           category TEXT NOT NULL CHECK (category IN (
             'security_failure','integrity_failure','audit_failure',
             'idempotency_failure','application_failure_threshold'
           )),
           occurred_at_ms INTEGER NOT NULL,
           UNIQUE (workspace_id, source_run_id, source_application_id, category)
         );
         CREATE INDEX IF NOT EXISTS idx_evolution_auto_failures_workspace_time
           ON evolution_auto_application_failures(workspace_id,occurred_at_ms);
         CREATE UNIQUE INDEX IF NOT EXISTS idx_evolution_workspace_breaker
           ON evolution_auto_breakers(workspace_id) WHERE skill_id IS NULL;
         CREATE UNIQUE INDEX IF NOT EXISTS idx_evolution_skill_breaker
           ON evolution_auto_breakers(workspace_id,skill_id) WHERE skill_id IS NOT NULL;",
    )?;
    Ok(())
}

pub(crate) fn apply_probation_baseline_schema(
    connection: &Connection,
) -> Result<(), DatabaseError> {
    if !crate::platform::database::table_has_column(
        connection,
        "evolution_probation_observations",
        "baseline_exceeded",
    )? {
        connection.execute(
            "ALTER TABLE evolution_probation_observations
             ADD COLUMN baseline_exceeded INTEGER NOT NULL DEFAULT 0
             CHECK (baseline_exceeded IN (0,1))",
            [],
        )?;
    }
    Ok(())
}
