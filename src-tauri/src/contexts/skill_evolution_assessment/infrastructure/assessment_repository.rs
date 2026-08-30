use crate::contexts::skill_evolution_assessment::domain::{
    AssessmentAttemptStatus, AssessmentConfidence, AssessmentOutput, AssessmentRisk,
    AssessmentRoute, AssessmentWitness, QualityCheckKind, QualityCheckResult, RoutingDecision,
    SelectionClassification,
};
use crate::platform::database::NativeDatabase;
use rusqlite::params;
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssessmentRepositoryError {
    InvalidInput,
    LeaseUnavailable,
    Storage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PersistAssessmentOutcome {
    Inserted { attempt_id: String },
    Coalesced { attempt_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssessmentModelCallRecord {
    pub(crate) model_call_id: String,
    pub(crate) stage: String,
    pub(crate) request_projection_hash: String,
    pub(crate) profile_id: Option<String>,
    pub(crate) provider_protocol: Option<String>,
    pub(crate) model_id: Option<String>,
    pub(crate) template_version: String,
    pub(crate) response_schema_version: String,
    pub(crate) outcome_category: String,
    pub(crate) sanitized_response_json: Option<String>,
    pub(crate) input_tokens: Option<u32>,
    pub(crate) output_tokens: Option<u32>,
    pub(crate) latency_ms: u64,
}

pub(crate) struct PersistCompletedAssessment<'a> {
    pub(crate) witness: &'a AssessmentWitness,
    pub(crate) output: &'a AssessmentOutput,
    pub(crate) routing: &'a RoutingDecision,
    pub(crate) model_calls: &'a [AssessmentModelCallRecord],
    pub(crate) model_evaluation_allowed: bool,
    pub(crate) created_at_ms: i64,
    pub(crate) completed_at_ms: i64,
}

#[derive(Clone)]
pub(crate) struct SqliteAssessmentRepository {
    database: NativeDatabase,
}

impl SqliteAssessmentRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn persist_completed(
        &self,
        record: &PersistCompletedAssessment<'_>,
    ) -> Result<PersistAssessmentOutcome, AssessmentRepositoryError> {
        validate_record(record)?;
        let witness_hash = record.witness.canonical_hash();
        let explanation = normalized_explanation(record)?;
        let connection = self
            .database
            .connection()
            .map_err(|_| AssessmentRepositoryError::Storage)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| AssessmentRepositoryError::Storage)?;
        let inserted = transaction.execute(
            "INSERT INTO evolution_assessment_attempts (attempt_id, seed_id, seed_revision, \
                witness_hash, status, classification, route, confidence, risk, seed_fingerprint, \
                lineage_hash, target_universe_hash, sanitizer_version, selector_policy_version, \
                lexical_policy_version, gate_policy_version, routing_policy_version, \
                confidence_policy_version, evaluator_config_hash, consent_version, \
                model_evaluation_allowed, is_current, winning_rule, normalized_explanation_json, \
                created_at_ms, completed_at_ms) \
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,1,?22,?23,?24,?25)",
            params![
                record.output.attempt_id,
                record.witness.input.seed_id,
                record.witness.input.seed_revision,
                witness_hash,
                attempt_status(record.output.status),
                classification(record.output.classification),
                route(record.output.route),
                confidence(record.output.confidence),
                risk(record.output.risk),
                record.witness.input.seed_fingerprint,
                record.witness.input.lineage_hash,
                target_universe_hash(record.witness)?,
                record.witness.input.sanitizer_version,
                record.witness.selector_policy_version,
                record.witness.lexical_policy_version,
                record.witness.gate_policy_version,
                record.witness.routing_policy_version,
                record.witness.confidence_policy_version,
                record.witness.evaluator_configuration,
                record.witness.consent_version,
                i64::from(record.model_evaluation_allowed),
                record.routing.winning_rule,
                explanation,
                record.created_at_ms,
                record.completed_at_ms,
            ],
        );
        match inserted {
            Ok(_) => {}
            Err(error)
                if error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) =>
            {
                let attempt_id = transaction
                    .query_row(
                        "SELECT attempt_id FROM evolution_assessment_attempts \
                         WHERE seed_id = ?1 AND seed_revision = ?2 AND witness_hash = ?3",
                        params![
                            record.witness.input.seed_id,
                            record.witness.input.seed_revision,
                            witness_hash,
                        ],
                        |row| row.get(0),
                    )
                    .map_err(|_| AssessmentRepositoryError::Storage)?;
                return Ok(PersistAssessmentOutcome::Coalesced { attempt_id });
            }
            Err(_) => return Err(AssessmentRepositoryError::Storage),
        }
        persist_targets(&transaction, record)?;
        persist_checks(&transaction, record)?;
        persist_evidence_links(&transaction, record)?;
        persist_model_calls(&transaction, record)?;
        transaction
            .commit()
            .map_err(|_| AssessmentRepositoryError::Storage)?;
        Ok(PersistAssessmentOutcome::Inserted {
            attempt_id: record.output.attempt_id.clone(),
        })
    }

    pub(crate) fn complete_leased(
        &self,
        record: &PersistCompletedAssessment<'_>,
        lease_owner: &str,
    ) -> Result<PersistAssessmentOutcome, AssessmentRepositoryError> {
        validate_record(record)?;
        if lease_owner.trim().is_empty() || lease_owner.len() > 128 {
            return Err(AssessmentRepositoryError::InvalidInput);
        }
        let witness_hash = record.witness.canonical_hash();
        let explanation = normalized_explanation(record)?;
        let connection = self
            .database
            .connection()
            .map_err(|_| AssessmentRepositoryError::Storage)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| AssessmentRepositoryError::Storage)?;
        let updated = transaction
            .execute(
                "UPDATE evolution_assessment_attempts SET status='completed', classification=?1, \
                 route=?2, confidence=?3, risk=?4, winning_rule=?5, \
                 normalized_explanation_json=?6, model_evaluation_allowed=?7, \
                 lease_owner=NULL, lease_expires_at_ms=NULL, heartbeat_at_ms=NULL, \
                 completed_at_ms=?8 WHERE attempt_id=?9 AND witness_hash=?10 AND status='running' \
                 AND lease_owner=?11 AND lease_expires_at_ms >= ?8",
                params![
                    classification(record.output.classification),
                    route(record.output.route),
                    confidence(record.output.confidence),
                    risk(record.output.risk),
                    record.routing.winning_rule,
                    explanation,
                    i64::from(record.model_evaluation_allowed),
                    record.completed_at_ms,
                    record.output.attempt_id,
                    witness_hash,
                    lease_owner,
                ],
            )
            .map_err(|_| AssessmentRepositoryError::Storage)?;
        if updated == 0 {
            let existing = transaction.query_row(
                "SELECT status, witness_hash FROM evolution_assessment_attempts WHERE attempt_id=?1",
                [&record.output.attempt_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            );
            return match existing {
                Ok((status, existing_hash))
                    if status == "completed" && existing_hash == witness_hash =>
                {
                    Ok(PersistAssessmentOutcome::Coalesced {
                        attempt_id: record.output.attempt_id.clone(),
                    })
                }
                _ => Err(AssessmentRepositoryError::LeaseUnavailable),
            };
        }
        persist_targets(&transaction, record)?;
        persist_checks(&transaction, record)?;
        persist_evidence_links(&transaction, record)?;
        persist_model_calls(&transaction, record)?;
        transaction
            .commit()
            .map_err(|_| AssessmentRepositoryError::Storage)?;
        Ok(PersistAssessmentOutcome::Inserted {
            attempt_id: record.output.attempt_id.clone(),
        })
    }
}

fn normalized_explanation(
    record: &PersistCompletedAssessment<'_>,
) -> Result<String, AssessmentRepositoryError> {
    serde_json::to_string(&json!({
        "selectionThreshold": record.output.selection_threshold,
        "attributionUncertain": record.output.attribution_uncertain,
        "lessonShape": record.output.lesson_shape,
        "evaluator": record.output.evaluator,
        "routingRules": record.routing.rules,
        "routeConstraints": record.routing.route_constraints,
    }))
    .map_err(|_| AssessmentRepositoryError::InvalidInput)
}

fn validate_record(
    record: &PersistCompletedAssessment<'_>,
) -> Result<(), AssessmentRepositoryError> {
    if record.output.status != AssessmentAttemptStatus::Completed
        || record.output.checks.len() != 9
        || record.output.attempt_id.trim().is_empty()
        || record.witness.input.seed_id.trim().is_empty()
        || record.created_at_ms < 0
        || record.completed_at_ms < record.created_at_ms
        || record.model_calls.len() > 2
    {
        return Err(AssessmentRepositoryError::InvalidInput);
    }
    Ok(())
}

fn persist_targets(
    transaction: &rusqlite::Transaction<'_>,
    record: &PersistCompletedAssessment<'_>,
) -> Result<(), AssessmentRepositoryError> {
    for (ordinal, target) in record.output.targets.iter().enumerate() {
        let matched = serde_json::to_string(&target.matched_feature_classes)
            .map_err(|_| AssessmentRepositoryError::InvalidInput)?;
        let exclusions = serde_json::to_string(&target.exclusions)
            .map_err(|_| AssessmentRepositoryError::InvalidInput)?;
        transaction
            .execute(
                "INSERT INTO evolution_assessment_targets (attempt_id,ordinal,skill_id,skill_type, \
                 revision_hash,scope,lifecycle,trust,score,attribution_uncertain, \
                 matched_feature_classes_json,exclusions_json) \
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                params![record.output.attempt_id, ordinal as i64, target.witness.skill_id, target.witness.skill_type, target.witness.revision_hash, serde_value(target.witness.scope)?, serde_value(target.witness.lifecycle)?, serde_value(target.witness.trust)?, target.score, i64::from(target.attribution_uncertain), matched, exclusions],
            )
            .map_err(|_| AssessmentRepositoryError::Storage)?;
        for (component, score) in [
            ("attribution", target.attribution_score),
            ("participation", target.participation_score),
            ("compatibility", target.compatibility_score),
            ("lexical", target.lexical_score),
            ("locality", target.locality_score),
        ] {
            transaction
                .execute(
                    "INSERT INTO evolution_assessment_score_components VALUES (?1,?2,?3,?4)",
                    params![record.output.attempt_id, ordinal as i64, component, score],
                )
                .map_err(|_| AssessmentRepositoryError::Storage)?;
        }
    }
    Ok(())
}

fn persist_checks(
    transaction: &rusqlite::Transaction<'_>,
    record: &PersistCompletedAssessment<'_>,
) -> Result<(), AssessmentRepositoryError> {
    for (ordinal, check) in record.output.checks.iter().enumerate() {
        transaction
            .execute(
                "INSERT INTO evolution_assessment_checks VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    record.output.attempt_id,
                    ordinal as i64,
                    check_kind(check.kind),
                    check_result(check.result),
                    risk(check.severity),
                    check.reason_code,
                    serde_json::to_string(&check.evidence_ids)
                        .map_err(|_| AssessmentRepositoryError::InvalidInput)?,
                    serde_json::to_string(&check.route_constraints)
                        .map_err(|_| AssessmentRepositoryError::InvalidInput)?
                ],
            )
            .map_err(|_| AssessmentRepositoryError::Storage)?;
    }
    Ok(())
}

fn persist_evidence_links(
    transaction: &rusqlite::Transaction<'_>,
    record: &PersistCompletedAssessment<'_>,
) -> Result<(), AssessmentRepositoryError> {
    let mut ids = record.witness.input.evidence_ids.clone();
    ids.sort();
    ids.dedup();
    for evidence_id in ids {
        transaction
            .execute(
                "INSERT INTO evolution_assessment_evidence_links VALUES (?1,?2,'supporting')",
                params![record.output.attempt_id, evidence_id],
            )
            .map_err(|_| AssessmentRepositoryError::Storage)?;
    }
    Ok(())
}

fn persist_model_calls(
    transaction: &rusqlite::Transaction<'_>,
    record: &PersistCompletedAssessment<'_>,
) -> Result<(), AssessmentRepositoryError> {
    for call in record.model_calls {
        if call
            .sanitized_response_json
            .as_ref()
            .is_some_and(|value| value.len() > 8_192)
        {
            return Err(AssessmentRepositoryError::InvalidInput);
        }
        let latency_ms =
            i64::try_from(call.latency_ms).map_err(|_| AssessmentRepositoryError::InvalidInput)?;
        transaction.execute(
            "INSERT INTO evolution_assessment_model_calls VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![call.model_call_id, record.output.attempt_id, call.stage, call.request_projection_hash, call.profile_id, call.provider_protocol, call.model_id, call.template_version, call.response_schema_version, call.outcome_category, call.sanitized_response_json, call.input_tokens, call.output_tokens, latency_ms, record.completed_at_ms],
        ).map_err(|_| AssessmentRepositoryError::Storage)?;
    }
    Ok(())
}

pub(super) fn target_universe_hash(
    witness: &AssessmentWitness,
) -> Result<String, AssessmentRepositoryError> {
    use sha2::{Digest, Sha256};
    let mut targets = witness.targets.clone();
    targets.sort_by(|left, right| {
        left.skill_id
            .cmp(&right.skill_id)
            .then(left.revision_hash.cmp(&right.revision_hash))
    });
    let bytes =
        serde_json::to_vec(&targets).map_err(|_| AssessmentRepositoryError::InvalidInput)?;
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn serde_value<T: serde::Serialize>(value: T) -> Result<String, AssessmentRepositoryError> {
    serde_json::to_value(value)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .ok_or(AssessmentRepositoryError::InvalidInput)
}

fn attempt_status(value: AssessmentAttemptStatus) -> &'static str {
    match value {
        AssessmentAttemptStatus::Pending => "pending",
        AssessmentAttemptStatus::Running => "running",
        AssessmentAttemptStatus::Completed => "completed",
        AssessmentAttemptStatus::Failed => "failed",
        AssessmentAttemptStatus::Superseded => "superseded",
    }
}
fn classification(value: SelectionClassification) -> &'static str {
    match value {
        SelectionClassification::Selected => "selected",
        SelectionClassification::Ambiguous => "ambiguous",
        SelectionClassification::NoTarget => "no_target",
    }
}
fn route(value: AssessmentRoute) -> &'static str {
    match value {
        AssessmentRoute::Advance => "advance",
        AssessmentRoute::Drop => "drop",
        AssessmentRoute::RecordMemoryOnly => "record_memory_only",
        AssessmentRoute::MergeDuplicate => "merge_duplicate",
        AssessmentRoute::NeedsHumanReview => "needs_human_review",
    }
}
fn confidence(value: AssessmentConfidence) -> &'static str {
    match value {
        AssessmentConfidence::Low => "low",
        AssessmentConfidence::Medium => "medium",
        AssessmentConfidence::High => "high",
    }
}
fn risk(value: AssessmentRisk) -> &'static str {
    match value {
        AssessmentRisk::Low => "low",
        AssessmentRisk::Medium => "medium",
        AssessmentRisk::High => "high",
    }
}
fn check_result(value: QualityCheckResult) -> &'static str {
    match value {
        QualityCheckResult::Pass => "pass",
        QualityCheckResult::Fail => "fail",
        QualityCheckResult::Review => "review",
        QualityCheckResult::NotApplicable => "not_applicable",
    }
}
fn check_kind(value: QualityCheckKind) -> &'static str {
    match value {
        QualityCheckKind::PrivacyResidue => "privacy_residue",
        QualityCheckKind::EvidenceSufficiency => "evidence_sufficiency",
        QualityCheckKind::DuplicateKnowledge => "duplicate_knowledge",
        QualityCheckKind::TransientIncident => "transient_incident",
        QualityCheckKind::GuidanceSpecificity => "guidance_specificity",
        QualityCheckKind::EvidenceConsistency => "evidence_consistency",
        QualityCheckKind::TargetCompatibility => "target_compatibility",
        QualityCheckKind::ExecutableContentRisk => "executable_content_risk",
        QualityCheckKind::TargetLifecycleMutability => "target_lifecycle_mutability",
    }
}
