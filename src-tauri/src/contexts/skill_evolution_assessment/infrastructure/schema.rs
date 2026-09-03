use crate::platform::database::DatabaseError;
use rusqlite::Connection;

pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS evolution_assessment_attempts (
            attempt_id TEXT PRIMARY KEY,
            seed_id TEXT NOT NULL REFERENCES evolution_candidate_seeds(seed_id) ON DELETE CASCADE,
            seed_revision TEXT NOT NULL,
            witness_hash TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('pending','running','completed','failed','superseded')),
            classification TEXT,
            route TEXT,
            confidence TEXT,
            risk TEXT,
            seed_fingerprint TEXT NOT NULL,
            lineage_hash TEXT NOT NULL,
            target_universe_hash TEXT NOT NULL,
            sanitizer_version TEXT NOT NULL,
            selector_policy_version TEXT NOT NULL,
            lexical_policy_version TEXT NOT NULL,
            gate_policy_version TEXT NOT NULL,
            routing_policy_version TEXT NOT NULL,
            confidence_policy_version TEXT NOT NULL,
            evaluator_config_hash TEXT,
            consent_version TEXT NOT NULL,
            model_evaluation_allowed INTEGER NOT NULL CHECK (model_evaluation_allowed IN (0,1)),
            is_current INTEGER NOT NULL DEFAULT 1 CHECK (is_current IN (0,1)),
            winning_rule TEXT,
            normalized_explanation_json TEXT,
            lease_owner TEXT,
            lease_expires_at_ms INTEGER,
            heartbeat_at_ms INTEGER,
            created_at_ms INTEGER NOT NULL,
            completed_at_ms INTEGER,
            UNIQUE (seed_id, seed_revision, witness_hash)
        );

        CREATE TABLE IF NOT EXISTS evolution_assessment_targets (
            attempt_id TEXT NOT NULL REFERENCES evolution_assessment_attempts(attempt_id) ON DELETE CASCADE,
            ordinal INTEGER NOT NULL,
            skill_id TEXT NOT NULL,
            skill_type TEXT NOT NULL,
            revision_hash TEXT NOT NULL,
            scope TEXT NOT NULL,
            lifecycle TEXT NOT NULL,
            trust TEXT NOT NULL,
            score INTEGER NOT NULL CHECK (score BETWEEN 0 AND 100),
            attribution_uncertain INTEGER NOT NULL CHECK (attribution_uncertain IN (0,1)),
            matched_feature_classes_json TEXT NOT NULL,
            exclusions_json TEXT NOT NULL,
            PRIMARY KEY (attempt_id, ordinal)
        );

        CREATE TABLE IF NOT EXISTS evolution_assessment_score_components (
            attempt_id TEXT NOT NULL,
            target_ordinal INTEGER NOT NULL,
            component TEXT NOT NULL,
            score INTEGER NOT NULL,
            PRIMARY KEY (attempt_id, target_ordinal, component),
            FOREIGN KEY (attempt_id, target_ordinal)
                REFERENCES evolution_assessment_targets(attempt_id, ordinal) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS evolution_assessment_checks (
            attempt_id TEXT NOT NULL REFERENCES evolution_assessment_attempts(attempt_id) ON DELETE CASCADE,
            ordinal INTEGER NOT NULL CHECK (ordinal BETWEEN 0 AND 8),
            kind TEXT NOT NULL,
            result TEXT NOT NULL CHECK (result IN ('pass','fail','review','not_applicable')),
            severity TEXT NOT NULL CHECK (severity IN ('low','medium','high')),
            reason_code TEXT NOT NULL,
            evidence_ids_json TEXT NOT NULL,
            route_constraints_json TEXT NOT NULL,
            PRIMARY KEY (attempt_id, ordinal),
            UNIQUE (attempt_id, kind)
        );

        CREATE TABLE IF NOT EXISTS evolution_assessment_evidence_links (
            attempt_id TEXT NOT NULL REFERENCES evolution_assessment_attempts(attempt_id) ON DELETE CASCADE,
            evidence_id TEXT NOT NULL,
            relation TEXT NOT NULL,
            PRIMARY KEY (attempt_id, evidence_id, relation)
        );

        CREATE TABLE IF NOT EXISTS evolution_assessment_model_calls (
            model_call_id TEXT PRIMARY KEY,
            attempt_id TEXT NOT NULL REFERENCES evolution_assessment_attempts(attempt_id) ON DELETE CASCADE,
            stage TEXT NOT NULL CHECK (stage IN ('target_consultation','quality_judge')),
            request_projection_hash TEXT NOT NULL,
            profile_id TEXT,
            provider_protocol TEXT,
            model_id TEXT,
            template_version TEXT NOT NULL,
            response_schema_version TEXT NOT NULL,
            outcome_category TEXT NOT NULL,
            sanitized_response_json TEXT CHECK (sanitized_response_json IS NULL OR length(sanitized_response_json) <= 8192),
            input_tokens INTEGER,
            output_tokens INTEGER,
            latency_ms INTEGER NOT NULL,
            created_at_ms INTEGER NOT NULL,
            UNIQUE (attempt_id, stage)
        );

        CREATE TABLE IF NOT EXISTS evolution_assessment_supersessions (
            prior_attempt_id TEXT PRIMARY KEY REFERENCES evolution_assessment_attempts(attempt_id) ON DELETE CASCADE,
            replacement_attempt_id TEXT NOT NULL REFERENCES evolution_assessment_attempts(attempt_id) ON DELETE CASCADE,
            reason_code TEXT NOT NULL,
            changed_witness_hash TEXT NOT NULL,
            created_at_ms INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS evolution_assessment_policy (
            policy_id INTEGER PRIMARY KEY CHECK (policy_id = 1),
            evaluator_policy_version TEXT NOT NULL,
            disclosure_version TEXT NOT NULL,
            model_evaluation_enabled INTEGER NOT NULL CHECK (model_evaluation_enabled IN (0,1)),
            changed_at_ms INTEGER NOT NULL,
            local_actor TEXT NOT NULL CHECK (length(local_actor) <= 128)
        );

        INSERT OR IGNORE INTO evolution_assessment_policy (
            policy_id, evaluator_policy_version, disclosure_version,
            model_evaluation_enabled, changed_at_ms, local_actor
        ) VALUES (1, 'structured-evaluator-v1', 'assessment-disclosure-v1', 0, 0, 'system_default');

        CREATE TABLE IF NOT EXISTS evolution_assessment_queue_state (
            queue_id TEXT PRIMARY KEY,
            seed_id TEXT NOT NULL REFERENCES evolution_candidate_seeds(seed_id) ON DELETE CASCADE,
            witness_hash TEXT NOT NULL,
            lane TEXT NOT NULL CHECK (lane IN ('deterministic','optional_model')),
            status TEXT NOT NULL CHECK (status IN ('queued','leased','completed','failed','fallback')),
            priority INTEGER NOT NULL,
            attempt_count INTEGER NOT NULL DEFAULT 0,
            available_at_ms INTEGER NOT NULL,
            lease_owner TEXT,
            lease_expires_at_ms INTEGER,
            created_at_ms INTEGER NOT NULL,
            updated_at_ms INTEGER NOT NULL,
            UNIQUE (seed_id, witness_hash, lane)
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_evolution_assessment_current_seed
            ON evolution_assessment_attempts(seed_id) WHERE is_current = 1;
        CREATE INDEX IF NOT EXISTS idx_evolution_assessment_attempts_seed_created
            ON evolution_assessment_attempts(seed_id, created_at_ms DESC, attempt_id DESC);
        CREATE INDEX IF NOT EXISTS idx_evolution_assessment_targets_skill
            ON evolution_assessment_targets(skill_id, revision_hash, attempt_id);
        CREATE INDEX IF NOT EXISTS idx_evolution_assessment_evidence
            ON evolution_assessment_evidence_links(evidence_id, attempt_id);
        CREATE INDEX IF NOT EXISTS idx_evolution_assessment_queue_ready
            ON evolution_assessment_queue_state(status, lane, priority DESC, available_at_ms, queue_id);
        "#,
    )?;
    ensure_column(
        connection,
        "evolution_assessment_targets",
        "skill_type",
        "TEXT NOT NULL DEFAULT 'unknown'",
    )?;
    Ok(())
}

fn ensure_column(
    connection: &Connection,
    table: &str,
    column: &str,
    declaration: &str,
) -> Result<(), DatabaseError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }
    connection.execute_batch(&format!(
        "ALTER TABLE {table} ADD COLUMN {column} {declaration}"
    ))?;
    Ok(())
}
