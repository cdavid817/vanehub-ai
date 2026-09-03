CREATE TABLE IF NOT EXISTS evolution_trigger_receipts (
  receipt_id TEXT PRIMARY KEY,
  schema_version INTEGER NOT NULL CHECK (schema_version = 1),
  family TEXT NOT NULL CHECK (family IN (
    'startup_recovery','periodic_maintenance','application_idle_transition',
    'agent_run_completion','conversation_completion','explicit_feedback_commit',
    'verification_completion','delegated_utility_completion',
    'relevant_policy_or_skill_change','manual_run_request'
  )),
  workspace_id TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  source_id TEXT NOT NULL,
  source_revision INTEGER NOT NULL CHECK (source_revision >= 0),
  occurred_at_ms INTEGER NOT NULL,
  priority INTEGER NOT NULL CHECK (priority BETWEEN 0 AND 255),
  safe_reason_codes_json TEXT NOT NULL,
  actor TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  UNIQUE (family, source_kind, source_id, source_revision)
);
CREATE INDEX IF NOT EXISTS idx_evolution_trigger_workspace_time
  ON evolution_trigger_receipts(workspace_id, occurred_at_ms, receipt_id);

CREATE TABLE IF NOT EXISTS evolution_run_requests (
  request_id TEXT PRIMARY KEY,
  schema_version INTEGER NOT NULL CHECK (schema_version = 1),
  workspace_id TEXT NOT NULL,
  actor TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('pending','claimed','folded','completed')),
  trigger_counters_json TEXT NOT NULL,
  follow_up INTEGER NOT NULL CHECK (follow_up IN (0,1)),
  not_before_ms INTEGER NOT NULL,
  claimed_run_id TEXT,
  revision INTEGER NOT NULL CHECK (revision >= 0),
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_evolution_run_request_pending_workspace
  ON evolution_run_requests(workspace_id, follow_up)
  WHERE status IN ('pending','claimed');

CREATE TABLE IF NOT EXISTS evolution_run_request_trigger_links (
  request_id TEXT NOT NULL REFERENCES evolution_run_requests(request_id) ON DELETE CASCADE,
  receipt_id TEXT NOT NULL UNIQUE REFERENCES evolution_trigger_receipts(receipt_id),
  PRIMARY KEY (request_id, receipt_id)
);

CREATE TABLE IF NOT EXISTS evolution_runs (
  run_id TEXT PRIMARY KEY,
  schema_version INTEGER NOT NULL CHECK (schema_version = 1),
  request_id TEXT NOT NULL REFERENCES evolution_run_requests(request_id),
  workspace_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN (
    'requested','waiting_idle','running','partial','completed','failed',
    'cancel_requested','cancelled','recovered'
  )),
  current_stage TEXT,
  policy_witness_hash TEXT NOT NULL,
  budget_json TEXT NOT NULL,
  usage_json TEXT NOT NULL,
  cancel_requested_at_ms INTEGER,
  lease_owner TEXT,
  lease_expires_at_ms INTEGER,
  safe_failure_code TEXT,
  revision INTEGER NOT NULL CHECK (revision >= 0),
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_evolution_runs_active_workspace
  ON evolution_runs(workspace_id)
  WHERE status IN ('requested','waiting_idle','running','partial','cancel_requested');

CREATE TABLE IF NOT EXISTS evolution_run_trigger_links (
  run_id TEXT NOT NULL REFERENCES evolution_runs(run_id) ON DELETE CASCADE,
  receipt_id TEXT NOT NULL REFERENCES evolution_trigger_receipts(receipt_id),
  PRIMARY KEY (run_id, receipt_id)
);

CREATE TABLE IF NOT EXISTS evolution_run_stages (
  stage_id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES evolution_runs(run_id) ON DELETE CASCADE,
  stage TEXT NOT NULL CHECK (stage IN (
    'recover','maintain_evidence','build_seeds','assess','route_governance',
    'evaluate_auto_apply','project_results','notify'
  )),
  attempt INTEGER NOT NULL CHECK (attempt > 0),
  status TEXT NOT NULL,
  input_witness_hash TEXT NOT NULL,
  output_witness_hash TEXT,
  safe_failure_code TEXT,
  started_at_ms INTEGER,
  completed_at_ms INTEGER,
  revision INTEGER NOT NULL CHECK (revision >= 0),
  UNIQUE (run_id, stage, attempt)
);

CREATE TABLE IF NOT EXISTS evolution_run_items (
  item_id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES evolution_runs(run_id) ON DELETE CASCADE,
  stage TEXT NOT NULL,
  subsystem_idempotency_key TEXT NOT NULL UNIQUE,
  source_id TEXT NOT NULL,
  source_revision INTEGER NOT NULL CHECK (source_revision >= 0),
  committed_receipt_id TEXT,
  safe_failure_code TEXT,
  created_at_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_evolution_run_items_stage
  ON evolution_run_items(run_id, stage, source_id, source_revision);

CREATE TABLE IF NOT EXISTS evolution_run_checkpoints (
  checkpoint_id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES evolution_runs(run_id) ON DELETE CASCADE,
  stage TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN (
    'pending','committed','continuation_required','reconciled'
  )),
  cursor_record_id TEXT,
  cursor_record_revision INTEGER,
  usage_json TEXT NOT NULL,
  continuation_not_before_ms INTEGER,
  committed_at_ms INTEGER NOT NULL,
  UNIQUE (run_id, stage, checkpoint_id)
);

CREATE TABLE IF NOT EXISTS evolution_orchestration_policy (
  workspace_id TEXT PRIMARY KEY,
  schema_version INTEGER NOT NULL CHECK (schema_version = 1),
  mode TEXT NOT NULL CHECK (mode IN ('off','observe','enabled')),
  allowed_skill_ids_json TEXT NOT NULL,
  consent_json TEXT,
  automatic_budget_json TEXT NOT NULL,
  manual_budget_json TEXT NOT NULL,
  user_idle_ms INTEGER NOT NULL CHECK (user_idle_ms >= 0),
  maximum_idle_wait_ms INTEGER NOT NULL CHECK (maximum_idle_wait_ms > 0),
  notify_routine_completion INTEGER NOT NULL CHECK (notify_routine_completion IN (0,1)),
  revision INTEGER NOT NULL CHECK (revision >= 0),
  created_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_correction_authorizations (
  authorization_id TEXT PRIMARY KEY,
  feedback_id TEXT NOT NULL,
  feedback_revision INTEGER NOT NULL CHECK (feedback_revision >= 0),
  disclosure_version TEXT NOT NULL,
  authorized INTEGER NOT NULL CHECK (authorized IN (0,1)),
  actor TEXT NOT NULL,
  witness_hash TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  revoked_at_ms INTEGER,
  UNIQUE (feedback_id, feedback_revision)
);

CREATE TABLE IF NOT EXISTS evolution_deterministic_drafts (
  draft_id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL,
  target_skill_id TEXT NOT NULL,
  authorization_id TEXT NOT NULL REFERENCES evolution_correction_authorizations(authorization_id),
  assessment_id TEXT NOT NULL,
  producer_version TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  content_size_bytes INTEGER NOT NULL CHECK (content_size_bytes BETWEEN 0 AND 2048),
  provenance TEXT NOT NULL,
  source_witness_hash TEXT NOT NULL,
  created_at_ms INTEGER NOT NULL,
  UNIQUE (authorization_id, assessment_id, producer_version)
);

CREATE TABLE IF NOT EXISTS evolution_auto_eligibility (
  eligibility_id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES evolution_runs(run_id),
  draft_id TEXT NOT NULL REFERENCES evolution_deterministic_drafts(draft_id),
  target_skill_id TEXT NOT NULL,
  result TEXT NOT NULL CHECK (result IN (
    'ineligible','waiting','routed_to_curator','would_apply','eligible'
  )),
  predicates_json TEXT NOT NULL,
  proof_hash TEXT NOT NULL,
  overlay_preview_hash TEXT,
  evaluated_at_ms INTEGER NOT NULL,
  revision INTEGER NOT NULL CHECK (revision >= 0)
);

CREATE TABLE IF NOT EXISTS evolution_auto_rate_reservations (
  reservation_id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES evolution_runs(run_id),
  workspace_id TEXT NOT NULL,
  skill_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN (
    'reserved','committed','released','recovery_required'
  )),
  application_id TEXT,
  reserved_at_ms INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL,
  revision INTEGER NOT NULL CHECK (revision >= 0)
);
CREATE INDEX IF NOT EXISTS idx_evolution_rate_workspace_time
  ON evolution_auto_rate_reservations(workspace_id, reserved_at_ms, status);
CREATE INDEX IF NOT EXISTS idx_evolution_rate_skill_time
  ON evolution_auto_rate_reservations(skill_id, reserved_at_ms, status);

CREATE TABLE IF NOT EXISTS evolution_auto_preflight_witnesses (
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
  ON evolution_auto_preflight_witnesses(expires_at_ms)
  WHERE status = 'active';

CREATE TABLE IF NOT EXISTS evolution_auto_breakers (
  breaker_id TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL,
  skill_id TEXT,
  status TEXT NOT NULL CHECK (status IN (
    'closed','open','awaiting_health','awaiting_acknowledgement'
  )),
  safe_cause_code TEXT,
  source_run_id TEXT,
  source_application_id TEXT,
  health_check_version TEXT NOT NULL,
  health_probe_passed INTEGER NOT NULL CHECK (health_probe_passed IN (0,1)),
  acknowledged_actor TEXT,
  opened_at_ms INTEGER,
  updated_at_ms INTEGER NOT NULL,
  revision INTEGER NOT NULL CHECK (revision >= 0),
  UNIQUE (workspace_id, skill_id)
);

CREATE TABLE IF NOT EXISTS evolution_auto_applications (
  application_id TEXT PRIMARY KEY,
  run_id TEXT NOT NULL REFERENCES evolution_runs(run_id),
  eligibility_id TEXT NOT NULL UNIQUE REFERENCES evolution_auto_eligibility(eligibility_id),
  preflight_witness_hash TEXT NOT NULL UNIQUE,
  policy_witness_hash TEXT NOT NULL,
  rate_reservation_id TEXT NOT NULL UNIQUE REFERENCES evolution_auto_rate_reservations(reservation_id),
  curator_application_id TEXT NOT NULL UNIQUE,
  overlay_application_id TEXT NOT NULL UNIQUE,
  target_skill_id TEXT NOT NULL,
  prior_effective_hash TEXT NOT NULL,
  resulting_effective_hash TEXT NOT NULL,
  actor TEXT NOT NULL CHECK (actor = 'system_policy'),
  committed_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS evolution_auto_probations (
  probation_id TEXT PRIMARY KEY,
  application_id TEXT NOT NULL UNIQUE REFERENCES evolution_auto_applications(application_id),
  workspace_id TEXT NOT NULL,
  skill_id TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('active','healthy','regressed','expired','suspended')),
  prior_effective_hash TEXT NOT NULL,
  current_effective_hash TEXT NOT NULL,
  evidence_fingerprint TEXT NOT NULL,
  evidence_categories_json TEXT NOT NULL,
  baseline_witness_hash TEXT NOT NULL,
  starts_at_ms INTEGER NOT NULL,
  ends_at_ms INTEGER NOT NULL CHECK (ends_at_ms > starts_at_ms),
  revision INTEGER NOT NULL CHECK (revision >= 0)
);
CREATE INDEX IF NOT EXISTS idx_evolution_probation_active_skill
  ON evolution_auto_probations(workspace_id, skill_id, ends_at_ms)
  WHERE status = 'active';

CREATE TABLE IF NOT EXISTS evolution_probation_observations (
  observation_id TEXT PRIMARY KEY,
  probation_id TEXT NOT NULL REFERENCES evolution_auto_probations(probation_id) ON DELETE CASCADE,
  source_kind TEXT NOT NULL,
  source_id TEXT NOT NULL,
  source_revision INTEGER NOT NULL CHECK (source_revision >= 0),
  verified INTEGER NOT NULL CHECK (verified IN (0,1)),
  negative INTEGER NOT NULL CHECK (negative IN (0,1)),
  baseline_exceeded INTEGER NOT NULL CHECK (baseline_exceeded IN (0,1)),
  harmful_correction INTEGER NOT NULL CHECK (harmful_correction IN (0,1)),
  safe_category TEXT NOT NULL,
  witness_hash TEXT NOT NULL,
  observed_at_ms INTEGER NOT NULL,
  UNIQUE (probation_id, source_kind, source_id, source_revision)
);
