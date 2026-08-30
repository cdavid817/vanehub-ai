use super::{api::SkillEvolutionOrchestrationApi, api_queries::map_persistence};
use crate::contexts::skill_evolution_orchestration::{
    application::{AuthoritativeTriggerSourceV1, EvolutionTriggerReceiptOutcomeV1},
    domain::{EvolutionActorProvenance, EvolutionPolicyMode, EvolutionPolicyMutationV1},
    infrastructure::{OrchestrationRepository, SqliteCircuitBreakerRepository},
};
use rusqlite::OptionalExtension;
use serde_json::{json, Value};

impl SkillEvolutionOrchestrationApi {
    pub(crate) fn update_policy_projection(
        &self,
        workspace_id: &str,
        expected_revision: u64,
        mode: &str,
        allowed_skill_ids: Vec<String>,
        acknowledge_current_disclosure: bool,
        now_ms: i64,
    ) -> Result<Value, String> {
        let mode = match mode {
            "off" => EvolutionPolicyMode::Off,
            "observe" => EvolutionPolicyMode::Observe,
            "enabled" => EvolutionPolicyMode::Enabled,
            _ => return Err("invalid_input".into()),
        };
        OrchestrationRepository::new(self.database.clone())
            .update_policy(
                workspace_id,
                EvolutionPolicyMutationV1 {
                    expected_revision,
                    mode,
                    allowed_skill_ids,
                    acknowledge_current_disclosure,
                    notify_routine_completion: false,
                    updated_at_ms: now_ms,
                },
            )
            .map_err(map_persistence)?;
        self.policy_projection(workspace_id, now_ms)
    }

    pub(crate) fn request_manual_run(
        &self,
        workspace_id: &str,
        now_ms: i64,
    ) -> Result<Value, String> {
        let outcome = self
            .ingress
            .manual_run_request(
                AuthoritativeTriggerSourceV1 {
                    workspace_id: workspace_id.into(),
                    source_id: format!("manual-{now_ms}"),
                    source_revision: 1,
                    occurred_at_ms: now_ms,
                },
                now_ms,
            )
            .map_err(|_| "trigger_unavailable".to_string())?;
        match outcome {
            EvolutionTriggerReceiptOutcomeV1::Queued {
                request_id,
                created_request,
                ..
            } => Ok(json!({ "requestId": request_id, "queued": created_request })),
            EvolutionTriggerReceiptOutcomeV1::Duplicate { receipt_id } => {
                Ok(json!({ "requestId": receipt_id, "queued": false }))
            }
        }
    }

    pub(crate) fn cancel_run(
        &self,
        run_id: &str,
        expected_revision: u64,
        now_ms: i64,
    ) -> Result<Value, String> {
        let result = OrchestrationRepository::new(self.database.clone())
            .request_run_cancellation(run_id, expected_revision, now_ms)
            .map_err(map_persistence)?;
        Ok(
            json!({ "runId": result.run_id, "status": result.status.as_str(),
            "revision": result.revision }),
        )
    }

    pub(crate) fn acknowledge_breaker_projection(
        &self,
        breaker_id: &str,
        expected_revision: u64,
        now_ms: i64,
    ) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let scope = connection
            .query_row(
                "SELECT workspace_id,skill_id FROM evolution_auto_breakers WHERE breaker_id=?1",
                [breaker_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()
            .map_err(|_| storage())?
            .ok_or_else(|| "not_found".to_string())?;
        drop(connection);
        let breaker = SqliteCircuitBreakerRepository::new(self.database.clone())
            .acknowledge(
                &scope.0,
                scope.1.as_deref(),
                expected_revision,
                EvolutionActorProvenance::InteractiveUser,
                now_ms,
            )
            .map_err(|error| match error {
                crate::contexts::skill_evolution_orchestration::infrastructure::CircuitBreakerRepositoryError::InvalidInput => "invalid_input",
                crate::contexts::skill_evolution_orchestration::infrastructure::CircuitBreakerRepositoryError::NotFound => "not_found",
                crate::contexts::skill_evolution_orchestration::infrastructure::CircuitBreakerRepositoryError::Conflict => "health_and_acknowledgement_required",
                crate::contexts::skill_evolution_orchestration::infrastructure::CircuitBreakerRepositoryError::Storage => "storage_unavailable",
            }.to_string())?;
        Ok(
            json!({ "breakerId": breaker.breaker_id, "workspaceId": breaker.workspace_id,
            "skillId": breaker.skill_id, "status": breaker_status(breaker.status),
            "safeCauseCode": breaker.safe_cause_code,
            "healthCheckVersion": breaker.health_check_version,
            "healthProbePassed": breaker.health_probe_passed, "revision": breaker.revision,
            "updatedAtMs": breaker.updated_at_ms }),
        )
    }
}

fn breaker_status(
    value: crate::contexts::skill_evolution_orchestration::domain::CircuitBreakerStatus,
) -> &'static str {
    use crate::contexts::skill_evolution_orchestration::domain::CircuitBreakerStatus;
    match value {
        CircuitBreakerStatus::Closed => "closed",
        CircuitBreakerStatus::Open => "open",
        CircuitBreakerStatus::AwaitingHealth => "awaiting_health",
        CircuitBreakerStatus::AwaitingAcknowledgement => "awaiting_acknowledgement",
    }
}

fn storage() -> String {
    "storage_unavailable".into()
}
