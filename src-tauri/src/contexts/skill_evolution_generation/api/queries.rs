use super::{stable_id, GenerationApiError, SkillEvolutionGenerationApi};
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};

impl SkillEvolutionGenerationApi {
    pub(crate) fn jobs(
        &self,
        workspace_id: Option<&str>,
        skill_id: Option<&str>,
        status: Option<&str>,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<Value, GenerationApiError> {
        if limit == 0 || limit > 100 {
            return Err(GenerationApiError::InvalidRequest);
        }
        let offset = cursor
            .map(str::parse::<usize>)
            .transpose()
            .map_err(|_| GenerationApiError::InvalidRequest)?
            .unwrap_or(0);
        let connection = self
            .database
            .connection()
            .map_err(|_| GenerationApiError::Storage)?;
        let mut statement = connection.prepare(
            "SELECT j.job_id,j.request_id,j.workspace_id,j.seed_id,j.assessment_attempt_id,j.status,j.current_stage,j.usage_json,j.safe_failure_code,j.supersedes_job_id,j.created_at_ms,j.updated_at_ms,
             (SELECT artifact_kind FROM evolution_generated_drafts d WHERE d.job_id=j.job_id ORDER BY generation_attempt DESC LIMIT 1),
             (SELECT status FROM evolution_generation_handoffs h WHERE h.job_id=j.job_id ORDER BY updated_at_ms DESC LIMIT 1),j.input_witness_hash
             FROM evolution_generation_jobs j WHERE (?1 IS NULL OR j.workspace_id=?1) AND (?2 IS NULL OR j.status=?2)
             AND (?3 IS NULL OR EXISTS(SELECT 1 FROM evolution_generation_job_sources s WHERE s.job_id=j.job_id AND s.source_kind='effective_skill' AND s.source_id=?3))
             ORDER BY j.created_at_ms DESC,j.job_id DESC LIMIT ?4 OFFSET ?5"
        ).map_err(|_| GenerationApiError::Storage)?;
        let rows = statement
            .query_map(
                params![
                    workspace_id,
                    status,
                    skill_id,
                    (limit + 1) as i64,
                    offset as i64
                ],
                job_summary,
            )
            .map_err(|_| GenerationApiError::Storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| GenerationApiError::Storage)?;
        let has_more = rows.len() > limit;
        Ok(
            json!({"items":rows.into_iter().take(limit).collect::<Vec<_>>(),"nextCursor":has_more.then(|| (offset+limit).to_string())}),
        )
    }

    pub(crate) fn job_detail(&self, job_id: &str) -> Result<Option<Value>, GenerationApiError> {
        if job_id.trim().is_empty() {
            return Err(GenerationApiError::InvalidRequest);
        }
        let connection = self
            .database
            .connection()
            .map_err(|_| GenerationApiError::Storage)?;
        let mut summary = connection.query_row(
            "SELECT j.job_id,j.request_id,j.workspace_id,j.seed_id,j.assessment_attempt_id,j.status,j.current_stage,j.usage_json,j.safe_failure_code,j.supersedes_job_id,j.created_at_ms,j.updated_at_ms,
             (SELECT artifact_kind FROM evolution_generated_drafts d WHERE d.job_id=j.job_id ORDER BY generation_attempt DESC LIMIT 1),
             (SELECT status FROM evolution_generation_handoffs h WHERE h.job_id=j.job_id ORDER BY updated_at_ms DESC LIMIT 1),j.input_witness_hash
             FROM evolution_generation_jobs j WHERE j.job_id=?1", [job_id], job_summary)
            .optional().map_err(|_| GenerationApiError::Storage)?;
        let Some(ref mut detail) = summary else {
            return Ok(None);
        };
        let stages = json_rows(&connection,
            "SELECT json_object('attemptId',attempt_id,'stage',stage,'attempt',attempt,'status',status,'inputHash',input_hash,'outputHash',output_hash,'usage',json(usage_json),'safeFailureCode',safe_failure_code,'startedAt',started_at_ms,'completedAt',completed_at_ms) FROM evolution_generation_stage_attempts WHERE job_id=?1 ORDER BY started_at_ms,attempt_id", job_id)?;
        let dossier = connection.query_row("SELECT d.dossier_id,d.revision,d.content_hash FROM evolution_evidence_dossier_links l JOIN evolution_evidence_dossiers d ON d.dossier_id=l.dossier_id WHERE l.link_kind='job' AND l.linked_id=?1 ORDER BY d.revision DESC LIMIT 1", [job_id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,i64>(1)?,row.get::<_,String>(2)?))).optional().map_err(|_| GenerationApiError::Storage)?;
        let plan_json = connection.query_row("SELECT r.mutation_plan_json FROM evolution_generation_structured_results r JOIN evolution_generation_stage_attempts a ON a.attempt_id=r.stage_attempt_id WHERE a.job_id=?1 ORDER BY r.created_at_ms DESC LIMIT 1", [job_id], |row| row.get::<_,String>(0)).optional().map_err(|_| GenerationApiError::Storage)?;
        let citations = plan_json
            .and_then(|value| serde_json::from_str::<Value>(&value).ok())
            .and_then(|value| value.get("evidenceCitations").cloned())
            .unwrap_or_else(|| json!([]));
        let artifact = connection.query_row("SELECT draft_id,generation_attempt,artifact_kind,media_type,rendered_content,size_bytes,content_hash FROM evolution_generated_drafts WHERE job_id=?1 ORDER BY generation_attempt DESC LIMIT 1", [job_id], |row| Ok(json!({"draftId":row.get::<_,String>(0)?,"generationAttempt":row.get::<_,i64>(1)?,"artifactKind":row.get::<_,String>(2)?,"mediaType":row.get::<_,String>(3)?,"renderedContent":row.get::<_,String>(4)?,"sizeBytes":row.get::<_,i64>(5)?,"contentHash":row.get::<_,String>(6)?,"permanentlyManual":true,"citations":citations}))).optional().map_err(|_| GenerationApiError::Storage)?;
        let validation = connection.query_row("SELECT validation_id,status,checks_json,preview_witness_hash,report_hash,repair_attempt FROM evolution_generation_validations WHERE job_id=?1 ORDER BY created_at_ms DESC LIMIT 1", [job_id], |row| { let checks:String=row.get(2)?; Ok(json!({"validationId":row.get::<_,String>(0)?,"status":row.get::<_,String>(1)?,"checks":serde_json::from_str::<Value>(&checks).unwrap_or_else(|_|json!([])),"previewWitnessHash":row.get::<_,Option<String>>(3)?,"reportHash":row.get::<_,String>(4)?,"repairAttempt":row.get::<_,i64>(5)?})) }).optional().map_err(|_| GenerationApiError::Storage)?;
        let object = detail.as_object_mut().ok_or(GenerationApiError::Storage)?;
        object.insert("stages".into(), Value::Array(stages));
        object.insert("permanentlyManual".into(), Value::Bool(true));
        if let Some((id, revision, hash)) = dossier {
            object.insert("dossierId".into(), json!(id));
            object.insert("dossierRevision".into(), json!(revision));
            object.insert("dossierHash".into(), json!(hash));
        }
        if let Some(artifact) = artifact {
            object.insert("draftId".into(), artifact["draftId"].clone());
            object.insert("artifactHash".into(), artifact["contentHash"].clone());
            object.insert("draft".into(), artifact);
        }
        if let Some(validation) = validation {
            object.insert("validationId".into(), validation["validationId"].clone());
            object.insert(
                "previewWitnessHash".into(),
                validation["previewWitnessHash"].clone(),
            );
            object.insert("validation".into(), validation);
        }
        Ok(summary)
    }

    pub(crate) fn provenance(&self, job_id: &str) -> Result<Value, GenerationApiError> {
        let connection = self
            .database
            .connection()
            .map_err(|_| GenerationApiError::Storage)?;
        let exists = connection
            .query_row(
                "SELECT 1 FROM evolution_generation_jobs WHERE job_id=?1",
                [job_id],
                |_| Ok(()),
            )
            .optional()
            .map_err(|_| GenerationApiError::Storage)?
            .is_some();
        if !exists {
            return Err(GenerationApiError::NotFound);
        }
        let models=json_rows(&connection,"SELECT json_object('modelCallId',m.model_call_id,'stageAttemptId',m.stage_attempt_id,'purpose',m.purpose,'providerProtocol',m.provider_protocol,'providerProfileId',m.provider_profile_id,'modelId',m.model_id,'templateVersion',m.prompt_template_version,'responseSchemaVersion',m.response_schema_version,'outcome',m.outcome,'inputTokens',m.input_tokens,'outputTokens',m.output_tokens,'latencyMs',m.latency_ms,'structuredResponseHash',m.structured_response_hash,'safeFailureCode',m.safe_failure_code,'createdAtMs',m.created_at_ms) FROM evolution_generation_model_calls m JOIN evolution_generation_stage_attempts a ON a.attempt_id=m.stage_attempt_id WHERE a.job_id=?1 ORDER BY m.created_at_ms,m.model_call_id",job_id)?;
        let tools=json_rows(&connection,"SELECT json_object('receiptId',r.receipt_id,'stageAttemptId',r.stage_attempt_id,'toolName',r.tool_name,'argumentHash',r.argument_hash,'sourceWitnessHash',r.source_witness_hash,'outcome',r.outcome,'resultHash',r.result_hash,'safeFailureCode',r.safe_failure_code,'durationMs',r.duration_ms,'createdAtMs',r.created_at_ms) FROM evolution_generation_tool_receipts r JOIN evolution_generation_stage_attempts a ON a.attempt_id=r.stage_attempt_id WHERE a.job_id=?1 ORDER BY r.created_at_ms,r.receipt_id",job_id)?;
        let validations=json_rows(&connection,"SELECT json_object('validationId',validation_id,'status',status,'checks',json(checks_json),'previewWitnessHash',preview_witness_hash,'reportHash',report_hash,'repairAttempt',repair_attempt,'createdAtMs',created_at_ms) FROM evolution_generation_validations WHERE job_id=?1 ORDER BY created_at_ms,validation_id",job_id)?;
        Ok(
            json!({"jobId":job_id,"modelCalls":models,"toolReceipts":tools,"validations":validations}),
        )
    }

    pub(crate) fn quarantine(
        &self,
        workspace_id: Option<&str>,
        limit: usize,
        cursor: Option<&str>,
    ) -> Result<Value, GenerationApiError> {
        if limit == 0 || limit > 100 {
            return Err(GenerationApiError::InvalidRequest);
        }
        let offset = cursor
            .map(str::parse::<usize>)
            .transpose()
            .map_err(|_| GenerationApiError::InvalidRequest)?
            .unwrap_or(0);
        let connection = self
            .database
            .connection()
            .map_err(|_| GenerationApiError::Storage)?;
        let mut statement=connection.prepare("SELECT proposal_id,job_id,status,candidate_id,scope,workspace_id,artifact_hash,catalog_witness_hash,revision FROM evolution_generated_skill_quarantine WHERE (?1 IS NULL OR workspace_id=?1) ORDER BY created_at_ms DESC,proposal_id DESC LIMIT ?2 OFFSET ?3").map_err(|_| GenerationApiError::Storage)?;
        let rows=statement.query_map(params![workspace_id,(limit+1) as i64,offset as i64],|row| Ok(json!({"proposalId":row.get::<_,String>(0)?,"jobId":row.get::<_,String>(1)?,"status":row.get::<_,String>(2)?,"candidateId":row.get::<_,String>(3)?,"scope":row.get::<_,String>(4)?,"workspaceId":row.get::<_,Option<String>>(5)?,"artifactHash":row.get::<_,String>(6)?,"catalogWitnessHash":row.get::<_,String>(7)?,"revision":row.get::<_,i64>(8)?}))).map_err(|_| GenerationApiError::Storage)?.collect::<Result<Vec<_>,_>>().map_err(|_| GenerationApiError::Storage)?;
        let more = rows.len() > limit;
        Ok(
            json!({"items":rows.into_iter().take(limit).collect::<Vec<_>>(),"nextCursor":more.then(||(offset+limit).to_string())}),
        )
    }

    pub(crate) fn regenerate(
        &self,
        job_id: &str,
        expected_hash: &str,
        request_id: &str,
        now_ms: i64,
    ) -> Result<Value, GenerationApiError> {
        if [job_id, expected_hash, request_id]
            .iter()
            .any(|v| v.trim().is_empty())
        {
            return Err(GenerationApiError::InvalidRequest);
        }
        let mut connection = self
            .database
            .connection()
            .map_err(|_| GenerationApiError::Storage)?;
        let transaction = connection
            .transaction()
            .map_err(|_| GenerationApiError::Storage)?;
        let source=transaction.query_row("SELECT schema_version,workspace_id,seed_id,seed_revision,assessment_attempt_id,assessment_revision,input_witness_json,input_witness_hash,current_attempt,budget_json,status FROM evolution_generation_jobs WHERE job_id=?1",[job_id],|row| Ok((row.get::<_,i64>(0)?,row.get::<_,Option<String>>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,String>(5)?,row.get::<_,String>(6)?,row.get::<_,String>(7)?,row.get::<_,i64>(8)?,row.get::<_,String>(9)?,row.get::<_,String>(10)?))).optional().map_err(|_| GenerationApiError::Storage)?.ok_or(GenerationApiError::NotFound)?;
        if source.7 != expected_hash {
            return Err(GenerationApiError::Conflict);
        }
        if !matches!(source.10.as_str(), "completed" | "failed" | "cancelled") {
            return Err(GenerationApiError::Immutable);
        }
        let new_id = stable_id("generation", request_id);
        let usage="{\"elapsedMs\":0,\"modelCalls\":0,\"toolCalls\":0,\"inputTokens\":0,\"outputTokens\":0,\"validationRepairs\":0}";
        transaction.execute("INSERT INTO evolution_generation_jobs(job_id,schema_version,request_id,workspace_id,seed_id,seed_revision,assessment_attempt_id,assessment_revision,status,current_stage,input_witness_json,input_witness_hash,current_attempt,budget_json,usage_json,supersedes_job_id,revision,created_at_ms,updated_at_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'queued',NULL,?9,?10,?11,?12,?13,?14,1,?15,?15)",params![new_id,source.0,request_id,source.1,source.2,source.3,source.4,source.5,source.6,source.7,source.8+1,source.9,usage,job_id,now_ms]).map_err(|error| if error.sqlite_error_code()==Some(rusqlite::ErrorCode::ConstraintViolation){GenerationApiError::Conflict}else{GenerationApiError::Storage})?;
        transaction.execute("UPDATE evolution_generation_jobs SET status='superseded',superseded_by_job_id=?1,updated_at_ms=?2,revision=revision+1 WHERE job_id=?3 AND status IN ('completed','failed','cancelled')",params![new_id,now_ms,job_id]).map_err(|_|GenerationApiError::Storage)?;
        transaction
            .commit()
            .map_err(|_| GenerationApiError::Storage)?;
        self.job_detail(&new_id)?.ok_or(GenerationApiError::Storage)
    }
}

fn job_summary(row: &rusqlite::Row<'_>) -> rusqlite::Result<Value> {
    let usage: String = row.get(7)?;
    Ok(
        json!({"jobId":row.get::<_,String>(0)?,"requestId":row.get::<_,String>(1)?,"workspaceId":row.get::<_,Option<String>>(2)?,"seedId":row.get::<_,String>(3)?,"assessmentAttemptId":row.get::<_,String>(4)?,"status":row.get::<_,String>(5)?,"currentStage":row.get::<_,Option<String>>(6)?,"usage":serde_json::from_str::<Value>(&usage).unwrap_or_else(|_|json!({})),"safeFailureCode":row.get::<_,Option<String>>(8)?,"supersedesJobId":row.get::<_,Option<String>>(9)?,"createdAt":row.get::<_,i64>(10)?.to_string(),"updatedAt":row.get::<_,i64>(11)?.to_string(),"artifactKind":row.get::<_,Option<String>>(12)?,"handoffStatus":row.get::<_,Option<String>>(13)?,"inputWitnessHash":row.get::<_,String>(14)?}),
    )
}

fn json_rows(
    connection: &rusqlite::Connection,
    sql: &str,
    id: &str,
) -> Result<Vec<Value>, GenerationApiError> {
    let mut statement = connection
        .prepare(sql)
        .map_err(|_| GenerationApiError::Storage)?;
    let rows = statement
        .query_map([id], |row| row.get::<_, String>(0))
        .map_err(|_| GenerationApiError::Storage)?;
    rows.map(|row| {
        row.map_err(|_| GenerationApiError::Storage)
            .and_then(|value| serde_json::from_str(&value).map_err(|_| GenerationApiError::Storage))
    })
    .collect()
}
