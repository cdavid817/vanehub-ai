CREATE TABLE IF NOT EXISTS evolution_curator_candidates (
    candidate_id TEXT PRIMARY KEY,
    schema_version INTEGER NOT NULL CHECK (schema_version = 1),
    workspace_id TEXT NOT NULL,
    seed_id TEXT NOT NULL,
    seed_revision TEXT NOT NULL,
    assessment_attempt_id TEXT NOT NULL REFERENCES evolution_assessment_attempts(attempt_id),
    assessment_revision TEXT NOT NULL,
    target_skill_id TEXT NOT NULL,
    target_revision TEXT NOT NULL,
    overlay_scope TEXT NOT NULL CHECK (overlay_scope IN ('project','user')),
    route TEXT NOT NULL CHECK (route IN ('advance','needs_human_review')),
    risk TEXT NOT NULL CHECK (risk IN ('low','medium','high')),
    confidence TEXT NOT NULL CHECK (confidence IN ('low','medium','high')),
    policy_witness_hash TEXT NOT NULL,
    witness_hash TEXT NOT NULL,
    snapshot_json TEXT NOT NULL CHECK (length(CAST(snapshot_json AS BLOB)) <= 16384),
    state TEXT NOT NULL CHECK (state IN ('pending','awaiting_draft','ready_for_review','deferred','rejected','applying','applied','apply_failed','superseded')),
    staleness_json TEXT NOT NULL DEFAULT '[]' CHECK (length(CAST(staleness_json AS BLOB)) <= 4096),
    current_draft_id TEXT,
    current_preview_id TEXT,
    superseded_by_candidate_id TEXT REFERENCES evolution_curator_candidates(candidate_id),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    UNIQUE (assessment_attempt_id, assessment_revision, target_revision, witness_hash)
);

CREATE TABLE IF NOT EXISTS evolution_curator_intake_receipts (
    envelope_hash TEXT PRIMARY KEY,
    assessment_attempt_id TEXT NOT NULL,
    assessment_revision TEXT NOT NULL,
    route TEXT NOT NULL CHECK (route IN ('advance','drop','record_memory_only','merge_duplicate','needs_human_review')),
    witness_hash TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK (outcome IN ('candidate_created','non_approvable','non_current','purged_evidence')),
    candidate_id TEXT REFERENCES evolution_curator_candidates(candidate_id),
    received_at_ms INTEGER NOT NULL,
    UNIQUE (assessment_attempt_id, assessment_revision, witness_hash)
);

CREATE TABLE IF NOT EXISTS evolution_curator_candidate_sources (
    candidate_id TEXT NOT NULL REFERENCES evolution_curator_candidates(candidate_id) ON DELETE CASCADE,
    evidence_id TEXT NOT NULL,
    evidence_revision TEXT NOT NULL,
    lineage_hash TEXT NOT NULL,
    redacted_at_ms INTEGER,
    PRIMARY KEY (candidate_id, evidence_id, evidence_revision)
);

CREATE TABLE IF NOT EXISTS evolution_curator_drafts (
    draft_id TEXT NOT NULL,
    candidate_id TEXT NOT NULL REFERENCES evolution_curator_candidates(candidate_id) ON DELETE CASCADE,
    revision INTEGER NOT NULL CHECK (revision > 0),
    kind TEXT NOT NULL CHECK (kind IN ('learn_block','exact_patch')),
    target_skill_id TEXT NOT NULL,
    target_revision TEXT NOT NULL,
    overlay_scope TEXT NOT NULL CHECK (overlay_scope IN ('project','user')),
    validated_body_json TEXT CHECK (validated_body_json IS NULL OR length(CAST(validated_body_json AS BLOB)) <= 16384),
    body_hash TEXT NOT NULL,
    rationale TEXT NOT NULL CHECK (length(CAST(rationale AS BLOB)) <= 2048),
    expected_effective_change TEXT NOT NULL CHECK (length(CAST(expected_effective_change AS BLOB)) <= 2048),
    evidence_ids_json TEXT NOT NULL CHECK (length(CAST(evidence_ids_json AS BLOB)) <= 4096),
    scanner_version TEXT NOT NULL,
    base_hash TEXT NOT NULL,
    base_package_hash TEXT NOT NULL,
    effective_hash TEXT NOT NULL,
    overlay_revision INTEGER,
    pin_witness TEXT NOT NULL,
    trust_witness TEXT NOT NULL,
    conflict_witness TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    PRIMARY KEY (draft_id, revision),
    UNIQUE (candidate_id, revision)
);

CREATE TABLE IF NOT EXISTS evolution_curator_draft_assessments (
    draft_assessment_id TEXT PRIMARY KEY,
    candidate_id TEXT NOT NULL REFERENCES evolution_curator_candidates(candidate_id) ON DELETE CASCADE,
    candidate_revision INTEGER NOT NULL,
    draft_id TEXT NOT NULL,
    draft_revision INTEGER NOT NULL,
    draft_hash TEXT NOT NULL,
    candidate_witness_hash TEXT NOT NULL,
    target_skill_id TEXT NOT NULL,
    target_revision TEXT NOT NULL,
    checks_json TEXT NOT NULL CHECK (length(CAST(checks_json AS BLOB)) <= 16384),
    approvable INTEGER NOT NULL CHECK (approvable IN (0,1)),
    model_evaluation_allowed INTEGER NOT NULL CHECK (model_evaluation_allowed IN (0,1)),
    model_consulted INTEGER NOT NULL CHECK (model_consulted IN (0,1)),
    model_fallback_reason TEXT,
    witness_hash TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    invalidated_at_ms INTEGER,
    FOREIGN KEY (draft_id, draft_revision) REFERENCES evolution_curator_drafts(draft_id, revision),
    UNIQUE (candidate_id, draft_id, draft_revision, draft_hash)
);

CREATE TABLE IF NOT EXISTS evolution_curator_previews (
    preview_id TEXT PRIMARY KEY,
    candidate_id TEXT NOT NULL REFERENCES evolution_curator_candidates(candidate_id) ON DELETE CASCADE,
    candidate_revision INTEGER NOT NULL,
    draft_id TEXT NOT NULL,
    draft_revision INTEGER NOT NULL,
    draft_assessment_id TEXT NOT NULL REFERENCES evolution_curator_draft_assessments(draft_assessment_id),
    witness_hash TEXT NOT NULL,
    effective_diff_hash TEXT NOT NULL,
    witnesses_json TEXT NOT NULL CHECK (length(CAST(witnesses_json AS BLOB)) <= 16384),
    diff_projection_json TEXT NOT NULL CHECK (length(CAST(diff_projection_json AS BLOB)) <= 65536),
    validation_json TEXT NOT NULL CHECK (length(CAST(validation_json AS BLOB)) <= 16384),
    issued_at_ms INTEGER NOT NULL,
    expires_at_ms INTEGER NOT NULL,
    invalidated_at_ms INTEGER,
    FOREIGN KEY (draft_id, draft_revision) REFERENCES evolution_curator_drafts(draft_id, revision)
);

CREATE TABLE IF NOT EXISTS evolution_curator_decisions (
    decision_id TEXT PRIMARY KEY,
    candidate_id TEXT NOT NULL REFERENCES evolution_curator_candidates(candidate_id),
    candidate_revision INTEGER NOT NULL,
    decision_kind TEXT NOT NULL CHECK (decision_kind IN ('approve','reject','defer','resume')),
    actor_class TEXT NOT NULL CHECK (actor_class IN ('local_interactive_user','system','web_mock_interactive_user')),
    reason_code TEXT NOT NULL,
    note_hash TEXT,
    preview_hash TEXT,
    review_after_ms INTEGER,
    idempotency_key TEXT NOT NULL,
    decided_at_ms INTEGER NOT NULL,
    UNIQUE (candidate_id, decision_kind, idempotency_key)
);

CREATE TABLE IF NOT EXISTS evolution_curator_events (
    candidate_id TEXT NOT NULL REFERENCES evolution_curator_candidates(candidate_id),
    sequence INTEGER NOT NULL CHECK (sequence > 0),
    event_kind TEXT NOT NULL,
    actor_class TEXT NOT NULL,
    occurred_at_ms INTEGER NOT NULL,
    prior_state TEXT,
    next_state TEXT NOT NULL,
    object_revision INTEGER NOT NULL,
    reason_code TEXT,
    prior_hash TEXT,
    event_hash TEXT NOT NULL,
    PRIMARY KEY (candidate_id, sequence),
    UNIQUE (candidate_id, event_hash)
);

CREATE TRIGGER IF NOT EXISTS evolution_curator_events_immutable
BEFORE UPDATE ON evolution_curator_events
BEGIN
    SELECT RAISE(ABORT, 'curator audit events are immutable');
END;

CREATE TABLE IF NOT EXISTS evolution_curator_applications (
    application_id TEXT PRIMARY KEY,
    candidate_id TEXT NOT NULL REFERENCES evolution_curator_candidates(candidate_id),
    decision_id TEXT NOT NULL UNIQUE REFERENCES evolution_curator_decisions(decision_id),
    status TEXT NOT NULL CHECK (status IN ('intent_recorded','applying','applied','failed','reconciled')),
    approved_witness_hash TEXT NOT NULL,
    overlay_revision TEXT,
    overlay_history_id TEXT,
    failure_code TEXT,
    revision INTEGER NOT NULL DEFAULT 1,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_curator_outbox (
    outbox_id TEXT PRIMARY KEY,
    application_id TEXT NOT NULL UNIQUE REFERENCES evolution_curator_applications(application_id),
    candidate_id TEXT NOT NULL REFERENCES evolution_curator_candidates(candidate_id),
    witness_hash TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending','processing','completed','failed')),
    attempt_count INTEGER NOT NULL DEFAULT 0,
    available_at_ms INTEGER NOT NULL,
    lease_owner TEXT,
    lease_expires_at_ms INTEGER,
    completed_at_ms INTEGER,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_curator_system_policy_authorizations (
    application_id TEXT PRIMARY KEY REFERENCES evolution_curator_applications(application_id),
    run_id TEXT NOT NULL,
    eligibility_id TEXT NOT NULL,
    eligibility_proof_hash TEXT NOT NULL,
    preflight_witness_hash TEXT NOT NULL UNIQUE,
    policy_witness_hash TEXT NOT NULL,
    rate_reservation_id TEXT NOT NULL UNIQUE,
    actor TEXT NOT NULL CHECK (actor = 'system_policy'),
    authorized_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_curator_rollback_candidates (
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
);

CREATE TABLE IF NOT EXISTS evolution_curator_policy (
    workspace_id TEXT PRIMARY KEY,
    schema_version INTEGER NOT NULL CHECK (schema_version = 1),
    policy_json TEXT NOT NULL CHECK (length(CAST(policy_json AS BLOB)) <= 8192),
    policy_hash TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_curator_notification_receipts (
    candidate_id TEXT NOT NULL REFERENCES evolution_curator_candidates(candidate_id),
    candidate_revision INTEGER NOT NULL,
    event_kind TEXT NOT NULL,
    delivery_status TEXT NOT NULL CHECK (delivery_status IN ('pending','delivered','failed','suppressed')),
    delivered_at_ms INTEGER,
    PRIMARY KEY (candidate_id, candidate_revision, event_kind)
);

CREATE INDEX IF NOT EXISTS idx_evolution_curator_queue
    ON evolution_curator_candidates(workspace_id, state, risk, updated_at_ms, candidate_id);
CREATE INDEX IF NOT EXISTS idx_evolution_curator_target
    ON evolution_curator_candidates(workspace_id, target_skill_id, state);
CREATE INDEX IF NOT EXISTS idx_evolution_curator_outbox_ready
    ON evolution_curator_outbox(status, available_at_ms, outbox_id);
CREATE INDEX IF NOT EXISTS idx_evolution_curator_events_order
    ON evolution_curator_events(candidate_id, sequence);
