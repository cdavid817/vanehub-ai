CREATE TABLE IF NOT EXISTS evolution_generation_policy (
    workspace_id TEXT PRIMARY KEY,
    schema_version INTEGER NOT NULL CHECK (schema_version = 1),
    consent_state TEXT NOT NULL CHECK (consent_state IN ('disabled','enabled','revoked','disclosure_stale')),
    disclosure_version TEXT NOT NULL,
    provider_profile_id TEXT,
    job_budget_json TEXT NOT NULL CHECK (json_valid(job_budget_json) AND length(CAST(job_budget_json AS BLOB)) <= 4096),
    daily_budget_json TEXT NOT NULL CHECK (json_valid(daily_budget_json) AND length(CAST(daily_budget_json AS BLOB)) <= 4096),
    retention_json TEXT NOT NULL CHECK (json_valid(retention_json) AND length(CAST(retention_json AS BLOB)) <= 4096),
    policy_json TEXT NOT NULL CHECK (json_valid(policy_json) AND length(CAST(policy_json AS BLOB)) <= 16384),
    policy_hash TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_generation_jobs (
    job_id TEXT PRIMARY KEY,
    schema_version INTEGER NOT NULL CHECK (schema_version = 1),
    request_id TEXT NOT NULL UNIQUE,
    workspace_id TEXT,
    seed_id TEXT NOT NULL REFERENCES evolution_candidate_seeds(seed_id),
    seed_revision TEXT NOT NULL,
    assessment_attempt_id TEXT NOT NULL REFERENCES evolution_assessment_attempts(attempt_id),
    assessment_revision TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('requested','blocked_consent','queued','running','cancel_requested','cancelled','failed','completed','superseded')),
    current_stage TEXT CHECK (current_stage IS NULL OR current_stage IN ('freeze_input','inspect_target','build_dossier','plan_mutation','synthesize_structured_draft','validate_and_simulate','package_for_governance')),
    input_witness_json TEXT NOT NULL CHECK (json_valid(input_witness_json) AND length(CAST(input_witness_json AS BLOB)) <= 32768),
    input_witness_hash TEXT NOT NULL,
    current_attempt INTEGER NOT NULL DEFAULT 1 CHECK (current_attempt > 0),
    budget_json TEXT NOT NULL CHECK (json_valid(budget_json) AND length(CAST(budget_json AS BLOB)) <= 4096),
    usage_json TEXT NOT NULL CHECK (json_valid(usage_json) AND length(CAST(usage_json AS BLOB)) <= 4096),
    safe_failure_code TEXT,
    cancel_requested_at_ms INTEGER,
    supersedes_job_id TEXT REFERENCES evolution_generation_jobs(job_id),
    superseded_by_job_id TEXT REFERENCES evolution_generation_jobs(job_id),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    UNIQUE (input_witness_hash, current_attempt)
);

CREATE TABLE IF NOT EXISTS evolution_generation_job_sources (
    job_id TEXT NOT NULL REFERENCES evolution_generation_jobs(job_id) ON DELETE CASCADE,
    source_kind TEXT NOT NULL,
    source_id TEXT NOT NULL,
    source_revision TEXT NOT NULL,
    source_hash TEXT NOT NULL,
    redacted_at_ms INTEGER,
    PRIMARY KEY (job_id, source_kind, source_id, source_revision)
);

CREATE TABLE IF NOT EXISTS evolution_evidence_dossiers (
    dossier_id TEXT PRIMARY KEY,
    schema_version INTEGER NOT NULL CHECK (schema_version = 1),
    revision INTEGER NOT NULL CHECK (revision > 0),
    input_witness_hash TEXT NOT NULL,
    builder_version TEXT NOT NULL,
    sanitizer_version TEXT NOT NULL,
    canonical_size_bytes INTEGER NOT NULL CHECK (canonical_size_bytes >= 0),
    content_hash TEXT NOT NULL UNIQUE,
    supersedes_dossier_id TEXT REFERENCES evolution_evidence_dossiers(dossier_id),
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_evidence_dossier_sections (
    dossier_id TEXT NOT NULL REFERENCES evolution_evidence_dossiers(dossier_id) ON DELETE CASCADE,
    ordinal INTEGER NOT NULL CHECK (ordinal BETWEEN 0 AND 12),
    section_kind TEXT NOT NULL CHECK (section_kind IN ('identity_and_provenance','executive_summary','candidate_seed','source_signal_inventory','attribution_and_target_selection','assessment_and_quality_gates','current_effective_skill_snapshot','relevant_guidance_and_resource_context','failure_recovery_and_verification_timeline','privacy_and_redaction_report','proposed_mutation_rationale','verification_plan','lineage_and_version_witnesses')),
    status TEXT NOT NULL CHECK (status IN ('complete','partial','not_applicable','unavailable','redacted')),
    source_witnesses_json TEXT NOT NULL CHECK (json_valid(source_witnesses_json) AND length(CAST(source_witnesses_json AS BLOB)) <= 16384),
    records_json TEXT NOT NULL CHECK (json_valid(records_json) AND length(CAST(records_json AS BLOB)) <= 65536),
    truncation_json TEXT NOT NULL CHECK (json_valid(truncation_json) AND length(CAST(truncation_json AS BLOB)) <= 4096),
    unavailable_reason_code TEXT,
    section_hash TEXT NOT NULL,
    PRIMARY KEY (dossier_id, ordinal),
    UNIQUE (dossier_id, section_kind)
);

CREATE TRIGGER IF NOT EXISTS evolution_evidence_dossier_sections_order
BEFORE INSERT ON evolution_evidence_dossier_sections
WHEN NEW.section_kind <> CASE NEW.ordinal
    WHEN 0 THEN 'identity_and_provenance'
    WHEN 1 THEN 'executive_summary'
    WHEN 2 THEN 'candidate_seed'
    WHEN 3 THEN 'source_signal_inventory'
    WHEN 4 THEN 'attribution_and_target_selection'
    WHEN 5 THEN 'assessment_and_quality_gates'
    WHEN 6 THEN 'current_effective_skill_snapshot'
    WHEN 7 THEN 'relevant_guidance_and_resource_context'
    WHEN 8 THEN 'failure_recovery_and_verification_timeline'
    WHEN 9 THEN 'privacy_and_redaction_report'
    WHEN 10 THEN 'proposed_mutation_rationale'
    WHEN 11 THEN 'verification_plan'
    WHEN 12 THEN 'lineage_and_version_witnesses'
END
BEGIN
    SELECT RAISE(ABORT, 'dossier section order is invalid');
END;

CREATE TABLE IF NOT EXISTS evolution_evidence_dossier_links (
    dossier_id TEXT NOT NULL REFERENCES evolution_evidence_dossiers(dossier_id) ON DELETE CASCADE,
    link_kind TEXT NOT NULL CHECK (link_kind IN ('job','assessment','evidence','effective_skill','curator_candidate')),
    linked_id TEXT NOT NULL,
    linked_revision TEXT NOT NULL,
    witness_hash TEXT NOT NULL,
    PRIMARY KEY (dossier_id, link_kind, linked_id, linked_revision)
);

CREATE TABLE IF NOT EXISTS evolution_generation_stage_attempts (
    attempt_id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES evolution_generation_jobs(job_id) ON DELETE CASCADE,
    stage TEXT NOT NULL CHECK (stage IN ('freeze_input','inspect_target','build_dossier','plan_mutation','synthesize_structured_draft','validate_and_simulate','package_for_governance')),
    attempt INTEGER NOT NULL CHECK (attempt > 0),
    status TEXT NOT NULL CHECK (status IN ('pending','running','succeeded','failed','cancelled','superseded')),
    input_hash TEXT NOT NULL,
    output_hash TEXT,
    usage_json TEXT NOT NULL CHECK (json_valid(usage_json) AND length(CAST(usage_json AS BLOB)) <= 4096),
    safe_failure_code TEXT,
    started_at_ms INTEGER NOT NULL,
    completed_at_ms INTEGER,
    superseded_by_attempt_id TEXT REFERENCES evolution_generation_stage_attempts(attempt_id),
    UNIQUE (job_id, stage, attempt)
);

CREATE TABLE IF NOT EXISTS evolution_generation_model_calls (
    model_call_id TEXT PRIMARY KEY,
    stage_attempt_id TEXT NOT NULL REFERENCES evolution_generation_stage_attempts(attempt_id) ON DELETE CASCADE,
    purpose TEXT NOT NULL,
    provider_protocol TEXT,
    provider_profile_id TEXT,
    model_id TEXT,
    prompt_template_version TEXT NOT NULL,
    response_schema_version TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK (outcome IN ('valid','provider_unavailable','timeout','rate_limited','malformed_json','invalid_schema','oversized_output','consent_lost','provider_failure')),
    input_tokens INTEGER NOT NULL DEFAULT 0 CHECK (input_tokens >= 0),
    output_tokens INTEGER NOT NULL DEFAULT 0 CHECK (output_tokens >= 0),
    latency_ms INTEGER NOT NULL CHECK (latency_ms >= 0),
    structured_response_hash TEXT,
    safe_failure_code TEXT,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_generation_tool_receipts (
    receipt_id TEXT PRIMARY KEY,
    stage_attempt_id TEXT NOT NULL REFERENCES evolution_generation_stage_attempts(attempt_id) ON DELETE CASCADE,
    tool_name TEXT NOT NULL CHECK (tool_name IN ('read_dossier_section','read_skill_excerpt','find_exact_anchor','validate_draft_structure','simulate_local_preview','get_assessment','get_evidence_dossier_section','get_effective_skill','preview_overlay','preview_skill_creation')),
    argument_hash TEXT NOT NULL,
    source_witness_hash TEXT NOT NULL,
    outcome TEXT NOT NULL CHECK (outcome IN ('succeeded','stale_witness','invalid_argument','result_too_large','budget_exceeded','policy_denied','failed')),
    result_hash TEXT,
    safe_failure_code TEXT,
    duration_ms INTEGER NOT NULL CHECK (duration_ms >= 0),
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_generation_structured_results (
    result_id TEXT PRIMARY KEY,
    stage_attempt_id TEXT NOT NULL UNIQUE REFERENCES evolution_generation_stage_attempts(attempt_id) ON DELETE CASCADE,
    schema_version INTEGER NOT NULL CHECK (schema_version = 1),
    mutation_plan_json TEXT NOT NULL CHECK (json_valid(mutation_plan_json) AND length(CAST(mutation_plan_json AS BLOB)) <= 32768),
    structured_draft_json TEXT NOT NULL CHECK (json_valid(structured_draft_json) AND length(CAST(structured_draft_json AS BLOB)) <= 65536),
    result_hash TEXT NOT NULL UNIQUE,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_generated_drafts (
    draft_id TEXT NOT NULL,
    job_id TEXT NOT NULL REFERENCES evolution_generation_jobs(job_id) ON DELETE CASCADE,
    generation_attempt INTEGER NOT NULL CHECK (generation_attempt > 0),
    artifact_kind TEXT NOT NULL CHECK (artifact_kind IN ('overlay_learn_block','overlay_exact_patch','new_skill')),
    renderer_version TEXT NOT NULL,
    media_type TEXT NOT NULL,
    rendered_content TEXT NOT NULL CHECK (length(CAST(rendered_content AS BLOB)) <= 131072),
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
    content_hash TEXT NOT NULL,
    permanently_manual INTEGER NOT NULL DEFAULT 1 CHECK (permanently_manual = 1),
    created_at_ms INTEGER NOT NULL,
    PRIMARY KEY (draft_id, generation_attempt),
    UNIQUE (job_id, generation_attempt)
);

CREATE TABLE IF NOT EXISTS evolution_generation_validations (
    validation_id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES evolution_generation_jobs(job_id) ON DELETE CASCADE,
    draft_id TEXT NOT NULL,
    draft_attempt INTEGER NOT NULL,
    validator_version TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending','passed','failed','repairable','superseded')),
    checks_json TEXT NOT NULL CHECK (json_valid(checks_json) AND length(CAST(checks_json AS BLOB)) <= 32768),
    preview_witness_hash TEXT,
    report_hash TEXT NOT NULL,
    repair_attempt INTEGER NOT NULL DEFAULT 0 CHECK (repair_attempt >= 0),
    created_at_ms INTEGER NOT NULL,
    FOREIGN KEY (draft_id, draft_attempt) REFERENCES evolution_generated_drafts(draft_id, generation_attempt),
    UNIQUE (draft_id, draft_attempt, repair_attempt)
);

CREATE TABLE IF NOT EXISTS evolution_generation_handoffs (
    handoff_id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES evolution_generation_jobs(job_id),
    validation_id TEXT NOT NULL REFERENCES evolution_generation_validations(validation_id),
    curator_candidate_id TEXT REFERENCES evolution_curator_candidates(candidate_id),
    package_json TEXT NOT NULL CHECK (json_valid(package_json) AND length(CAST(package_json AS BLOB)) <= 32768),
    package_hash TEXT NOT NULL UNIQUE,
    status TEXT NOT NULL CHECK (status IN ('pending','delivered','duplicate','failed','superseded')),
    safe_failure_code TEXT,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_generated_skill_quarantine (
    proposal_id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL UNIQUE REFERENCES evolution_generation_jobs(job_id),
    status TEXT NOT NULL CHECK (status IN ('pending_validation','quarantined','reviewable','rejected','applied','purged','superseded')),
    candidate_id TEXT NOT NULL,
    scope TEXT NOT NULL CHECK (scope IN ('user','project')),
    workspace_id TEXT,
    rendered_skill_md TEXT NOT NULL CHECK (length(CAST(rendered_skill_md AS BLOB)) <= 131072),
    artifact_hash TEXT NOT NULL,
    catalog_witness_hash TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_generation_exports (
    export_id TEXT PRIMARY KEY,
    dossier_id TEXT NOT NULL REFERENCES evolution_evidence_dossiers(dossier_id),
    format TEXT NOT NULL CHECK (format IN ('json','markdown')),
    schema_version INTEGER NOT NULL CHECK (schema_version = 1),
    complete INTEGER NOT NULL CHECK (complete IN (0,1)),
    redaction_manifest_hash TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_generation_governance_tombstones (
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
);

CREATE TRIGGER IF NOT EXISTS evolution_generation_stage_attempts_immutable
BEFORE UPDATE ON evolution_generation_stage_attempts
WHEN OLD.status IN ('succeeded','failed','cancelled','superseded')
  OR NEW.attempt_id <> OLD.attempt_id
  OR NEW.job_id <> OLD.job_id
  OR NEW.stage <> OLD.stage
  OR NEW.attempt <> OLD.attempt
  OR NEW.input_hash <> OLD.input_hash
  OR NEW.started_at_ms <> OLD.started_at_ms
BEGIN
    SELECT RAISE(ABORT, 'generation stage attempts are immutable');
END;

CREATE TRIGGER IF NOT EXISTS evolution_generation_structured_results_immutable
BEFORE UPDATE ON evolution_generation_structured_results
BEGIN
    SELECT RAISE(ABORT, 'generation structured results are immutable');
END;

CREATE TRIGGER IF NOT EXISTS evolution_generated_drafts_immutable
BEFORE UPDATE ON evolution_generated_drafts
BEGIN
    SELECT RAISE(ABORT, 'generated drafts are immutable');
END;

CREATE INDEX IF NOT EXISTS idx_evolution_generation_jobs_queue
    ON evolution_generation_jobs(status, updated_at_ms, job_id);
CREATE INDEX IF NOT EXISTS idx_evolution_generation_jobs_workspace
    ON evolution_generation_jobs(workspace_id, created_at_ms DESC, job_id);
CREATE INDEX IF NOT EXISTS idx_evolution_generation_jobs_assessment
    ON evolution_generation_jobs(assessment_attempt_id, assessment_revision);
CREATE INDEX IF NOT EXISTS idx_evolution_generation_stage_attempts_job
    ON evolution_generation_stage_attempts(job_id, stage, attempt DESC);
CREATE INDEX IF NOT EXISTS idx_evolution_generation_model_calls_attempt
    ON evolution_generation_model_calls(stage_attempt_id, created_at_ms);
CREATE INDEX IF NOT EXISTS idx_evolution_generation_tool_receipts_attempt
    ON evolution_generation_tool_receipts(stage_attempt_id, created_at_ms);
CREATE INDEX IF NOT EXISTS idx_evolution_generation_quarantine_scope
    ON evolution_generated_skill_quarantine(scope, workspace_id, status, updated_at_ms DESC);
