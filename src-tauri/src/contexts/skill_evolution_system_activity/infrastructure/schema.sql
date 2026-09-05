CREATE TABLE IF NOT EXISTS evolution_system_activity_sessions (
    session_id TEXT PRIMARY KEY,
    schema_version INTEGER NOT NULL CHECK (schema_version = 1),
    activity_kind TEXT NOT NULL CHECK (activity_kind = 'skill_evolution'),
    scope_kind TEXT NOT NULL CHECK (scope_kind IN ('global','workspace')),
    canonical_scope_id TEXT NOT NULL,
    safe_display_identity TEXT,
    active_generation_id TEXT NOT NULL,
    last_sequence INTEGER NOT NULL DEFAULT 0 CHECK (last_sequence >= 0),
    unread_count INTEGER NOT NULL DEFAULT 0 CHECK (unread_count >= 0),
    attention_kind TEXT NOT NULL DEFAULT 'none',
    preference_revision INTEGER NOT NULL DEFAULT 1,
    created_at_ms INTEGER NOT NULL,
    first_activity_at_ms INTEGER NOT NULL,
    last_activity_at_ms INTEGER NOT NULL,
    last_projected_at_ms INTEGER NOT NULL,
    UNIQUE (activity_kind, scope_kind, canonical_scope_id)
);

CREATE TABLE IF NOT EXISTS evolution_activity_envelopes (
    event_id TEXT PRIMARY KEY,
    schema_version INTEGER NOT NULL CHECK (schema_version = 1),
    event_code TEXT NOT NULL,
    source_domain TEXT NOT NULL,
    source_id TEXT NOT NULL,
    source_revision TEXT NOT NULL,
    source_sequence INTEGER NOT NULL CHECK (source_sequence >= 0),
    scope_kind TEXT NOT NULL CHECK (scope_kind IN ('global','workspace')),
    canonical_scope_id TEXT NOT NULL,
    occurred_at_ms INTEGER NOT NULL,
    committed_at_ms INTEGER NOT NULL,
    severity TEXT NOT NULL CHECK (severity IN ('info','warning','error','critical')),
    status TEXT NOT NULL,
    attention_kind TEXT NOT NULL,
    envelope_json TEXT NOT NULL CHECK (length(CAST(envelope_json AS BLOB)) <= 16384),
    payload_json TEXT CHECK (payload_json IS NULL OR length(CAST(payload_json AS BLOB)) <= 8192),
    projection_version INTEGER NOT NULL,
    content_hash TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS evolution_activity_items (
    item_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES evolution_system_activity_sessions(session_id),
    generation_id TEXT NOT NULL,
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    event_id TEXT NOT NULL REFERENCES evolution_activity_envelopes(event_id),
    supersedes_event_id TEXT REFERENCES evolution_activity_envelopes(event_id),
    created_at_ms INTEGER NOT NULL,
    UNIQUE (session_id, generation_id, sequence),
    UNIQUE (session_id, generation_id, event_id)
);

CREATE TRIGGER IF NOT EXISTS evolution_activity_items_immutable
BEFORE UPDATE ON evolution_activity_items
BEGIN
    SELECT RAISE(ABORT, 'system activity items are immutable');
END;

CREATE TABLE IF NOT EXISTS evolution_activity_source_receipts (
    source_domain TEXT NOT NULL,
    source_id TEXT NOT NULL,
    source_revision TEXT NOT NULL,
    event_code TEXT NOT NULL,
    projection_version INTEGER NOT NULL,
    event_id TEXT NOT NULL REFERENCES evolution_activity_envelopes(event_id),
    source_hash TEXT NOT NULL,
    committed_at_ms INTEGER NOT NULL,
    PRIMARY KEY (source_domain, source_id, source_revision, event_code, projection_version)
);

CREATE TABLE IF NOT EXISTS evolution_activity_target_receipts (
    event_id TEXT NOT NULL REFERENCES evolution_activity_envelopes(event_id),
    target_kind TEXT NOT NULL CHECK (target_kind IN ('system_timeline','skill_dashboard','unread_state','notification')),
    target_scope TEXT NOT NULL,
    delivery_status TEXT NOT NULL CHECK (delivery_status IN ('delivered','suppressed','failed')),
    delivered_at_ms INTEGER,
    detail_code TEXT,
    PRIMARY KEY (event_id, target_kind, target_scope)
);

CREATE TABLE IF NOT EXISTS evolution_activity_domain_cursors (
    source_domain TEXT PRIMARY KEY,
    opaque_cursor TEXT,
    last_sequence INTEGER NOT NULL DEFAULT 0,
    last_source_hash TEXT,
    retention_floor TEXT,
    pending_count INTEGER NOT NULL DEFAULT 0,
    oldest_pending_at_ms INTEGER,
    gap_code TEXT,
    failure_code TEXT,
    last_success_at_ms INTEGER,
    revision INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS evolution_activity_dashboard_state (
    scope_kind TEXT NOT NULL,
    canonical_scope_id TEXT NOT NULL,
    generation_id TEXT NOT NULL,
    materialization_kind TEXT NOT NULL,
    state_json TEXT NOT NULL CHECK (length(CAST(state_json AS BLOB)) <= 8192),
    last_event_id TEXT NOT NULL REFERENCES evolution_activity_envelopes(event_id),
    updated_at_ms INTEGER NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (scope_kind, canonical_scope_id, generation_id, materialization_kind)
);

CREATE TABLE IF NOT EXISTS evolution_activity_read_state (
    session_id TEXT NOT NULL REFERENCES evolution_system_activity_sessions(session_id),
    user_id TEXT NOT NULL,
    highest_read_sequence INTEGER NOT NULL DEFAULT 0,
    mark_unread_sequence INTEGER,
    last_seen_at_ms INTEGER NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (session_id, user_id)
);

CREATE TABLE IF NOT EXISTS evolution_activity_preferences (
    scope_kind TEXT NOT NULL,
    canonical_scope_id TEXT NOT NULL,
    visible INTEGER NOT NULL DEFAULT 1 CHECK (visible IN (0,1)),
    minimum_timeline_severity TEXT NOT NULL DEFAULT 'info',
    notification_threshold TEXT NOT NULL DEFAULT 'warning',
    digest_cadence TEXT NOT NULL DEFAULT 'off' CHECK (digest_cadence IN ('off','hourly','daily')),
    read_retention_days INTEGER NOT NULL DEFAULT 180 CHECK (read_retention_days BETWEEN 30 AND 365),
    detail_retention_days INTEGER NOT NULL DEFAULT 180 CHECK (detail_retention_days BETWEEN 30 AND 365),
    export_item_limit INTEGER NOT NULL DEFAULT 1000 CHECK (export_item_limit BETWEEN 1 AND 10000),
    export_size_limit_bytes INTEGER NOT NULL DEFAULT 10485760 CHECK (export_size_limit_bytes > 0),
    revision INTEGER NOT NULL DEFAULT 1,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (scope_kind, canonical_scope_id)
);

CREATE TABLE IF NOT EXISTS evolution_activity_digest_buckets (
    bucket_id TEXT PRIMARY KEY,
    scope_kind TEXT NOT NULL,
    canonical_scope_id TEXT NOT NULL,
    cadence TEXT NOT NULL CHECK (cadence IN ('hourly','daily')),
    window_started_at_ms INTEGER NOT NULL,
    window_ends_at_ms INTEGER NOT NULL,
    counts_json TEXT NOT NULL CHECK (length(CAST(counts_json AS BLOB)) <= 8192),
    highest_severity TEXT NOT NULL,
    delivered_at_ms INTEGER,
    UNIQUE (scope_kind, canonical_scope_id, cadence, window_started_at_ms)
);

CREATE TABLE IF NOT EXISTS evolution_activity_projection_leases (
    lease_key TEXT PRIMARY KEY CHECK (lease_key = 'global'),
    owner_id TEXT NOT NULL,
    expires_at_ms INTEGER NOT NULL,
    heartbeat_at_ms INTEGER NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS evolution_activity_rebuilds (
    rebuild_id TEXT PRIMARY KEY,
    scope_kind TEXT NOT NULL,
    canonical_scope_id TEXT NOT NULL,
    source_snapshot_json TEXT NOT NULL CHECK (length(CAST(source_snapshot_json AS BLOB)) <= 16384),
    source_snapshot_hash TEXT NOT NULL,
    shadow_generation_id TEXT NOT NULL UNIQUE,
    prior_generation_id TEXT NOT NULL,
    status TEXT NOT NULL,
    item_budget INTEGER NOT NULL,
    processed_items INTEGER NOT NULL DEFAULT 0,
    validation_hash TEXT,
    failure_code TEXT,
    revision INTEGER NOT NULL DEFAULT 1,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_activity_rebuild_checkpoints (
    rebuild_id TEXT NOT NULL REFERENCES evolution_activity_rebuilds(rebuild_id) ON DELETE CASCADE,
    source_domain TEXT NOT NULL,
    opaque_cursor TEXT,
    high_watermark TEXT NOT NULL,
    processed_items INTEGER NOT NULL DEFAULT 0,
    receipt_hash TEXT NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    PRIMARY KEY (rebuild_id, source_domain)
);

CREATE TABLE IF NOT EXISTS evolution_activity_exports (
    export_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES evolution_system_activity_sessions(session_id),
    generation_id TEXT NOT NULL,
    format TEXT NOT NULL CHECK (format IN ('json','markdown')),
    filters_json TEXT NOT NULL CHECK (length(CAST(filters_json AS BLOB)) <= 8192),
    item_count INTEGER NOT NULL,
    byte_count INTEGER NOT NULL,
    complete INTEGER NOT NULL CHECK (complete IN (0,1)),
    redaction_version TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_evolution_activity_items_timeline
    ON evolution_activity_items(session_id, generation_id, sequence DESC);
CREATE INDEX IF NOT EXISTS idx_evolution_activity_envelopes_filter
    ON evolution_activity_envelopes(scope_kind, canonical_scope_id, committed_at_ms DESC, severity, source_domain, status);
CREATE INDEX IF NOT EXISTS idx_evolution_activity_envelopes_source
    ON evolution_activity_envelopes(source_domain, source_id, source_sequence, event_id);
CREATE INDEX IF NOT EXISTS idx_evolution_activity_target_delivery
    ON evolution_activity_target_receipts(target_kind, delivery_status, delivered_at_ms);
