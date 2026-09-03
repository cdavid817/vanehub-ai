use crate::contexts::skill_evolution_orchestration::domain::{
    canonical_json, orchestration_idempotency_key, EvolutionActorProvenance,
    EvolutionTriggerCountersV1, EvolutionTriggerEnvelopeV1, EvolutionTriggerFamily,
    ORCHESTRATION_SCHEMA_VERSION_V1,
};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

use super::{actor_name, validate_trigger, OrchestrationPersistenceError, OrchestrationRepository};

const AUTOMATIC_DEBOUNCE_MS: i64 = 30_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ReceiveTriggerOutcome {
    Duplicate {
        receipt_id: String,
    },
    Queued {
        receipt_id: String,
        request_id: String,
        created_request: bool,
        follow_up: bool,
        not_before_ms: i64,
    },
}

impl OrchestrationRepository {
    pub(crate) fn receive_trigger(
        &self,
        trigger: &EvolutionTriggerEnvelopeV1,
        received_at_ms: i64,
    ) -> Result<ReceiveTriggerOutcome, OrchestrationPersistenceError> {
        validate_trigger(trigger)?;
        if received_at_ms < 0 {
            return Err(OrchestrationPersistenceError::InvalidInput);
        }
        let mut connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        if let Some(receipt_id) = insert_receipt(&transaction, trigger, received_at_ms)? {
            transaction
                .commit()
                .map_err(|_| OrchestrationPersistenceError::Storage)?;
            return Ok(ReceiveTriggerOutcome::Duplicate { receipt_id });
        }

        let follow_up = active_run_exists(&transaction, &trigger.workspace_id)?;
        let requested_not_before = if trigger.family == EvolutionTriggerFamily::ManualRunRequest {
            received_at_ms
        } else {
            received_at_ms
                .checked_add(AUTOMATIC_DEBOUNCE_MS)
                .ok_or(OrchestrationPersistenceError::InvalidInput)?
        };
        let existing = pending_request(&transaction, &trigger.workspace_id, follow_up)?;
        let (request_id, created_request, not_before_ms) = match existing {
            Some((request_id, counters_json, current_not_before, revision)) => {
                let mut counters = decode_counters(&counters_json)?;
                counters
                    .increment(trigger.family)
                    .ok_or(OrchestrationPersistenceError::InvalidInput)?;
                let not_before_ms = if trigger.family == EvolutionTriggerFamily::ManualRunRequest {
                    current_not_before.min(requested_not_before)
                } else {
                    current_not_before
                };
                update_request(
                    &transaction,
                    &request_id,
                    counters,
                    trigger.actor,
                    not_before_ms,
                    revision,
                    received_at_ms,
                )?;
                (request_id, false, not_before_ms)
            }
            None => {
                let request_id = request_id(trigger, follow_up)?;
                let mut counters = EvolutionTriggerCountersV1::default();
                counters
                    .increment(trigger.family)
                    .ok_or(OrchestrationPersistenceError::InvalidInput)?;
                insert_request(
                    &transaction,
                    &request_id,
                    trigger,
                    counters,
                    follow_up,
                    requested_not_before,
                    received_at_ms,
                )?;
                (request_id, true, requested_not_before)
            }
        };
        transaction
            .execute(
                "INSERT INTO evolution_run_request_trigger_links (request_id,receipt_id)
                 VALUES (?1,?2)",
                params![request_id, trigger.trigger_id],
            )
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        transaction
            .commit()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        Ok(ReceiveTriggerOutcome::Queued {
            receipt_id: trigger.trigger_id.clone(),
            request_id,
            created_request,
            follow_up,
            not_before_ms,
        })
    }
}

fn insert_receipt(
    transaction: &Transaction<'_>,
    trigger: &EvolutionTriggerEnvelopeV1,
    received_at_ms: i64,
) -> Result<Option<String>, OrchestrationPersistenceError> {
    let source_revision = i64::try_from(trigger.source_revision)
        .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
    let safe_reasons = canonical_json(&trigger.safe_reason_codes)
        .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
    let changed = transaction
        .execute(
            "INSERT OR IGNORE INTO evolution_trigger_receipts
             (receipt_id,schema_version,family,workspace_id,source_kind,source_id,
              source_revision,occurred_at_ms,priority,safe_reason_codes_json,actor,created_at_ms)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                trigger.trigger_id,
                trigger.schema_version,
                trigger.family.as_str(),
                trigger.workspace_id,
                trigger.source_kind,
                trigger.source_id,
                source_revision,
                trigger.occurred_at_ms,
                trigger.priority,
                safe_reasons,
                actor_name(trigger.actor),
                received_at_ms,
            ],
        )
        .map_err(|_| OrchestrationPersistenceError::Storage)?;
    if changed == 1 {
        return Ok(None);
    }
    transaction
        .query_row(
            "SELECT receipt_id FROM evolution_trigger_receipts
             WHERE family=?1 AND source_kind=?2 AND source_id=?3 AND source_revision=?4",
            params![
                trigger.family.as_str(),
                trigger.source_kind,
                trigger.source_id,
                source_revision,
            ],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| OrchestrationPersistenceError::Storage)?
        .ok_or(OrchestrationPersistenceError::Conflict)
        .map(Some)
}

fn active_run_exists(
    transaction: &Transaction<'_>,
    workspace_id: &str,
) -> Result<bool, OrchestrationPersistenceError> {
    transaction
        .query_row(
            "SELECT 1 FROM evolution_runs WHERE workspace_id=?1 AND status IN
             ('requested','waiting_idle','running','partial','cancel_requested') LIMIT 1",
            [workspace_id],
            |_| Ok(()),
        )
        .optional()
        .map(|value| value.is_some())
        .map_err(|_| OrchestrationPersistenceError::Storage)
}

type PendingRequest = (String, String, i64, i64);

fn pending_request(
    transaction: &Transaction<'_>,
    workspace_id: &str,
    follow_up: bool,
) -> Result<Option<PendingRequest>, OrchestrationPersistenceError> {
    transaction
        .query_row(
            "SELECT request_id,trigger_counters_json,not_before_ms,revision
             FROM evolution_run_requests WHERE workspace_id=?1 AND follow_up=?2
             AND status IN ('pending','claimed') LIMIT 1",
            params![workspace_id, follow_up],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(|_| OrchestrationPersistenceError::Storage)
}

fn request_id(
    trigger: &EvolutionTriggerEnvelopeV1,
    follow_up: bool,
) -> Result<String, OrchestrationPersistenceError> {
    let digest = orchestration_idempotency_key(
        "run-request",
        "coalesce",
        &(&trigger.workspace_id, follow_up, &trigger.trigger_id),
    )
    .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
    Ok(format!("request:{digest}"))
}

fn decode_counters(
    json: &str,
) -> Result<EvolutionTriggerCountersV1, OrchestrationPersistenceError> {
    serde_json::from_str(json).map_err(|_| OrchestrationPersistenceError::Corrupt)
}

fn update_request(
    transaction: &Transaction<'_>,
    request_id: &str,
    counters: EvolutionTriggerCountersV1,
    actor: EvolutionActorProvenance,
    not_before_ms: i64,
    revision: i64,
    updated_at_ms: i64,
) -> Result<(), OrchestrationPersistenceError> {
    let counters = canonical_json(&counters).map_err(|_| OrchestrationPersistenceError::Storage)?;
    let next_revision = revision
        .checked_add(1)
        .ok_or(OrchestrationPersistenceError::InvalidInput)?;
    let changed = transaction
        .execute(
            "UPDATE evolution_run_requests SET actor=?1,trigger_counters_json=?2,
             not_before_ms=?3,revision=?4,updated_at_ms=?5
             WHERE request_id=?6 AND revision=?7",
            params![
                actor_name(actor),
                counters,
                not_before_ms,
                next_revision,
                updated_at_ms,
                request_id,
                revision,
            ],
        )
        .map_err(|_| OrchestrationPersistenceError::Storage)?;
    if changed == 1 {
        Ok(())
    } else {
        Err(OrchestrationPersistenceError::Conflict)
    }
}

fn insert_request(
    transaction: &Transaction<'_>,
    request_id: &str,
    trigger: &EvolutionTriggerEnvelopeV1,
    counters: EvolutionTriggerCountersV1,
    follow_up: bool,
    not_before_ms: i64,
    created_at_ms: i64,
) -> Result<(), OrchestrationPersistenceError> {
    let counters = canonical_json(&counters).map_err(|_| OrchestrationPersistenceError::Storage)?;
    transaction
        .execute(
            "INSERT INTO evolution_run_requests
             (request_id,schema_version,workspace_id,actor,status,trigger_counters_json,
              follow_up,not_before_ms,claimed_run_id,revision,created_at_ms,updated_at_ms)
             VALUES (?1,?2,?3,?4,'pending',?5,?6,?7,NULL,0,?8,?8)",
            params![
                request_id,
                ORCHESTRATION_SCHEMA_VERSION_V1,
                trigger.workspace_id,
                actor_name(trigger.actor),
                counters,
                follow_up,
                not_before_ms,
                created_at_ms,
            ],
        )
        .map_err(|_| OrchestrationPersistenceError::Storage)?;
    Ok(())
}
