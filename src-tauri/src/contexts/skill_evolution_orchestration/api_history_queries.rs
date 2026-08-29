use super::{
    api::SkillEvolutionOrchestrationApi,
    api_queries::{page, parse_cursor, validate_query},
};
use rusqlite::params;
use serde_json::{json, Value};

impl SkillEvolutionOrchestrationApi {
    pub(crate) fn eligibility(
        &self,
        workspace_id: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Value, String> {
        validate_query(workspace_id, cursor, limit)?;
        let offset = parse_cursor(cursor)?;
        let connection = self.database.connection().map_err(|_| storage())?;
        let mut statement = connection
            .prepare(
                "SELECT e.eligibility_id,e.run_id,e.draft_id,e.target_skill_id,d.provenance,e.result,
             e.proof_hash,e.overlay_preview_hash,e.evaluated_at_ms,e.predicates_json,
             COALESCE((SELECT p.status FROM evolution_auto_preflight_witnesses p
               WHERE p.eligibility_id=e.eligibility_id ORDER BY p.issued_at_ms DESC LIMIT 1),'not_issued')
             FROM evolution_auto_eligibility e JOIN evolution_runs r ON r.run_id=e.run_id
             JOIN evolution_deterministic_drafts d ON d.draft_id=e.draft_id
             WHERE r.workspace_id=?1 ORDER BY e.evaluated_at_ms DESC,e.eligibility_id
             LIMIT ?2 OFFSET ?3",
            )
            .map_err(|_| storage())?;
        let items = statement.query_map(params![workspace_id,limit as i64,offset as i64], |row| {
            let predicates = row.get::<_,String>(9)?;
            let predicates: Value = serde_json::from_str(&predicates).map_err(|error|
                rusqlite::Error::FromSqlConversionFailure(
                    9,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                ))?;
            Ok(json!({ "eligibilityId": row.get::<_,String>(0)?, "runId": row.get::<_,String>(1)?,
                "draftId": row.get::<_,String>(2)?, "targetSkillId": row.get::<_,String>(3)?,
                "draftProvenance": row.get::<_,String>(4)?, "result": row.get::<_,String>(5)?,
                "proofHash": row.get::<_,String>(6)?, "overlayPreviewHash": row.get::<_,Option<String>>(7)?,
                "evaluatedAtMs": row.get::<_,i64>(8)?, "predicates": predicates,
                "preflightState": row.get::<_,String>(10)? }))
        }).map_err(|_| storage())?.collect::<Result<Vec<_>,_>>().map_err(|_| storage())?;
        page(items, offset, limit)
    }

    pub(crate) fn applications(
        &self,
        workspace_id: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Value, String> {
        validate_query(workspace_id, cursor, limit)?;
        let offset = parse_cursor(cursor)?;
        let connection = self.database.connection().map_err(|_| storage())?;
        let mut statement = connection
            .prepare(
                "SELECT a.application_id,a.run_id,a.eligibility_id,a.target_skill_id,
             a.curator_application_id,a.overlay_application_id,a.actor,a.committed_at_ms
             FROM evolution_auto_applications a JOIN evolution_runs r ON r.run_id=a.run_id
             WHERE r.workspace_id=?1 ORDER BY a.committed_at_ms DESC,a.application_id
             LIMIT ?2 OFFSET ?3",
            )
            .map_err(|_| storage())?;
        let items = statement
            .query_map(params![workspace_id, limit as i64, offset as i64], |row| {
                Ok(json!({
            "applicationId": row.get::<_,String>(0)?, "runId": row.get::<_,String>(1)?,
            "eligibilityId": row.get::<_,String>(2)?, "targetSkillId": row.get::<_,String>(3)?,
            "curatorApplicationId": row.get::<_,String>(4)?,
            "overlayApplicationId": row.get::<_,String>(5)?, "actor": row.get::<_,String>(6)?,
            "committedAtMs": row.get::<_,i64>(7)? }))
            })
            .map_err(|_| storage())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| storage())?;
        page(items, offset, limit)
    }

    pub(crate) fn probations(
        &self,
        workspace_id: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Value, String> {
        validate_query(workspace_id, cursor, limit)?;
        let offset = parse_cursor(cursor)?;
        let connection = self.database.connection().map_err(|_| storage())?;
        let mut statement = connection
            .prepare(
                "SELECT probation_id,application_id,workspace_id,skill_id,status,
             starts_at_ms,ends_at_ms,revision FROM evolution_auto_probations
             WHERE workspace_id=?1 ORDER BY starts_at_ms DESC,probation_id LIMIT ?2 OFFSET ?3",
            )
            .map_err(|_| storage())?;
        let items = statement
            .query_map(params![workspace_id, limit as i64, offset as i64], |row| {
                Ok(json!({
            "probationId": row.get::<_,String>(0)?, "applicationId": row.get::<_,String>(1)?,
            "workspaceId": row.get::<_,String>(2)?, "skillId": row.get::<_,String>(3)?,
            "status": row.get::<_,String>(4)?, "startsAtMs": row.get::<_,i64>(5)?,
            "endsAtMs": row.get::<_,i64>(6)?, "revision": row.get::<_,i64>(7)? }))
            })
            .map_err(|_| storage())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| storage())?;
        page(items, offset, limit)
    }

    pub(crate) fn breakers(
        &self,
        workspace_id: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Value, String> {
        validate_query(workspace_id, cursor, limit)?;
        let offset = parse_cursor(cursor)?;
        let connection = self.database.connection().map_err(|_| storage())?;
        let mut statement = connection
            .prepare(
                "SELECT breaker_id,workspace_id,skill_id,status,safe_cause_code,
             health_check_version,health_probe_passed,revision,updated_at_ms
             FROM evolution_auto_breakers WHERE workspace_id=?1
             ORDER BY updated_at_ms DESC,breaker_id LIMIT ?2 OFFSET ?3",
            )
            .map_err(|_| storage())?;
        let items = statement
            .query_map(params![workspace_id, limit as i64, offset as i64], |row| {
                Ok(json!({
            "breakerId": row.get::<_,String>(0)?, "workspaceId": row.get::<_,String>(1)?,
            "skillId": row.get::<_,Option<String>>(2)?, "status": row.get::<_,String>(3)?,
            "safeCauseCode": row.get::<_,Option<String>>(4)?,
            "healthCheckVersion": row.get::<_,String>(5)?,
            "healthProbePassed": row.get::<_,bool>(6)?, "revision": row.get::<_,i64>(7)?,
            "updatedAtMs": row.get::<_,i64>(8)? }))
            })
            .map_err(|_| storage())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| storage())?;
        page(items, offset, limit)
    }
}

fn storage() -> String {
    "storage_unavailable".into()
}
