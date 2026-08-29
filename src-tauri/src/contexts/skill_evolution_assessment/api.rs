//! Published native facade for sanitized assessment queries and scheduling.

mod draft_review;
pub(crate) use draft_review::*;

use crate::contexts::skill_evolution_assessment::application::{
    AssessmentQueueLane, AssessmentQueueRequest, QueueEnqueueOutcome,
};
use crate::contexts::skill_evolution_assessment::domain::ModelEvaluationConsent;
use crate::contexts::skill_evolution_assessment::infrastructure::{
    SqliteAssessmentPolicyRepository, SqliteAssessmentQueueRepository,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AssessmentApiError {
    InvalidRequest,
    NotFound,
    Storage,
}

impl AssessmentApiError {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::InvalidRequest => "assessment-invalid-request",
            Self::NotFound => "assessment-not-found",
            Self::Storage => "assessment-storage-failed",
        }
    }
}

#[derive(Clone)]
pub(crate) struct SkillEvolutionAssessmentApi {
    database: NativeDatabase,
    policy: SqliteAssessmentPolicyRepository,
    queue: SqliteAssessmentQueueRepository,
}

impl SkillEvolutionAssessmentApi {
    pub(crate) fn new(database: NativeDatabase) -> Result<Self, AssessmentApiError> {
        let queue = SqliteAssessmentQueueRepository::new(database.clone(), 256)
            .map_err(|_| AssessmentApiError::Storage)?;
        Ok(Self {
            policy: SqliteAssessmentPolicyRepository::new(database.clone()),
            database,
            queue,
        })
    }

    pub(crate) fn query(
        &self,
        workspace: Option<&str>,
        skill_id: Option<&str>,
        seed_id: Option<&str>,
        include_history: bool,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<Value, AssessmentApiError> {
        if limit == 0 || limit > 100 {
            return Err(AssessmentApiError::InvalidRequest);
        }
        let offset = cursor
            .map(|value| value.parse::<usize>())
            .transpose()
            .map_err(|_| AssessmentApiError::InvalidRequest)?
            .unwrap_or(0);
        let connection = self
            .database
            .connection()
            .map_err(|_| AssessmentApiError::Storage)?;
        let mut statement = connection
            .prepare(
                "SELECT a.attempt_id,a.seed_id,a.seed_revision,a.status,a.classification,a.route, \
             a.confidence,a.risk,a.is_current,a.winning_rule,a.created_at_ms,a.completed_at_ms, \
             s.replacement_attempt_id,s.reason_code,s.changed_witness_hash FROM evolution_assessment_attempts a \
             JOIN evolution_candidate_seeds seed ON seed.seed_id=a.seed_id \
             LEFT JOIN evolution_assessment_supersessions s ON s.prior_attempt_id=a.attempt_id \
             WHERE (?1 IS NULL OR seed.workspace=?1) AND (?2 IS NULL OR a.seed_id=?2) \
             AND (?3=1 OR a.is_current=1) AND (?4 IS NULL OR EXISTS(SELECT 1 FROM \
             evolution_assessment_targets t WHERE t.attempt_id=a.attempt_id AND t.skill_id=?4)) \
             ORDER BY a.created_at_ms DESC,a.attempt_id DESC LIMIT ?5 OFFSET ?6",
            )
            .map_err(|_| AssessmentApiError::Storage)?;
        let rows = statement
            .query_map(
                params![
                    workspace,
                    seed_id,
                    i64::from(include_history),
                    skill_id,
                    (limit + 1) as i64,
                    offset as i64
                ],
                summary_json,
            )
            .map_err(|_| AssessmentApiError::Storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| AssessmentApiError::Storage)?;
        let has_more = rows.len() > limit;
        let items = rows.into_iter().take(limit).collect::<Vec<_>>();
        Ok(json!({
            "items": items,
            "nextCursor": has_more.then(|| (offset + limit).to_string()),
        }))
    }

    pub(crate) fn detail(&self, attempt_id: &str) -> Result<Option<Value>, AssessmentApiError> {
        if attempt_id.trim().is_empty() {
            return Err(AssessmentApiError::InvalidRequest);
        }
        let connection = self
            .database
            .connection()
            .map_err(|_| AssessmentApiError::Storage)?;
        let summary = connection.query_row(
            "SELECT a.attempt_id,a.seed_id,a.seed_revision,a.status,a.classification,a.route, \
             a.confidence,a.risk,a.is_current,a.winning_rule,a.created_at_ms,a.completed_at_ms, \
             s.replacement_attempt_id,s.reason_code,s.changed_witness_hash FROM evolution_assessment_attempts a LEFT JOIN \
             evolution_assessment_supersessions s ON s.prior_attempt_id=a.attempt_id WHERE a.attempt_id=?1",
            [attempt_id], summary_json,
        ).optional().map_err(|_| AssessmentApiError::Storage)?;
        let Some(mut summary) = summary else {
            return Ok(None);
        };
        let targets = query_json_rows(&connection,
            "SELECT json_object('ordinal',ordinal,'skillId',skill_id,'skillType',skill_type,'revisionHash',revision_hash, \
             'scope',scope,'lifecycle',lifecycle,'trust',trust,'score',score, \
             'attribution',CASE WHEN attribution_uncertain=0 THEN 'verified' WHEN COALESCE((SELECT score FROM \
             evolution_assessment_score_components ac WHERE ac.attempt_id=evolution_assessment_targets.attempt_id \
             AND ac.target_ordinal=ordinal AND ac.component='attribution'),0)>=20 THEN 'correlated' WHEN COALESCE((SELECT \
             score FROM evolution_assessment_score_components ac WHERE ac.attempt_id=evolution_assessment_targets.attempt_id \
             AND ac.target_ordinal=ordinal AND ac.component='attribution'),0)>0 THEN 'weak' ELSE 'unattributed' END, \
             'attributionUncertain',json(attribution_uncertain),'matchedFeatureClasses',json(matched_feature_classes_json), \
             'exclusions',json(exclusions_json),'components',json(COALESCE((SELECT json_group_array(json_object( \
             'component',component,'score',score)) FROM evolution_assessment_score_components c WHERE \
             c.attempt_id=evolution_assessment_targets.attempt_id AND c.target_ordinal=ordinal),'[]'))) \
             FROM evolution_assessment_targets \
             WHERE attempt_id=?1 ORDER BY ordinal", attempt_id)?;
        let checks = query_json_rows(&connection,
            "SELECT json_object('ordinal',ordinal,'kind',kind,'result',result,'severity',severity, \
             'reasonCode',reason_code,'evidenceIds',json(evidence_ids_json), \
             'routeConstraints',json(route_constraints_json)) FROM evolution_assessment_checks \
             WHERE attempt_id=?1 ORDER BY ordinal", attempt_id)?;
        let model = connection.query_row(
            "SELECT provider_protocol,model_id,template_version,response_schema_version,outcome_category \
             FROM evolution_assessment_model_calls WHERE attempt_id=?1 ORDER BY stage DESC LIMIT 1",
            [attempt_id], |row| Ok((row.get::<_, Option<String>>(0)?,row.get::<_, Option<String>>(1)?,row.get::<_, String>(2)?,row.get::<_, String>(3)?,row.get::<_, String>(4)?)),
        ).optional().map_err(|_| AssessmentApiError::Storage)?;
        let object = summary.as_object_mut().ok_or(AssessmentApiError::Storage)?;
        let explanation = connection.query_row(
            "SELECT normalized_explanation_json FROM evolution_assessment_attempts WHERE attempt_id=?1",
            [attempt_id],
            |row| row.get::<_, Option<String>>(0),
        ).map_err(|_| AssessmentApiError::Storage)?
            .and_then(|value| serde_json::from_str::<Value>(&value).ok());
        let version_witnesses = connection.query_row(
            "SELECT json_object('witnessHash',witness_hash,'lineageHash',lineage_hash, \
             'targetUniverseHash',target_universe_hash,'sanitizerVersion',sanitizer_version, \
             'selectorPolicyVersion',selector_policy_version,'gatePolicyVersion',gate_policy_version, \
             'routingPolicyVersion',routing_policy_version,'confidencePolicyVersion',confidence_policy_version, \
             'consentVersion',consent_version) FROM evolution_assessment_attempts WHERE attempt_id=?1",
            [attempt_id],
            |row| row.get::<_, String>(0),
        ).map_err(|_| AssessmentApiError::Storage)?;
        object.insert("targets".to_string(), Value::Array(targets));
        object.insert("checks".to_string(), Value::Array(checks));
        object.insert(
            "routeConstraints".to_string(),
            explanation
                .as_ref()
                .and_then(|value| value.get("routeConstraints"))
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        if let Some(threshold) = explanation
            .as_ref()
            .and_then(|value| value.get("selectionThreshold"))
        {
            object.insert("selectionThreshold".to_string(), threshold.clone());
        }
        object.insert(
            "versionWitnesses".to_string(),
            serde_json::from_str(&version_witnesses).map_err(|_| AssessmentApiError::Storage)?,
        );
        object.insert("provenance".to_string(), match model {
            Some((protocol, model_id, template, schema, outcome)) => json!({"deterministic":true,"modelEvaluationAllowed":true,"modelConsulted":outcome=="valid","fallbackReason":(outcome!="valid").then_some(outcome),"providerProtocol":protocol,"modelId":model_id,"templateVersion":template,"responseSchemaVersion":schema}),
            None => json!({"deterministic":true,"modelEvaluationAllowed":false,"modelConsulted":false}),
        });
        Ok(Some(summary))
    }

    pub(crate) fn policy(&self) -> Result<Value, AssessmentApiError> {
        let value = self
            .policy
            .load()
            .map_err(|_| AssessmentApiError::Storage)?;
        Ok(policy_json(&value))
    }

    pub(crate) fn update_consent(
        &self,
        consent: ModelEvaluationConsent,
    ) -> Result<Value, AssessmentApiError> {
        self.policy.update(&consent).map(|value| policy_json(&value)).map_err(|error| match error {
            crate::contexts::skill_evolution_assessment::infrastructure::AssessmentPolicyError::InvalidInput => AssessmentApiError::InvalidRequest,
            crate::contexts::skill_evolution_assessment::infrastructure::AssessmentPolicyError::Storage => AssessmentApiError::Storage,
        })
    }

    pub(crate) fn schedule(
        &self,
        seed_id: &str,
        expected_hash: Option<&str>,
        now_ms: i64,
    ) -> Result<Value, AssessmentApiError> {
        let connection = self
            .database
            .connection()
            .map_err(|_| AssessmentApiError::Storage)?;
        let hash = match expected_hash {
            Some(value) if !value.trim().is_empty() => value.to_string(),
            Some(_) => return Err(AssessmentApiError::InvalidRequest),
            None => connection.query_row("SELECT witness_hash FROM evolution_assessment_attempts WHERE seed_id=?1 AND is_current=1", [seed_id], |row| row.get(0)).optional().map_err(|_| AssessmentApiError::Storage)?.ok_or(AssessmentApiError::NotFound)?,
        };
        let digest = Sha256::digest(format!("{seed_id}:{hash}").as_bytes());
        let queue_id = format!(
            "assessment-{}",
            digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        );
        let outcome = self
            .queue
            .enqueue(&AssessmentQueueRequest {
                queue_id: queue_id.clone(),
                seed_id: seed_id.to_string(),
                witness_hash: hash,
                lane: AssessmentQueueLane::Deterministic,
                priority: 100,
                available_at_ms: now_ms,
                created_at_ms: now_ms,
            })
            .map_err(|_| AssessmentApiError::Storage)?;
        let status = match outcome {
            QueueEnqueueOutcome::Scheduled { .. } => "scheduled",
            QueueEnqueueOutcome::Coalesced { .. } => "coalesced",
            QueueEnqueueOutcome::OptionalFallback => "disabled",
            QueueEnqueueOutcome::Saturated => "saturated",
        };
        Ok(json!({"queueId":queue_id,"status":status}))
    }
}

fn summary_json(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    Ok(
        json!({"attemptId":row.get::<_,String>(0)?,"seedId":row.get::<_,String>(1)?,"seedRevision":row.get::<_,String>(2)?,"status":row.get::<_,String>(3)?,"classification":row.get::<_,Option<String>>(4)?,"route":row.get::<_,Option<String>>(5)?,"confidence":row.get::<_,Option<String>>(6)?,"risk":row.get::<_,Option<String>>(7)?,"isCurrent":row.get::<_,i64>(8)?!=0,"winningRule":row.get::<_,Option<String>>(9)?,"createdAtMs":row.get::<_,i64>(10)?,"completedAtMs":row.get::<_,Option<i64>>(11)?,"supersededByAttemptId":row.get::<_,Option<String>>(12)?,"supersessionReason":row.get::<_,Option<String>>(13)?,"changedWitnessHash":row.get::<_,Option<String>>(14)?}),
    )
}

fn query_json_rows(
    connection: &rusqlite::Connection,
    sql: &str,
    id: &str,
) -> Result<Vec<Value>, AssessmentApiError> {
    let mut statement = connection
        .prepare(sql)
        .map_err(|_| AssessmentApiError::Storage)?;
    let rows = statement
        .query_map([id], |row| row.get::<_, String>(0))
        .map_err(|_| AssessmentApiError::Storage)?
        .map(|row| {
            row.map_err(|_| AssessmentApiError::Storage)
                .and_then(|value| {
                    serde_json::from_str(&value).map_err(|_| AssessmentApiError::Storage)
                })
        })
        .collect();
    rows
}

fn policy_json(value: &ModelEvaluationConsent) -> Value {
    json!({"evaluatorPolicyVersion":value.policy_version,"disclosureVersion":value.disclosure_version,"modelEvaluationEnabled":value.enabled,"providerAvailable":true,"changedAtMs":value.changed_at_ms})
}
