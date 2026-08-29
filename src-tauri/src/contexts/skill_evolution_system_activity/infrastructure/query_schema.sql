CREATE TABLE IF NOT EXISTS evolution_activity_safe_identities (
    event_id TEXT NOT NULL REFERENCES evolution_activity_envelopes(event_id) ON DELETE CASCADE,
    identity_kind TEXT NOT NULL CHECK (identity_kind IN (
        'workspace','skill','run','evidence','seed','assessment','dossier','generation_job',
        'curator_candidate','application','probation','breaker'
    )),
    identity_value TEXT NOT NULL,
    normalized_value TEXT NOT NULL,
    PRIMARY KEY (event_id, identity_kind, identity_value)
);

CREATE INDEX IF NOT EXISTS idx_evolution_activity_identity_lookup
    ON evolution_activity_safe_identities(identity_kind, normalized_value, event_id);
CREATE INDEX IF NOT EXISTS idx_evolution_activity_envelopes_time
    ON evolution_activity_envelopes(committed_at_ms DESC, event_id);
CREATE INDEX IF NOT EXISTS idx_evolution_activity_envelopes_severity
    ON evolution_activity_envelopes(severity, committed_at_ms DESC, event_id);
CREATE INDEX IF NOT EXISTS idx_evolution_activity_envelopes_domain
    ON evolution_activity_envelopes(source_domain, committed_at_ms DESC, event_id);
CREATE INDEX IF NOT EXISTS idx_evolution_activity_envelopes_status
    ON evolution_activity_envelopes(status, committed_at_ms DESC, event_id);
CREATE INDEX IF NOT EXISTS idx_evolution_activity_envelopes_attention
    ON evolution_activity_envelopes(attention_kind, committed_at_ms DESC, event_id);
CREATE INDEX IF NOT EXISTS idx_evolution_activity_envelopes_event_code
    ON evolution_activity_envelopes(event_code, committed_at_ms DESC, event_id);

CREATE TABLE IF NOT EXISTS evolution_activity_purge_tombstones (
    event_id TEXT PRIMARY KEY REFERENCES evolution_activity_envelopes(event_id) ON DELETE CASCADE,
    purged_source_domain TEXT NOT NULL,
    purged_source_id TEXT NOT NULL,
    detail_unavailable_reason TEXT NOT NULL CHECK (detail_unavailable_reason = 'source_purged'),
    purged_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_activity_notification_requests (
    request_id TEXT PRIMARY KEY,
    event_id TEXT NOT NULL REFERENCES evolution_activity_envelopes(event_id),
    target_scope TEXT NOT NULL,
    request_kind TEXT NOT NULL CHECK (request_kind = 'immediate'),
    status TEXT NOT NULL CHECK (status IN ('pending','opened','dismissed')),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    UNIQUE (event_id, target_scope)
);

CREATE INDEX IF NOT EXISTS idx_evolution_activity_notification_status
    ON evolution_activity_notification_requests(status, created_at_ms DESC);
