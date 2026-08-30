use crate::platform::database::DatabaseError;
use rusqlite::Connection;

pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(include_str!("schema.sql"))?;
    Ok(())
}

pub(crate) fn apply_system_policy_authorization_schema(
    connection: &Connection,
) -> Result<(), DatabaseError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS evolution_curator_system_policy_authorizations (
           application_id TEXT PRIMARY KEY REFERENCES evolution_curator_applications(application_id),
           run_id TEXT NOT NULL,
           eligibility_id TEXT NOT NULL,
           eligibility_proof_hash TEXT NOT NULL,
           preflight_witness_hash TEXT NOT NULL UNIQUE,
           policy_witness_hash TEXT NOT NULL,
           rate_reservation_id TEXT NOT NULL UNIQUE,
           actor TEXT NOT NULL CHECK (actor = 'system_policy'),
           authorized_at_ms INTEGER NOT NULL
         );",
    )?;
    Ok(())
}

pub(crate) fn apply_rollback_candidate_schema(
    connection: &Connection,
) -> Result<(), DatabaseError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS evolution_curator_rollback_candidates (
           rollback_candidate_id TEXT PRIMARY KEY,
           source_candidate_id TEXT NOT NULL REFERENCES evolution_curator_candidates(candidate_id),
           source_application_id TEXT NOT NULL UNIQUE REFERENCES evolution_curator_applications(application_id),
           probation_id TEXT NOT NULL UNIQUE,
           workspace_id TEXT NOT NULL,
           skill_id TEXT NOT NULL,
           prior_effective_hash TEXT NOT NULL,
           current_effective_hash TEXT NOT NULL,
           observation_witness_hash TEXT NOT NULL,
           urgency TEXT NOT NULL CHECK (urgency IN ('standard','security')),
           status TEXT NOT NULL CHECK (status IN ('pending','reviewed','dismissed')),
           created_at_ms INTEGER NOT NULL
         );",
    )?;
    Ok(())
}
