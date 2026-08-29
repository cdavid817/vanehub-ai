CREATE TABLE IF NOT EXISTS evolution_orchestration_activity_outbox (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  outbox_id TEXT NOT NULL UNIQUE,
  source_domain TEXT NOT NULL CHECK (source_domain IN (
    'orchestration','automatic_application','probation','breaker','recovery'
  )),
  source_kind TEXT NOT NULL,
  source_id TEXT NOT NULL,
  source_revision TEXT NOT NULL,
  event_kind TEXT NOT NULL,
  committed_at_ms INTEGER NOT NULL,
  source_integrity_witness TEXT NOT NULL,
  UNIQUE (source_domain, source_kind, source_id, source_revision, event_kind)
);

CREATE TABLE IF NOT EXISTS evolution_evidence_activity_outbox (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  outbox_id TEXT NOT NULL UNIQUE,
  source_domain TEXT NOT NULL CHECK (source_domain IN ('evidence','retention')),
  source_kind TEXT NOT NULL,
  source_id TEXT NOT NULL,
  source_revision TEXT NOT NULL,
  event_kind TEXT NOT NULL,
  committed_at_ms INTEGER NOT NULL,
  source_integrity_witness TEXT NOT NULL,
  UNIQUE (source_domain, source_kind, source_id, source_revision, event_kind)
);

CREATE TABLE IF NOT EXISTS evolution_assessment_activity_outbox (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  outbox_id TEXT NOT NULL UNIQUE,
  source_domain TEXT NOT NULL CHECK (source_domain = 'assessment'),
  source_kind TEXT NOT NULL,
  source_id TEXT NOT NULL,
  source_revision TEXT NOT NULL,
  event_kind TEXT NOT NULL,
  committed_at_ms INTEGER NOT NULL,
  source_integrity_witness TEXT NOT NULL,
  UNIQUE (source_domain, source_kind, source_id, source_revision, event_kind)
);

CREATE TABLE IF NOT EXISTS evolution_generation_activity_outbox (
  sequence INTEGER PRIMARY KEY AUTOINCREMENT,
  outbox_id TEXT NOT NULL UNIQUE,
  source_domain TEXT NOT NULL CHECK (source_domain IN (
    'generation','skill_creation','recovery','retention'
  )),
  source_kind TEXT NOT NULL,
  source_id TEXT NOT NULL,
  source_revision TEXT NOT NULL,
  event_kind TEXT NOT NULL,
  committed_at_ms INTEGER NOT NULL,
  source_integrity_witness TEXT NOT NULL,
  UNIQUE (source_domain, source_kind, source_id, source_revision, event_kind)
);

CREATE TRIGGER IF NOT EXISTS evolution_orchestration_activity_outbox_immutable_update
BEFORE UPDATE ON evolution_orchestration_activity_outbox BEGIN
  SELECT RAISE(ABORT, 'orchestration activity outbox is immutable');
END;
CREATE TRIGGER IF NOT EXISTS evolution_orchestration_activity_outbox_immutable_delete
BEFORE DELETE ON evolution_orchestration_activity_outbox BEGIN
  SELECT RAISE(ABORT, 'orchestration activity outbox is immutable');
END;
CREATE TRIGGER IF NOT EXISTS evolution_evidence_activity_outbox_immutable_update
BEFORE UPDATE ON evolution_evidence_activity_outbox BEGIN
  SELECT RAISE(ABORT, 'evidence activity outbox is immutable');
END;
CREATE TRIGGER IF NOT EXISTS evolution_evidence_activity_outbox_immutable_delete
BEFORE DELETE ON evolution_evidence_activity_outbox BEGIN
  SELECT RAISE(ABORT, 'evidence activity outbox is immutable');
END;
CREATE TRIGGER IF NOT EXISTS evolution_assessment_activity_outbox_immutable_update
BEFORE UPDATE ON evolution_assessment_activity_outbox BEGIN
  SELECT RAISE(ABORT, 'assessment activity outbox is immutable');
END;
CREATE TRIGGER IF NOT EXISTS evolution_assessment_activity_outbox_immutable_delete
BEFORE DELETE ON evolution_assessment_activity_outbox BEGIN
  SELECT RAISE(ABORT, 'assessment activity outbox is immutable');
END;
CREATE TRIGGER IF NOT EXISTS evolution_generation_activity_outbox_immutable_update
BEFORE UPDATE ON evolution_generation_activity_outbox BEGIN
  SELECT RAISE(ABORT, 'generation activity outbox is immutable');
END;
CREATE TRIGGER IF NOT EXISTS evolution_generation_activity_outbox_immutable_delete
BEFORE DELETE ON evolution_generation_activity_outbox BEGIN
  SELECT RAISE(ABORT, 'generation activity outbox is immutable');
END;

CREATE TRIGGER IF NOT EXISTS evolution_runs_activity_insert
AFTER INSERT ON evolution_runs BEGIN
  INSERT INTO evolution_orchestration_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('run:' || NEW.run_id || ':' || NEW.revision,'orchestration','run',NEW.run_id,
          CAST(NEW.revision AS TEXT),NEW.status,NEW.updated_at_ms,NEW.policy_witness_hash);
END;
CREATE TRIGGER IF NOT EXISTS evolution_runs_activity_update
AFTER UPDATE OF status, revision ON evolution_runs
WHEN NEW.revision > OLD.revision BEGIN
  INSERT INTO evolution_orchestration_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('run:' || NEW.run_id || ':' || NEW.revision,'orchestration','run',NEW.run_id,
          CAST(NEW.revision AS TEXT),NEW.status,NEW.updated_at_ms,NEW.policy_witness_hash);
END;
CREATE TRIGGER IF NOT EXISTS evolution_run_stages_activity_insert
AFTER INSERT ON evolution_run_stages BEGIN
  INSERT INTO evolution_orchestration_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('stage:' || NEW.stage_id || ':' || NEW.revision,'orchestration','stage',NEW.stage_id,
          CAST(NEW.revision AS TEXT),NEW.status,COALESCE(NEW.completed_at_ms,NEW.started_at_ms,0),
          COALESCE(NEW.output_witness_hash,NEW.input_witness_hash));
END;
CREATE TRIGGER IF NOT EXISTS evolution_run_stages_activity_update
AFTER UPDATE OF status, revision ON evolution_run_stages
WHEN NEW.revision > OLD.revision BEGIN
  INSERT INTO evolution_orchestration_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('stage:' || NEW.stage_id || ':' || NEW.revision,'orchestration','stage',NEW.stage_id,
          CAST(NEW.revision AS TEXT),NEW.status,COALESCE(NEW.completed_at_ms,NEW.started_at_ms,0),
          COALESCE(NEW.output_witness_hash,NEW.input_witness_hash));
END;
CREATE TRIGGER IF NOT EXISTS evolution_auto_eligibility_activity_insert
AFTER INSERT ON evolution_auto_eligibility BEGIN
  INSERT INTO evolution_orchestration_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('eligibility:' || NEW.eligibility_id || ':' || NEW.revision,
          'automatic_application','eligibility',NEW.eligibility_id,CAST(NEW.revision AS TEXT),
          NEW.result,NEW.evaluated_at_ms,NEW.proof_hash);
END;
CREATE TRIGGER IF NOT EXISTS evolution_auto_applications_activity_insert
AFTER INSERT ON evolution_auto_applications BEGIN
  INSERT INTO evolution_orchestration_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('automatic-application:' || NEW.application_id,'automatic_application','application',
          NEW.application_id,'1','applied',NEW.committed_at_ms,NEW.preflight_witness_hash);
END;
CREATE TRIGGER IF NOT EXISTS evolution_auto_probations_activity_insert
AFTER INSERT ON evolution_auto_probations BEGIN
  INSERT INTO evolution_orchestration_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('probation:' || NEW.probation_id || ':' || NEW.revision,'probation','probation',
          NEW.probation_id,CAST(NEW.revision AS TEXT),NEW.status,NEW.starts_at_ms,
          NEW.baseline_witness_hash);
END;
CREATE TRIGGER IF NOT EXISTS evolution_auto_probations_activity_update
AFTER UPDATE OF status, revision ON evolution_auto_probations
WHEN NEW.revision > OLD.revision BEGIN
  INSERT INTO evolution_orchestration_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('probation:' || NEW.probation_id || ':' || NEW.revision,'probation','probation',
          NEW.probation_id,CAST(NEW.revision AS TEXT),NEW.status,NEW.ends_at_ms,
          NEW.baseline_witness_hash);
END;
CREATE TRIGGER IF NOT EXISTS evolution_auto_breakers_activity_insert
AFTER INSERT ON evolution_auto_breakers BEGIN
  INSERT INTO evolution_orchestration_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('breaker:' || NEW.breaker_id || ':' || NEW.revision,'breaker','breaker',NEW.breaker_id,
          CAST(NEW.revision AS TEXT),NEW.status,NEW.updated_at_ms,NEW.health_check_version);
END;
CREATE TRIGGER IF NOT EXISTS evolution_auto_breakers_activity_update
AFTER UPDATE OF status, revision ON evolution_auto_breakers
WHEN NEW.revision > OLD.revision BEGIN
  INSERT INTO evolution_orchestration_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('breaker:' || NEW.breaker_id || ':' || NEW.revision,'breaker','breaker',NEW.breaker_id,
          CAST(NEW.revision AS TEXT),NEW.status,NEW.updated_at_ms,NEW.health_check_version);
END;

CREATE TRIGGER IF NOT EXISTS evolution_signals_activity_insert
AFTER INSERT ON evolution_signals BEGIN
  INSERT INTO evolution_evidence_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('signal:' || NEW.signal_id || ':' || NEW.extractor_version,'evidence','signal',NEW.signal_id,
          CAST(NEW.extractor_version AS TEXT),'ingested',CAST(strftime('%s','now') AS INTEGER)*1000,
          NEW.deduplication_key);
END;
CREATE TRIGGER IF NOT EXISTS evolution_candidate_seeds_activity_insert
AFTER INSERT ON evolution_candidate_seeds BEGIN
  INSERT INTO evolution_evidence_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('seed:' || NEW.seed_id || ':' || NEW.signal_set_revision,'evidence','seed',NEW.seed_id,
          CAST(NEW.signal_set_revision AS TEXT),NEW.readiness,
          CAST(strftime('%s','now') AS INTEGER)*1000,NEW.grouping_key_hash);
END;
CREATE TRIGGER IF NOT EXISTS evolution_candidate_seeds_activity_update
AFTER UPDATE OF readiness, signal_set_revision ON evolution_candidate_seeds
WHEN NEW.signal_set_revision > OLD.signal_set_revision BEGIN
  INSERT INTO evolution_evidence_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('seed:' || NEW.seed_id || ':' || NEW.signal_set_revision,'evidence','seed',NEW.seed_id,
          CAST(NEW.signal_set_revision AS TEXT),NEW.readiness,
          CAST(strftime('%s','now') AS INTEGER)*1000,NEW.grouping_key_hash);
END;

CREATE TRIGGER IF NOT EXISTS evolution_assessment_attempts_activity_insert
AFTER INSERT ON evolution_assessment_attempts BEGIN
  INSERT INTO evolution_assessment_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('assessment:' || NEW.attempt_id || ':initial','assessment','attempt',NEW.attempt_id,
          'initial',NEW.status,COALESCE(NEW.completed_at_ms,NEW.created_at_ms),NEW.witness_hash);
END;
CREATE TRIGGER IF NOT EXISTS evolution_assessment_attempts_activity_update
AFTER UPDATE OF status ON evolution_assessment_attempts
WHEN NEW.status <> OLD.status BEGIN
  INSERT INTO evolution_assessment_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('assessment:' || NEW.attempt_id || ':' || NEW.status,'assessment','attempt',NEW.attempt_id,
          NEW.status,NEW.status,COALESCE(NEW.completed_at_ms,NEW.created_at_ms),NEW.witness_hash);
END;

CREATE TRIGGER IF NOT EXISTS evolution_generation_jobs_activity_insert
AFTER INSERT ON evolution_generation_jobs BEGIN
  INSERT INTO evolution_generation_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('generation-job:' || NEW.job_id || ':' || NEW.revision,'generation','job',NEW.job_id,
          CAST(NEW.revision AS TEXT),NEW.status,NEW.updated_at_ms,NEW.input_witness_hash);
END;
CREATE TRIGGER IF NOT EXISTS evolution_generation_jobs_activity_update
AFTER UPDATE OF status, revision ON evolution_generation_jobs
WHEN NEW.revision > OLD.revision BEGIN
  INSERT INTO evolution_generation_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('generation-job:' || NEW.job_id || ':' || NEW.revision,'generation','job',NEW.job_id,
          CAST(NEW.revision AS TEXT),NEW.status,NEW.updated_at_ms,NEW.input_witness_hash);
END;
CREATE TRIGGER IF NOT EXISTS evolution_evidence_dossiers_activity_insert
AFTER INSERT ON evolution_evidence_dossiers BEGIN
  INSERT INTO evolution_generation_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('dossier:' || NEW.dossier_id || ':' || NEW.revision,'generation','dossier',NEW.dossier_id,
          CAST(NEW.revision AS TEXT),'created',NEW.created_at_ms,NEW.content_hash);
END;
CREATE TRIGGER IF NOT EXISTS evolution_generated_skill_activity_insert
AFTER INSERT ON evolution_generated_skill_quarantine BEGIN
  INSERT INTO evolution_generation_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('skill-creation:' || NEW.proposal_id || ':' || NEW.revision,'skill_creation','proposal',
          NEW.proposal_id,CAST(NEW.revision AS TEXT),NEW.status,NEW.updated_at_ms,NEW.artifact_hash);
END;
CREATE TRIGGER IF NOT EXISTS evolution_generated_skill_activity_update
AFTER UPDATE OF status, revision ON evolution_generated_skill_quarantine
WHEN NEW.revision > OLD.revision BEGIN
  INSERT INTO evolution_generation_activity_outbox
    (outbox_id,source_domain,source_kind,source_id,source_revision,event_kind,
     committed_at_ms,source_integrity_witness)
  VALUES ('skill-creation:' || NEW.proposal_id || ':' || NEW.revision,'skill_creation','proposal',
          NEW.proposal_id,CAST(NEW.revision AS TEXT),NEW.status,NEW.updated_at_ms,NEW.artifact_hash);
END;
