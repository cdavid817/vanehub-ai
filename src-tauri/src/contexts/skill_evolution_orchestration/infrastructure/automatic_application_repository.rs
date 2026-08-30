use crate::{
    contexts::skill_evolution_orchestration::domain::{
        canonical_json, is_safe_identifier, AutoApplyProbationV1, AutomaticEvolutionApplicationV1,
        EvolutionActorProvenance, ProbationStatus, ROLLING_WEEK_MS,
    },
    platform::database::NativeDatabase,
};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

use super::automatic_application_store_support::probation_matches;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutomaticApplicationStoreError {
    InvalidInput,
    Conflict,
    NotFound,
    Storage,
}

#[derive(Clone)]
pub(crate) struct SqliteAutomaticApplicationRepository {
    database: NativeDatabase,
}

impl SqliteAutomaticApplicationRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn finalize(
        &self,
        application: &AutomaticEvolutionApplicationV1,
        probation: &AutoApplyProbationV1,
        run_item_id: &str,
        expected_rate_revision: u64,
    ) -> Result<bool, AutomaticApplicationStoreError> {
        validate(application, probation, run_item_id)?;
        let expected_rate_revision = i64::try_from(expected_rate_revision)
            .map_err(|_| AutomaticApplicationStoreError::InvalidInput)?;
        let categories = canonical_json(&probation.evidence_categories)
            .map_err(|_| AutomaticApplicationStoreError::InvalidInput)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| AutomaticApplicationStoreError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AutomaticApplicationStoreError::Storage)?;
        if let Some(existing) = load_application(&transaction, &application.application_id)? {
            return if existing == *application && probation_matches(&transaction, probation)? {
                Ok(false)
            } else {
                Err(AutomaticApplicationStoreError::Conflict)
            };
        }
        verify_chain(
            &transaction,
            application,
            run_item_id,
            expected_rate_revision,
        )?;
        insert_application(&transaction, application)?;
        transaction
            .execute(
                "INSERT INTO evolution_auto_probations VALUES
                 (?1,?2,?3,?4,'active',?5,?6,?7,?8,?9,?10,?11,0)",
                params![
                    probation.probation_id,
                    probation.application_id,
                    probation.workspace_id,
                    probation.skill_id,
                    probation.prior_effective_hash,
                    probation.current_effective_hash,
                    probation.evidence_fingerprint,
                    categories,
                    probation.baseline_witness_hash,
                    probation.starts_at_ms,
                    probation.ends_at_ms,
                ],
            )
            .map_err(|_| AutomaticApplicationStoreError::Storage)?;
        let next_revision = expected_rate_revision
            .checked_add(1)
            .ok_or(AutomaticApplicationStoreError::InvalidInput)?;
        let rate_changed = transaction
            .execute(
                "UPDATE evolution_auto_rate_reservations
                 SET status='committed',application_id=?1,updated_at_ms=?2,revision=?3
                 WHERE reservation_id=?4 AND status='reserved' AND revision=?5",
                params![
                    application.application_id,
                    application.committed_at_ms,
                    next_revision,
                    application.rate_reservation_id,
                    expected_rate_revision,
                ],
            )
            .map_err(|_| AutomaticApplicationStoreError::Storage)?;
        let item_changed = transaction
            .execute(
                "UPDATE evolution_run_items SET committed_receipt_id=?1
                 WHERE item_id=?2 AND committed_receipt_id IS NULL",
                params![application.application_id, run_item_id],
            )
            .map_err(|_| AutomaticApplicationStoreError::Storage)?;
        if rate_changed != 1 || item_changed != 1 {
            return Err(AutomaticApplicationStoreError::Conflict);
        }
        transaction
            .commit()
            .map_err(|_| AutomaticApplicationStoreError::Storage)?;
        Ok(true)
    }
}

fn validate(
    application: &AutomaticEvolutionApplicationV1,
    probation: &AutoApplyProbationV1,
    run_item_id: &str,
) -> Result<(), AutomaticApplicationStoreError> {
    let identifiers = [
        application.application_id.as_str(),
        application.run_id.as_str(),
        application.eligibility_id.as_str(),
        application.rate_reservation_id.as_str(),
        application.target_skill_id.as_str(),
        run_item_id,
        probation.probation_id.as_str(),
        probation.workspace_id.as_str(),
    ];
    if identifiers
        .iter()
        .any(|value| !is_safe_identifier(value, 256))
        || application.actor != EvolutionActorProvenance::SystemPolicy
        || application.application_id != application.curator_application_id
        || application.application_id != application.overlay_application_id
        || application.application_id != probation.application_id
        || application.target_skill_id != probation.skill_id
        || application.prior_effective_hash != probation.prior_effective_hash
        || application.resulting_effective_hash != probation.current_effective_hash
        || application.committed_at_ms < 0
        || probation.status != ProbationStatus::Active
        || probation.starts_at_ms != application.committed_at_ms
        || probation.ends_at_ms != probation.starts_at_ms.saturating_add(ROLLING_WEEK_MS)
        || probation.revision != 0
        || probation.evidence_categories.is_empty()
        || probation.evidence_categories.len() > 32
    {
        return Err(AutomaticApplicationStoreError::InvalidInput);
    }
    Ok(())
}

fn verify_chain(
    transaction: &Transaction<'_>,
    value: &AutomaticEvolutionApplicationV1,
    run_item_id: &str,
    expected_rate_revision: i64,
) -> Result<(), AutomaticApplicationStoreError> {
    let exists: bool = transaction
        .query_row(
            "SELECT EXISTS(
               SELECT 1 FROM evolution_runs r
               JOIN evolution_auto_eligibility e ON e.run_id=r.run_id
               JOIN evolution_auto_preflight_witnesses p ON p.eligibility_id=e.eligibility_id
               JOIN evolution_auto_rate_reservations rr ON rr.reservation_id=p.reservation_id
               JOIN evolution_curator_system_policy_authorizations a
                 ON a.preflight_witness_hash=p.proof_hash
               JOIN evolution_curator_applications ca ON ca.application_id=a.application_id
               JOIN evolution_run_items i ON i.run_id=r.run_id
               WHERE r.run_id=?1 AND r.policy_witness_hash=?2
                 AND e.eligibility_id=?3 AND e.proof_hash=p.eligibility_proof_hash
                 AND p.proof_hash=?4 AND p.status='consumed'
                 AND rr.reservation_id=?5 AND rr.status='reserved' AND rr.revision=?6
                 AND a.application_id=?7 AND a.run_id=r.run_id
                 AND a.eligibility_id=e.eligibility_id AND a.eligibility_proof_hash=e.proof_hash
                 AND a.policy_witness_hash=r.policy_witness_hash
                 AND a.rate_reservation_id=rr.reservation_id
                 AND ca.status IN ('applied','reconciled') AND ca.overlay_history_id IS NOT NULL
                 AND i.item_id=?8 AND i.stage='evaluate_auto_apply'
                 AND i.source_id=e.eligibility_id AND i.committed_receipt_id IS NULL
             )",
            params![
                value.run_id,
                value.policy_witness_hash,
                value.eligibility_id,
                value.preflight_witness_hash,
                value.rate_reservation_id,
                expected_rate_revision,
                value.application_id,
                run_item_id,
            ],
            |row| row.get(0),
        )
        .map_err(|_| AutomaticApplicationStoreError::Storage)?;
    if !exists {
        return Err(AutomaticApplicationStoreError::Conflict);
    }
    Ok(())
}

fn insert_application(
    transaction: &Transaction<'_>,
    value: &AutomaticEvolutionApplicationV1,
) -> Result<(), AutomaticApplicationStoreError> {
    transaction
        .execute(
            "INSERT INTO evolution_auto_applications VALUES
             (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,'system_policy',?12)",
            params![
                value.application_id,
                value.run_id,
                value.eligibility_id,
                value.preflight_witness_hash,
                value.policy_witness_hash,
                value.rate_reservation_id,
                value.curator_application_id,
                value.overlay_application_id,
                value.target_skill_id,
                value.prior_effective_hash,
                value.resulting_effective_hash,
                value.committed_at_ms,
            ],
        )
        .map_err(|_| AutomaticApplicationStoreError::Storage)?;
    Ok(())
}

fn load_application(
    transaction: &Transaction<'_>,
    application_id: &str,
) -> Result<Option<AutomaticEvolutionApplicationV1>, AutomaticApplicationStoreError> {
    transaction
        .query_row(
            "SELECT application_id,run_id,eligibility_id,preflight_witness_hash,
             policy_witness_hash,rate_reservation_id,curator_application_id,
             overlay_application_id,target_skill_id,prior_effective_hash,
             resulting_effective_hash,committed_at_ms FROM evolution_auto_applications
             WHERE application_id=?1",
            [application_id],
            |row| {
                Ok(AutomaticEvolutionApplicationV1 {
                    application_id: row.get(0)?,
                    run_id: row.get(1)?,
                    eligibility_id: row.get(2)?,
                    preflight_witness_hash: row.get(3)?,
                    policy_witness_hash: row.get(4)?,
                    rate_reservation_id: row.get(5)?,
                    curator_application_id: row.get(6)?,
                    overlay_application_id: row.get(7)?,
                    target_skill_id: row.get(8)?,
                    prior_effective_hash: row.get(9)?,
                    resulting_effective_hash: row.get(10)?,
                    actor: EvolutionActorProvenance::SystemPolicy,
                    committed_at_ms: row.get(11)?,
                })
            },
        )
        .optional()
        .map_err(|_| AutomaticApplicationStoreError::Storage)
}
