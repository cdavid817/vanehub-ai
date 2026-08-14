use rusqlite::Connection;

use crate::platform::database::DatabaseError;

pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS evolution_signals (
            signal_id TEXT PRIMARY KEY,
            source_event_id TEXT NOT NULL,
            source_kind TEXT NOT NULL,
            source_agent_id TEXT,
            extractor_id TEXT NOT NULL,
            extractor_version INTEGER NOT NULL,
            discriminator TEXT NOT NULL,
            deduplication_key TEXT NOT NULL UNIQUE,
            category TEXT NOT NULL,
            polarity TEXT NOT NULL CHECK (polarity IN ('positive', 'negative', 'neutral')),
            severity TEXT NOT NULL CHECK (severity IN ('low', 'medium', 'high')),
            attribution_strength TEXT NOT NULL CHECK (attribution_strength IN ('verified', 'correlated', 'weak', 'unattributed')),
            attribution_rationale TEXT NOT NULL,
            targeting_eligibility TEXT NOT NULL CHECK (targeting_eligibility IN ('automated_consideration', 'human_review_only', 'ineligible')),
            source_fidelity TEXT NOT NULL CHECK (source_fidelity IN ('native', 'proxied', 'inferred', 'opaque')),
            occurred_at TEXT NOT NULL,
            ingested_at TEXT NOT NULL,
            workspace TEXT,
            safe_summary TEXT NOT NULL CHECK (length(safe_summary) <= 1000),
            sanitizer_version INTEGER NOT NULL,
            task_fingerprint_version INTEGER NOT NULL,
            task_fingerprint TEXT NOT NULL,
            association_truncated_count INTEGER NOT NULL DEFAULT 0,
            source_link_truncated_count INTEGER NOT NULL DEFAULT 0,
            lineage_status TEXT NOT NULL DEFAULT 'active' CHECK (lineage_status IN ('active', 'superseded')),
            superseded_by_signal_id TEXT REFERENCES evolution_signals(signal_id) ON DELETE SET NULL,
            UNIQUE (source_event_id, extractor_id, extractor_version, discriminator)
        );

        CREATE TABLE IF NOT EXISTS evolution_signal_receipts (
            source_event_id TEXT NOT NULL,
            extractor_id TEXT NOT NULL,
            extractor_version INTEGER NOT NULL,
            discriminator TEXT NOT NULL,
            signal_id TEXT NOT NULL REFERENCES evolution_signals(signal_id) ON DELETE CASCADE,
            created_at TEXT NOT NULL,
            PRIMARY KEY (source_event_id, extractor_id, extractor_version, discriminator)
        );

        CREATE TABLE IF NOT EXISTS evolution_signal_skill_associations (
            signal_id TEXT NOT NULL REFERENCES evolution_signals(signal_id) ON DELETE CASCADE,
            skill_id TEXT NOT NULL,
            skill_revision TEXT NOT NULL,
            association_kind TEXT NOT NULL,
            observed_at TEXT NOT NULL,
            attribution_strength TEXT NOT NULL,
            attribution_rationale TEXT NOT NULL,
            targeting_eligibility TEXT NOT NULL,
            PRIMARY KEY (signal_id, skill_id, skill_revision, association_kind)
        );

        CREATE TABLE IF NOT EXISTS evolution_signal_source_links (
            signal_id TEXT NOT NULL REFERENCES evolution_signals(signal_id) ON DELETE CASCADE,
            source_kind TEXT NOT NULL,
            source_id TEXT NOT NULL,
            PRIMARY KEY (signal_id, source_kind, source_id)
        );

        CREATE TABLE IF NOT EXISTS evolution_candidate_seeds (
            seed_id TEXT PRIMARY KEY,
            seed_version INTEGER NOT NULL,
            grouping_key_hash TEXT NOT NULL,
            workspace TEXT,
            category TEXT NOT NULL,
            readiness TEXT NOT NULL CHECK (readiness IN ('collecting', 'ready', 'human_review_only', 'ineligible')),
            readiness_reason TEXT NOT NULL,
            safe_summary TEXT NOT NULL CHECK (length(safe_summary) <= 1000),
            first_occurred_at TEXT NOT NULL,
            last_occurred_at TEXT NOT NULL,
            independent_run_count INTEGER NOT NULL,
            signal_set_revision INTEGER NOT NULL,
            has_recovery INTEGER NOT NULL CHECK (has_recovery IN (0, 1)),
            lineage_summary_json TEXT NOT NULL,
            lineage_signal_count INTEGER NOT NULL DEFAULT 0,
            lineage_truncated_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            UNIQUE (grouping_key_hash, seed_version)
        );

        CREATE TABLE IF NOT EXISTS evolution_candidate_seed_signals (
            seed_id TEXT NOT NULL REFERENCES evolution_candidate_seeds(seed_id) ON DELETE CASCADE,
            signal_id TEXT NOT NULL REFERENCES evolution_signals(signal_id) ON DELETE CASCADE,
            lineage_order INTEGER NOT NULL,
            PRIMARY KEY (seed_id, signal_id)
        );

        CREATE TABLE IF NOT EXISTS evolution_feedback_current (
            message_id TEXT PRIMARY KEY,
            feedback_state TEXT NOT NULL CHECK (feedback_state IN ('helpful', 'unhelpful', 'corrected')),
            feedback_revision INTEGER NOT NULL,
            sanitized_note TEXT CHECK (sanitized_note IS NULL OR length(sanitized_note) <= 1000),
            sanitizer_version INTEGER NOT NULL,
            active_signal_id TEXT REFERENCES evolution_signals(signal_id) ON DELETE SET NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS evolution_feedback_events (
            feedback_event_id TEXT PRIMARY KEY,
            message_id TEXT NOT NULL,
            feedback_revision INTEGER NOT NULL,
            previous_state TEXT,
            next_state TEXT,
            sanitized_note TEXT CHECK (sanitized_note IS NULL OR length(sanitized_note) <= 1000),
            sanitizer_version INTEGER NOT NULL,
            signal_id TEXT REFERENCES evolution_signals(signal_id) ON DELETE SET NULL,
            created_at TEXT NOT NULL,
            UNIQUE (message_id, feedback_revision)
        );

        CREATE TABLE IF NOT EXISTS evolution_pipeline_state (
            state_key TEXT PRIMARY KEY,
            state_value TEXT NOT NULL,
            revision INTEGER NOT NULL,
            updated_at TEXT NOT NULL
        );

        INSERT OR IGNORE INTO evolution_pipeline_state (state_key, state_value, revision, updated_at)
        VALUES ('signal_set', 'clean', 0, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));

        CREATE INDEX IF NOT EXISTS idx_evolution_signals_workspace_created
            ON evolution_signals(workspace, ingested_at DESC, signal_id DESC);
        CREATE INDEX IF NOT EXISTS idx_evolution_signals_category_created
            ON evolution_signals(category, ingested_at DESC);
        CREATE INDEX IF NOT EXISTS idx_evolution_signals_fingerprint
            ON evolution_signals(workspace, task_fingerprint_version, task_fingerprint, occurred_at);
        CREATE INDEX IF NOT EXISTS idx_evolution_signals_attribution_created
            ON evolution_signals(attribution_strength, ingested_at DESC);
        CREATE INDEX IF NOT EXISTS idx_evolution_signals_source_agent_created
            ON evolution_signals(source_agent_id, ingested_at DESC);
        CREATE INDEX IF NOT EXISTS idx_evolution_signal_skills_revision
            ON evolution_signal_skill_associations(skill_id, skill_revision, signal_id);
        CREATE INDEX IF NOT EXISTS idx_evolution_signal_skills_attribution
            ON evolution_signal_skill_associations(attribution_strength, signal_id);
        CREATE INDEX IF NOT EXISTS idx_evolution_source_links_lookup
            ON evolution_signal_source_links(source_kind, source_id, signal_id);
        CREATE INDEX IF NOT EXISTS idx_evolution_receipts_signal
            ON evolution_signal_receipts(signal_id);
        CREATE INDEX IF NOT EXISTS idx_evolution_seeds_workspace_readiness
            ON evolution_candidate_seeds(workspace, readiness, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_evolution_seeds_category_created
            ON evolution_candidate_seeds(category, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_evolution_seed_signals_signal
            ON evolution_candidate_seed_signals(signal_id, seed_id);
        CREATE INDEX IF NOT EXISTS idx_evolution_feedback_events_message_created
            ON evolution_feedback_events(message_id, created_at DESC);
        "#,
    )?;
    Ok(())
}
