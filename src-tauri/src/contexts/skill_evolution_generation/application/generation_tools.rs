use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::contexts::skill_evolution_generation::{
    application::{canonical_hash, canonical_json},
    domain::{GenerationToolOutcome, GenerationToolReceiptV1},
};

const MAX_TOOL_ARGUMENT_BYTES_V1: usize = 4 * 1024;
const MAX_TOOL_RESULT_BYTES_V1: usize = 16 * 1024;
pub(crate) const GENERATION_TOOL_LIMIT_V1: u16 = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationToolName {
    ReadDossierSection,
    ReadSkillExcerpt,
    FindExactAnchor,
    ValidateDraftStructure,
    SimulateLocalPreview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum GenerationToolArgumentsV1 {
    ReadDossierSection {
        dossier_id: String,
        ordinal: u8,
        cursor: Option<String>,
        limit: u16,
    },
    ReadSkillExcerpt {
        excerpt_id: String,
    },
    FindExactAnchor {
        query: String,
    },
    ValidateDraftStructure {
        response_json: String,
    },
    SimulateLocalPreview {
        structure_hash: String,
    },
}

pub(crate) struct GenerationToolRequestV1<'a> {
    pub(crate) receipt_id: &'a str,
    pub(crate) stage_attempt_id: &'a str,
    pub(crate) tool_name: GenerationToolName,
    pub(crate) arguments: GenerationToolArgumentsV1,
    pub(crate) frozen_input_witness_hash: &'a str,
    pub(crate) current_input_witness_hash: &'a str,
    pub(crate) calls_already_used: u16,
    pub(crate) duration_ms: u64,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenerationToolSafeResultV1 {
    pub(crate) safe_value: Value,
    pub(crate) citations: Vec<String>,
    pub(crate) source_witness_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationToolError {
    UnknownTool,
    InvalidArgument,
    StaleWitness,
    BudgetExceeded,
    ResultTooLarge,
    PolicyDenied,
    Failed,
}

pub(crate) trait GenerationToolBackendPort {
    fn execute(
        &self,
        name: GenerationToolName,
        arguments: &GenerationToolArgumentsV1,
    ) -> Result<GenerationToolSafeResultV1, GenerationToolError>;
}

pub(crate) trait GenerationToolReceiptPort {
    fn persist_receipt(&self, receipt: &GenerationToolReceiptV1)
        -> Result<(), GenerationToolError>;
}

pub(crate) fn parse_generation_tool_name(
    name: &str,
) -> Result<GenerationToolName, GenerationToolError> {
    match name {
        "read_dossier_section" => Ok(GenerationToolName::ReadDossierSection),
        "read_skill_excerpt" => Ok(GenerationToolName::ReadSkillExcerpt),
        "find_exact_anchor" => Ok(GenerationToolName::FindExactAnchor),
        "validate_draft_structure" => Ok(GenerationToolName::ValidateDraftStructure),
        "simulate_local_preview" => Ok(GenerationToolName::SimulateLocalPreview),
        _ => Err(GenerationToolError::UnknownTool),
    }
}

pub(crate) fn execute_generation_tool(
    backend: &dyn GenerationToolBackendPort,
    receipts: &dyn GenerationToolReceiptPort,
    request: &GenerationToolRequestV1<'_>,
) -> Result<GenerationToolSafeResultV1, GenerationToolError> {
    let argument_hash =
        canonical_hash(&request.arguments).map_err(|_| GenerationToolError::InvalidArgument)?;
    let execution = execute_validated(backend, request);
    let (outcome, result_hash, safe_failure_code) = match &execution {
        Ok(result) => (
            GenerationToolOutcome::Succeeded,
            Some(
                canonical_hash(&(
                    canonical_json(&result.safe_value).map_err(|_| GenerationToolError::Failed)?,
                    &result.citations,
                ))
                .map_err(|_| GenerationToolError::Failed)?,
            ),
            None,
        ),
        Err(error) => (
            tool_outcome(*error),
            None,
            Some(failure_code(*error).into()),
        ),
    };
    receipts.persist_receipt(&GenerationToolReceiptV1 {
        receipt_id: request.receipt_id.into(),
        stage_attempt_id: request.stage_attempt_id.into(),
        tool_name: tool_name(request.tool_name).into(),
        argument_hash,
        source_witness_hash: request.frozen_input_witness_hash.into(),
        outcome,
        result_hash,
        safe_failure_code,
        duration_ms: request.duration_ms,
        created_at_ms: request.created_at_ms,
    })?;
    execution
}

fn execute_validated(
    backend: &dyn GenerationToolBackendPort,
    request: &GenerationToolRequestV1<'_>,
) -> Result<GenerationToolSafeResultV1, GenerationToolError> {
    validate_request(request)?;
    let result = backend.execute(request.tool_name, &request.arguments)?;
    let result_json =
        canonical_json(&result.safe_value).map_err(|_| GenerationToolError::Failed)?;
    if result_json.len() > MAX_TOOL_RESULT_BYTES_V1 {
        return Err(GenerationToolError::ResultTooLarge);
    }
    if result.citations.is_empty()
        || result.source_witness_hash != request.frozen_input_witness_hash
    {
        return Err(GenerationToolError::StaleWitness);
    }
    Ok(result)
}

fn validate_request(request: &GenerationToolRequestV1<'_>) -> Result<(), GenerationToolError> {
    if request.receipt_id.trim().is_empty()
        || request.stage_attempt_id.trim().is_empty()
        || request.created_at_ms < 0
        || request.current_input_witness_hash != request.frozen_input_witness_hash
    {
        return Err(GenerationToolError::StaleWitness);
    }
    if request.calls_already_used >= GENERATION_TOOL_LIMIT_V1 {
        return Err(GenerationToolError::BudgetExceeded);
    }
    let arguments =
        canonical_json(&request.arguments).map_err(|_| GenerationToolError::InvalidArgument)?;
    if arguments.len() > MAX_TOOL_ARGUMENT_BYTES_V1
        || arguments.contains("../")
        || arguments.contains("\\..\\")
    {
        return Err(GenerationToolError::InvalidArgument);
    }
    if !arguments_match_name(request.tool_name, &request.arguments) {
        return Err(GenerationToolError::PolicyDenied);
    }
    Ok(())
}

fn arguments_match_name(name: GenerationToolName, arguments: &GenerationToolArgumentsV1) -> bool {
    matches!(
        (name, arguments),
        (
            GenerationToolName::ReadDossierSection,
            GenerationToolArgumentsV1::ReadDossierSection { .. }
        ) | (
            GenerationToolName::ReadSkillExcerpt,
            GenerationToolArgumentsV1::ReadSkillExcerpt { .. }
        ) | (
            GenerationToolName::FindExactAnchor,
            GenerationToolArgumentsV1::FindExactAnchor { .. }
        ) | (
            GenerationToolName::ValidateDraftStructure,
            GenerationToolArgumentsV1::ValidateDraftStructure { .. }
        ) | (
            GenerationToolName::SimulateLocalPreview,
            GenerationToolArgumentsV1::SimulateLocalPreview { .. }
        )
    )
}

pub(crate) fn tool_name(name: GenerationToolName) -> &'static str {
    match name {
        GenerationToolName::ReadDossierSection => "read_dossier_section",
        GenerationToolName::ReadSkillExcerpt => "read_skill_excerpt",
        GenerationToolName::FindExactAnchor => "find_exact_anchor",
        GenerationToolName::ValidateDraftStructure => "validate_draft_structure",
        GenerationToolName::SimulateLocalPreview => "simulate_local_preview",
    }
}

fn tool_outcome(error: GenerationToolError) -> GenerationToolOutcome {
    match error {
        GenerationToolError::UnknownTool | GenerationToolError::PolicyDenied => {
            GenerationToolOutcome::PolicyDenied
        }
        GenerationToolError::InvalidArgument => GenerationToolOutcome::InvalidArgument,
        GenerationToolError::StaleWitness => GenerationToolOutcome::StaleWitness,
        GenerationToolError::BudgetExceeded => GenerationToolOutcome::BudgetExceeded,
        GenerationToolError::ResultTooLarge => GenerationToolOutcome::ResultTooLarge,
        GenerationToolError::Failed => GenerationToolOutcome::Failed,
    }
}

fn failure_code(error: GenerationToolError) -> &'static str {
    match error {
        GenerationToolError::UnknownTool => "generation_tool_unknown",
        GenerationToolError::InvalidArgument => "generation_tool_invalid_argument",
        GenerationToolError::StaleWitness => "generation_tool_stale_witness",
        GenerationToolError::BudgetExceeded => "generation_tool_budget_exceeded",
        GenerationToolError::ResultTooLarge => "generation_tool_result_too_large",
        GenerationToolError::PolicyDenied => "generation_tool_policy_denied",
        GenerationToolError::Failed => "generation_tool_failed",
    }
}
