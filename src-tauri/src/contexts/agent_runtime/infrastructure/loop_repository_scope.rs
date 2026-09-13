//! SQLite persistence for run scope bindings, frozen assessments, audit receipts, idempotent
//! control operations and sealed acceptance. Every write that changes authority runs in one
//! immediate transaction with the run transition it belongs to, and every state-changing update
//! bumps the run revision so acceptance can compare-and-set against exactly what it validated.

use super::loop_repository::StoredDefinition;
use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, LoopAuditConsumption, LoopAuditReceipt, LoopControlAction,
    LoopControlOperationClaim, LoopExecutionAssessment, LoopRunScopeRecord, LoopScopeBinding,
};
use crate::contexts::agent_runtime::domain::{
    LoopDefinition, LoopRun, LoopRunStatus, LoopTerminalReason,
};
use crate::platform::database::{NativeDatabase, SqliteWriteTransaction};
use rusqlite::{params, OptionalExtension, Row, Transaction};

pub(super) fn create_run_with_scope(
    database: &NativeDatabase,
    run: &LoopRun,
    definition_snapshot: &LoopDefinition,
    project_path: &str,
    created_at: &str,
    assessment: &LoopExecutionAssessment,
    audit: Option<&LoopAuditConsumption>,
) -> Result<(), AgentRuntimeApplicationError> {
    let snapshot = serde_json::to_string(&StoredDefinition::from_domain(definition_snapshot))
        .map_err(loop_error)?;
    let assessment = serde_json::to_string(assessment).map_err(loop_error)?;
    let mut connection = database.connection().map_err(loop_error)?;
    let transaction = connection.write_transaction().map_err(loop_error)?;
    if let Some(audit) = audit {
        consume_receipt(&transaction, audit)?;
    }
    transaction
        .execute(
            r#"INSERT INTO loop_runs (
                id, definition_id, definition_snapshot, status, phase, terminal_reason,
                current_iteration, consecutive_runtime_errors, consecutive_no_progress,
                pause_requested, project_path, simulated, created_at, updated_at,
                revision, execution_assessment
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 0, ?12, ?12, 1, ?13)"#,
            params![
                run.id(),
                run.definition_id(),
                snapshot,
                run.status().as_str(),
                run.phase().as_str(),
                run.terminal_reason().map(LoopTerminalReason::as_str),
                i64::from(run.current_iteration()),
                i64::from(run.consecutive_runtime_errors()),
                i64::from(run.consecutive_no_progress()),
                i64::from(run.pause_requested()),
                project_path,
                created_at,
                assessment,
            ],
        )
        .map_err(loop_error)?;
    transaction.commit().map_err(loop_error)
}

/// One receipt authorizes one committed transition. Every bound field is part of the predicate,
/// so a receipt for another action, target, revision, scope, assessment or process never matches.
fn consume_receipt(
    transaction: &Transaction<'_>,
    audit: &LoopAuditConsumption,
) -> Result<(), AgentRuntimeApplicationError> {
    let changed = transaction
        .execute(
            r#"UPDATE loop_audit_receipts SET consumed_at = ?2
               WHERE id = ?1 AND acknowledged_at IS NOT NULL AND consumed_at IS NULL
                 AND action = ?3 AND target_id = ?4 AND expected_revision = ?5
                 AND scope_digest = ?6 AND assessment_digest = ?7 AND app_epoch = ?8
                 AND expires_at > ?2 AND created_at <= ?2"#,
            params![
                audit.receipt_id,
                audit.now,
                audit.action.as_str(),
                audit.target_id,
                to_i64(audit.expected_revision)?,
                audit.scope_digest,
                audit.assessment_digest,
                audit.app_epoch,
            ],
        )
        .map_err(loop_error)?;
    if changed == 1 {
        Ok(())
    } else {
        Err(AgentRuntimeApplicationError::Validation(
            "The audit acknowledgement was already used, expired or no longer matches this action."
                .to_string(),
        ))
    }
}

pub(super) fn find_run_scope(
    database: &NativeDatabase,
    run_id: &str,
) -> Result<Option<LoopRunScopeRecord>, AgentRuntimeApplicationError> {
    database
        .connection()
        .map_err(loop_error)?
        .query_row(
            r#"SELECT revision, scope_binding, execution_assessment, sealed_evidence_id,
                acceptance_operation_id FROM loop_runs WHERE id = ?1"#,
            [run_id],
            read_scope_record,
        )
        .optional()
        .map_err(loop_error)
}

pub(super) fn read_scope_record(row: &Row<'_>) -> rusqlite::Result<LoopRunScopeRecord> {
    Ok(LoopRunScopeRecord {
        revision: u64::try_from(row.get::<_, i64>(0)?).unwrap_or(1),
        binding: row
            .get::<_, Option<String>>(1)?
            .map(|value| serde_json::from_str::<LoopScopeBinding>(&value))
            .transpose()
            .map_err(to_sql_error)?,
        assessment: row
            .get::<_, Option<String>>(2)?
            .map(|value| serde_json::from_str::<LoopExecutionAssessment>(&value))
            .transpose()
            .map_err(to_sql_error)?,
        sealed_evidence_id: row.get(3)?,
        acceptance_operation_id: row.get(4)?,
    })
}

pub(super) fn attach_run_scope_binding(
    database: &NativeDatabase,
    run_id: &str,
    binding: &LoopScopeBinding,
    expected_status: LoopRunStatus,
) -> Result<(), AgentRuntimeApplicationError> {
    let encoded = serde_json::to_string(binding).map_err(loop_error)?;
    let changed = database
        .connection()
        .map_err(loop_error)?
        .execute(
            r#"UPDATE loop_runs SET scope_binding = ?2, revision = revision + 1
               WHERE id = ?1 AND status = ?3 AND scope_binding IS NULL"#,
            params![run_id, encoded, expected_status.as_str()],
        )
        .map_err(loop_error)?;
    require_changed(
        changed,
        "Loop scope binding is already attached or the run state changed",
    )
}

pub(super) fn save_run_transition_with_audit(
    database: &NativeDatabase,
    run: &LoopRun,
    expected_status: LoopRunStatus,
    updated_at: &str,
    completed_at: Option<&str>,
    audit: Option<&LoopAuditConsumption>,
) -> Result<(), AgentRuntimeApplicationError> {
    let mut connection = database.connection().map_err(loop_error)?;
    let transaction = connection.write_transaction().map_err(loop_error)?;
    if let Some(audit) = audit {
        consume_receipt(&transaction, audit)?;
    }
    let changed = transaction
        .execute(
            r#"UPDATE loop_runs SET status = ?2, phase = ?3, terminal_reason = ?4,
                current_iteration = ?5, consecutive_runtime_errors = ?6,
                consecutive_no_progress = ?7, pause_requested = ?8, updated_at = ?9,
                completed_at = ?10, revision = revision + 1
               WHERE id = ?1 AND status = ?11"#,
            params![
                run.id(),
                run.status().as_str(),
                run.phase().as_str(),
                run.terminal_reason().map(LoopTerminalReason::as_str),
                i64::from(run.current_iteration()),
                i64::from(run.consecutive_runtime_errors()),
                i64::from(run.consecutive_no_progress()),
                i64::from(run.pause_requested()),
                updated_at,
                completed_at,
                expected_status.as_str(),
            ],
        )
        .map_err(loop_error)?;
    require_changed(
        changed,
        "Loop run state changed before this action completed",
    )?;
    transaction.commit().map_err(loop_error)
}

pub(super) fn save_continue_transition_with_audit(
    database: &NativeDatabase,
    run: &LoopRun,
    expected_status: LoopRunStatus,
    feedback: &str,
    updated_at: &str,
    audit: Option<&LoopAuditConsumption>,
) -> Result<(), AgentRuntimeApplicationError> {
    let previous_sequence = run.current_iteration().checked_sub(1).ok_or_else(|| {
        AgentRuntimeApplicationError::Loop("Loop iteration sequence is invalid.".to_string())
    })?;
    let mut connection = database.connection().map_err(loop_error)?;
    let transaction = connection.write_transaction().map_err(loop_error)?;
    if let Some(audit) = audit {
        consume_receipt(&transaction, audit)?;
    }
    let feedback_changed = transaction
        .execute(
            r#"UPDATE loop_iterations SET user_feedback = ?3
               WHERE run_id = ?1 AND sequence = ?2 AND user_feedback IS NULL"#,
            params![run.id(), i64::from(previous_sequence), feedback],
        )
        .map_err(loop_error)?;
    require_changed(
        feedback_changed,
        "Loop continuation feedback is already saved or the iteration changed",
    )?;
    let run_changed = transaction
        .execute(
            r#"UPDATE loop_runs SET status = ?2, phase = ?3, terminal_reason = ?4,
                current_iteration = ?5, consecutive_runtime_errors = ?6,
                consecutive_no_progress = ?7, pause_requested = ?8, updated_at = ?9,
                completed_at = NULL, acceptance_operation_id = NULL, revision = revision + 1
               WHERE id = ?1 AND status = ?10 AND acceptance_operation_id IS NULL"#,
            params![
                run.id(),
                run.status().as_str(),
                run.phase().as_str(),
                run.terminal_reason().map(LoopTerminalReason::as_str),
                i64::from(run.current_iteration()),
                i64::from(run.consecutive_runtime_errors()),
                i64::from(run.consecutive_no_progress()),
                i64::from(run.pause_requested()),
                updated_at,
                expected_status.as_str(),
            ],
        )
        .map_err(loop_error)?;
    require_changed(
        run_changed,
        "Loop run state changed or an acceptance is in progress",
    )?;
    transaction.commit().map_err(loop_error)
}

pub(super) fn attach_acceptance_operation(
    database: &NativeDatabase,
    run_id: &str,
    operation_id: &str,
    expected_revision: u64,
) -> Result<(), AgentRuntimeApplicationError> {
    let changed = database
        .connection()
        .map_err(loop_error)?
        .execute(
            r#"UPDATE loop_runs SET acceptance_operation_id = ?2
               WHERE id = ?1 AND revision = ?3 AND status = 'awaiting-acceptance'
                 AND acceptance_operation_id IS NULL AND sealed_evidence_id IS NULL"#,
            params![run_id, operation_id, to_i64(expected_revision)?],
        )
        .map_err(loop_error)?;
    require_changed(
        changed,
        "Another acceptance or transition already owns this run",
    )
}

pub(super) fn release_acceptance_operation(
    database: &NativeDatabase,
    run_id: &str,
    operation_id: &str,
) -> Result<(), AgentRuntimeApplicationError> {
    database
        .connection()
        .map_err(loop_error)?
        .execute(
            "UPDATE loop_runs SET acceptance_operation_id = NULL WHERE id = ?1 AND acceptance_operation_id = ?2",
            params![run_id, operation_id],
        )
        .map_err(loop_error)?;
    Ok(())
}

pub(super) fn seal_acceptance(
    database: &NativeDatabase,
    run: &LoopRun,
    expected_revision: u64,
    operation_id: &str,
    evidence_id: &str,
    completed_at: &str,
) -> Result<(), AgentRuntimeApplicationError> {
    let changed = database
        .connection()
        .map_err(loop_error)?
        .execute(
            r#"UPDATE loop_runs SET status = ?2, phase = ?3, terminal_reason = ?4,
                sealed_evidence_id = ?5, acceptance_operation_id = NULL, updated_at = ?6,
                completed_at = ?6, pause_requested = 0, revision = revision + 1
               WHERE id = ?1 AND revision = ?7 AND status = 'awaiting-acceptance'
                 AND acceptance_operation_id = ?8 AND sealed_evidence_id IS NULL"#,
            params![
                run.id(),
                run.status().as_str(),
                run.phase().as_str(),
                run.terminal_reason().map(LoopTerminalReason::as_str),
                evidence_id,
                completed_at,
                to_i64(expected_revision)?,
                operation_id,
            ],
        )
        .map_err(loop_error)?;
    require_changed(
        changed,
        "The run changed while its evidence was being sealed; acceptance was not committed",
    )
}

pub(super) fn create_audit_challenge(
    database: &NativeDatabase,
    receipt: &LoopAuditReceipt,
) -> Result<(), AgentRuntimeApplicationError> {
    database
        .connection()
        .map_err(loop_error)?
        .execute(
            r#"INSERT INTO loop_audit_receipts (
                id, challenge_id, action, target_id, expected_revision, scope_digest,
                assessment_digest, client_context, app_epoch, created_at, expires_at,
                acknowledged_at, consumed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, NULL, NULL)"#,
            params![
                receipt.id,
                receipt.challenge_id,
                receipt.action.as_str(),
                receipt.target_id,
                to_i64(receipt.expected_revision)?,
                receipt.scope_digest,
                receipt.assessment_digest,
                receipt.client_context,
                receipt.app_epoch,
                receipt.created_at,
                receipt.expires_at,
            ],
        )
        .map_err(loop_error)?;
    Ok(())
}

pub(super) fn acknowledge_audit_challenge(
    database: &NativeDatabase,
    challenge_id: &str,
    acknowledged_at: &str,
) -> Result<LoopAuditReceipt, AgentRuntimeApplicationError> {
    let mut connection = database.connection().map_err(loop_error)?;
    let transaction = connection.write_transaction().map_err(loop_error)?;
    let changed = transaction
        .execute(
            r#"UPDATE loop_audit_receipts SET acknowledged_at = ?2
               WHERE challenge_id = ?1 AND acknowledged_at IS NULL AND consumed_at IS NULL
                 AND expires_at > ?2"#,
            params![challenge_id, acknowledged_at],
        )
        .map_err(loop_error)?;
    if changed != 1 {
        return Err(AgentRuntimeApplicationError::Validation(
            "The audit challenge is unknown, already acknowledged or expired.".to_string(),
        ));
    }
    let receipt = transaction
        .query_row(
            &format!("{RECEIPT_SELECT} WHERE challenge_id = ?1"),
            [challenge_id],
            read_receipt,
        )
        .map_err(loop_error)?;
    transaction.commit().map_err(loop_error)?;
    Ok(receipt)
}

pub(super) fn find_audit_receipt(
    database: &NativeDatabase,
    receipt_id: &str,
) -> Result<Option<LoopAuditReceipt>, AgentRuntimeApplicationError> {
    database
        .connection()
        .map_err(loop_error)?
        .query_row(
            &format!("{RECEIPT_SELECT} WHERE id = ?1"),
            [receipt_id],
            read_receipt,
        )
        .optional()
        .map_err(loop_error)
}

const RECEIPT_SELECT: &str = r#"SELECT id, challenge_id, action, target_id, expected_revision,
    scope_digest, assessment_digest, client_context, app_epoch, created_at, expires_at,
    acknowledged_at, consumed_at FROM loop_audit_receipts"#;

fn read_receipt(row: &Row<'_>) -> rusqlite::Result<LoopAuditReceipt> {
    let action: String = row.get(2)?;
    Ok(LoopAuditReceipt {
        id: row.get(0)?,
        challenge_id: row.get(1)?,
        action: LoopControlAction::parse(&action).ok_or_else(|| {
            to_sql_error(std::io::Error::other(format!(
                "unknown audit action {action}"
            )))
        })?,
        target_id: row.get(3)?,
        expected_revision: u64::try_from(row.get::<_, i64>(4)?).unwrap_or(0),
        scope_digest: row.get(5)?,
        assessment_digest: row.get(6)?,
        client_context: row.get(7)?,
        app_epoch: row.get(8)?,
        created_at: row.get(9)?,
        expires_at: row.get(10)?,
        acknowledged_at: row.get(11)?,
        consumed_at: row.get(12)?,
    })
}

pub(super) fn claim_control_operation(
    database: &NativeDatabase,
    idempotency_key: &str,
    action: &str,
    target_id: &str,
    created_at: &str,
) -> Result<LoopControlOperationClaim, AgentRuntimeApplicationError> {
    let mut connection = database.connection().map_err(loop_error)?;
    let transaction = connection.write_transaction().map_err(loop_error)?;
    let inserted = transaction
        .execute(
            r#"INSERT OR IGNORE INTO loop_control_operations (idempotency_key, action, target_id, outcome, created_at)
               VALUES (?1, ?2, ?3, '', ?4)"#,
            params![idempotency_key, action, target_id, created_at],
        )
        .map_err(loop_error)?;
    let claim = if inserted == 1 {
        LoopControlOperationClaim::New
    } else {
        let (stored_action, stored_target, outcome): (String, String, String) = transaction
            .query_row(
                "SELECT action, target_id, outcome FROM loop_control_operations WHERE idempotency_key = ?1",
                [idempotency_key],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(loop_error)?;
        if stored_action != action || stored_target != target_id {
            return Err(AgentRuntimeApplicationError::Validation(
                "The idempotency key was already used for a different control request.".to_string(),
            ));
        }
        if outcome.is_empty() {
            LoopControlOperationClaim::InFlight
        } else {
            LoopControlOperationClaim::Existing(outcome)
        }
    };
    transaction.commit().map_err(loop_error)?;
    Ok(claim)
}

pub(super) fn record_control_operation(
    database: &NativeDatabase,
    idempotency_key: &str,
    outcome: &str,
) -> Result<(), AgentRuntimeApplicationError> {
    database
        .connection()
        .map_err(loop_error)?
        .execute(
            "UPDATE loop_control_operations SET outcome = ?2 WHERE idempotency_key = ?1",
            params![idempotency_key, outcome],
        )
        .map_err(loop_error)?;
    Ok(())
}

pub(super) fn release_control_operation(
    database: &NativeDatabase,
    idempotency_key: &str,
) -> Result<(), AgentRuntimeApplicationError> {
    database
        .connection()
        .map_err(loop_error)?
        .execute(
            "DELETE FROM loop_control_operations WHERE idempotency_key = ?1 AND outcome = ''",
            [idempotency_key],
        )
        .map_err(loop_error)?;
    Ok(())
}

fn require_changed(changed: usize, message: &str) -> Result<(), AgentRuntimeApplicationError> {
    if changed == 1 {
        Ok(())
    } else {
        Err(AgentRuntimeApplicationError::Loop(message.to_string()))
    }
}

fn to_i64(value: u64) -> Result<i64, AgentRuntimeApplicationError> {
    i64::try_from(value).map_err(|_| {
        AgentRuntimeApplicationError::Validation("Loop numeric value is too large.".to_string())
    })
}

fn loop_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Loop(error.to_string())
}

fn to_sql_error(error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}
