use crate::contexts::skill_evolution_generation::{
    application::{GenerationToolError, GenerationToolReceiptPort},
    domain::{GenerationToolOutcome, GenerationToolReceiptV1},
};
use rusqlite::{params, Connection};

pub(crate) struct GenerationToolReceiptRepository<'connection> {
    connection: &'connection Connection,
}

impl<'connection> GenerationToolReceiptRepository<'connection> {
    pub(crate) fn new(connection: &'connection Connection) -> Self {
        Self { connection }
    }
}

impl GenerationToolReceiptPort for GenerationToolReceiptRepository<'_> {
    fn persist_receipt(
        &self,
        receipt: &GenerationToolReceiptV1,
    ) -> Result<(), GenerationToolError> {
        validate(receipt)?;
        let duration_ms =
            i64::try_from(receipt.duration_ms).map_err(|_| GenerationToolError::InvalidArgument)?;
        self.connection
            .execute(
                "INSERT INTO evolution_generation_tool_receipts
                 (receipt_id,stage_attempt_id,tool_name,argument_hash,source_witness_hash,
                  outcome,result_hash,safe_failure_code,duration_ms,created_at_ms)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    receipt.receipt_id,
                    receipt.stage_attempt_id,
                    receipt.tool_name,
                    receipt.argument_hash,
                    receipt.source_witness_hash,
                    outcome_name(receipt.outcome),
                    receipt.result_hash,
                    receipt.safe_failure_code,
                    duration_ms,
                    receipt.created_at_ms,
                ],
            )
            .map_err(|_| GenerationToolError::Failed)?;
        Ok(())
    }
}

fn validate(receipt: &GenerationToolReceiptV1) -> Result<(), GenerationToolError> {
    let known_tool = matches!(
        receipt.tool_name.as_str(),
        "read_dossier_section"
            | "read_skill_excerpt"
            | "find_exact_anchor"
            | "validate_draft_structure"
            | "simulate_local_preview"
    );
    let valid_result = match receipt.outcome {
        GenerationToolOutcome::Succeeded => {
            receipt.result_hash.is_some() && receipt.safe_failure_code.is_none()
        }
        _ => receipt.result_hash.is_none() && receipt.safe_failure_code.is_some(),
    };
    if !known_tool
        || !valid_result
        || receipt.receipt_id.trim().is_empty()
        || receipt.stage_attempt_id.trim().is_empty()
        || receipt.argument_hash.trim().is_empty()
        || receipt.source_witness_hash.trim().is_empty()
        || receipt.created_at_ms < 0
    {
        return Err(GenerationToolError::InvalidArgument);
    }
    Ok(())
}

fn outcome_name(outcome: GenerationToolOutcome) -> &'static str {
    match outcome {
        GenerationToolOutcome::Succeeded => "succeeded",
        GenerationToolOutcome::StaleWitness => "stale_witness",
        GenerationToolOutcome::InvalidArgument => "invalid_argument",
        GenerationToolOutcome::ResultTooLarge => "result_too_large",
        GenerationToolOutcome::BudgetExceeded => "budget_exceeded",
        GenerationToolOutcome::PolicyDenied => "policy_denied",
        GenerationToolOutcome::Failed => "failed",
    }
}
