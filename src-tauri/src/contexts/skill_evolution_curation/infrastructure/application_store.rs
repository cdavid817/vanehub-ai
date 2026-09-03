use super::application_binding_store::{
    load_application, load_application_binding, load_current_application_binding,
};
use super::application_store_finalization::{finalize, prepare_retry};
use super::application_store_intent::{
    append_intent_events, insert_decision, insert_system_policy_authorization,
    invalidate_preview_and_transition, validate_binding, validate_intent,
};
use super::SqliteCuratorRepository;
use crate::contexts::skill_evolution_curation::{application::*, domain::*};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

impl CuratorApplicationStore for SqliteCuratorRepository<'_> {
    fn existing_application(
        &mut self,
        application_id: &str,
        candidate_id: &str,
        expected_candidate_revision: u64,
        approved_witness_hash: &str,
        approved_diff_hash: &str,
        system_policy_authorization: Option<&CuratorSystemPolicyAuthorizationV1>,
    ) -> Result<Option<CuratorPreparedApplication>, CuratorApplicationStoreError> {
        load_matching_existing(
            self.connection,
            application_id,
            candidate_id,
            expected_candidate_revision,
            approved_witness_hash,
            approved_diff_hash,
            system_policy_authorization,
        )
    }

    fn application_binding(
        &mut self,
        candidate_id: &str,
    ) -> Result<CuratorApplicationBinding, CuratorApplicationStoreError> {
        load_current_application_binding(self.connection, candidate_id)
    }

    fn prepare_application_intent(
        &mut self,
        intent: &CuratorApplicationIntent,
    ) -> Result<CuratorPreparedApplication, CuratorApplicationStoreError> {
        validate_intent(intent)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CuratorApplicationStoreError::Storage)?;
        if transaction
            .query_row(
                "SELECT application_id FROM evolution_curator_applications WHERE application_id=?1",
                [&intent.application_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| CuratorApplicationStoreError::Storage)?
            .is_some()
        {
            return load_matching_existing(
                &transaction,
                &intent.application_id,
                &intent.decision.candidate_id,
                intent.decision.candidate_revision,
                &intent.approved_witness_hash,
                &intent.approved_diff_hash,
                intent.system_policy_authorization.as_ref(),
            )?
            .ok_or(CuratorApplicationStoreError::Conflict);
        }
        let binding =
            load_current_application_binding(&transaction, &intent.decision.candidate_id)?;
        validate_binding(intent, &binding)?;
        let next_revision = binding
            .decision
            .candidate_revision
            .checked_add(1)
            .ok_or(CuratorApplicationStoreError::InvalidInput)?;
        insert_decision(&transaction, intent)?;
        transaction
            .execute(
                "INSERT INTO evolution_curator_applications
                 (application_id,candidate_id,decision_id,status,approved_witness_hash,revision,
                  created_at_ms,updated_at_ms) VALUES (?1,?2,?3,'intent_recorded',?4,1,?5,?5)",
                params![
                    intent.application_id,
                    intent.decision.candidate_id,
                    intent.decision.decision_id,
                    intent.approved_witness_hash,
                    intent.decision.decided_at_ms
                ],
            )
            .map_err(|_| CuratorApplicationStoreError::Storage)?;
        insert_system_policy_authorization(&transaction, intent)?;
        transaction
            .execute(
                "INSERT INTO evolution_curator_outbox
                 (outbox_id,application_id,candidate_id,witness_hash,status,available_at_ms,created_at_ms)
                 VALUES (?1,?2,?3,?4,'pending',?5,?5)",
                params![
                    intent.outbox_id,
                    intent.application_id,
                    intent.decision.candidate_id,
                    intent.approved_witness_hash,
                    intent.decision.decided_at_ms
                ],
            )
            .map_err(|_| CuratorApplicationStoreError::Storage)?;
        invalidate_preview_and_transition(&transaction, intent, next_revision)?;
        append_intent_events(&transaction, intent, next_revision)?;
        transaction
            .commit()
            .map_err(|_| CuratorApplicationStoreError::Storage)?;
        Ok(CuratorPreparedApplication {
            application: CuratorApplication {
                application_id: intent.application_id.clone(),
                candidate_id: intent.decision.candidate_id.clone(),
                decision_id: intent.decision.decision_id.clone(),
                status: CuratorApplicationStatus::IntentRecorded,
                approved_witness_hash: intent.approved_witness_hash.clone(),
                overlay_revision: None,
                overlay_history_id: None,
                failure_code: None,
                revision: 1,
            },
            binding,
            duplicate: false,
        })
    }

    fn finalize_application(
        &mut self,
        application_id: &str,
        expected_application_revision: u64,
        result: Result<&CuratorOverlayApplicationReceipt, CuratorApplicationFailure>,
        occurred_at_ms: i64,
    ) -> Result<CuratorApplication, CuratorApplicationStoreError> {
        finalize(
            self.connection,
            application_id,
            expected_application_revision,
            result,
            occurred_at_ms,
        )
    }

    fn pending_applications(
        &mut self,
        limit: usize,
    ) -> Result<Vec<CuratorPreparedApplication>, CuratorApplicationStoreError> {
        if limit == 0 || limit > CURATOR_RECOVERY_PAGE_LIMIT {
            return Err(CuratorApplicationStoreError::InvalidInput);
        }
        let mut statement = self.connection.prepare(
            "SELECT a.application_id FROM evolution_curator_applications a
             JOIN evolution_curator_outbox o ON o.application_id=a.application_id
             WHERE a.status IN ('intent_recorded','applying') AND o.status IN ('pending','processing')
             ORDER BY o.created_at_ms,o.outbox_id LIMIT ?1",
        ).map_err(|_| CuratorApplicationStoreError::Storage)?;
        let ids = statement
            .query_map(
                [i64::try_from(limit).map_err(|_| CuratorApplicationStoreError::InvalidInput)?],
                |row| row.get::<_, String>(0),
            )
            .map_err(|_| CuratorApplicationStoreError::Storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| CuratorApplicationStoreError::Storage)?;
        ids.into_iter()
            .map(|application_id| {
                Ok(CuratorPreparedApplication {
                    application: load_application(self.connection, &application_id)?,
                    binding: load_application_binding(self.connection, &application_id)?,
                    duplicate: true,
                })
            })
            .collect()
    }

    fn prepare_failed_retry(
        &mut self,
        candidate_id: &str,
        expected_candidate_revision: u64,
        occurred_at_ms: i64,
    ) -> Result<u64, CuratorApplicationStoreError> {
        prepare_retry(
            self.connection,
            candidate_id,
            expected_candidate_revision,
            occurred_at_ms,
        )
    }
}

fn load_matching_existing(
    connection: &rusqlite::Connection,
    application_id: &str,
    candidate_id: &str,
    expected_candidate_revision: u64,
    approved_witness_hash: &str,
    approved_diff_hash: &str,
    system_policy_authorization: Option<&CuratorSystemPolicyAuthorizationV1>,
) -> Result<Option<CuratorPreparedApplication>, CuratorApplicationStoreError> {
    let existing = connection
        .query_row(
            "SELECT a.candidate_id,d.candidate_revision,a.approved_witness_hash,
                    p.effective_diff_hash
             FROM evolution_curator_applications a
             JOIN evolution_curator_decisions d ON d.decision_id=a.decision_id
             JOIN evolution_curator_previews p ON p.candidate_id=a.candidate_id
              AND p.witness_hash=a.approved_witness_hash
             WHERE a.application_id=?1",
            [application_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    let Some((stored_candidate, stored_revision, stored_preview, stored_diff)) = existing else {
        return Ok(None);
    };
    if stored_candidate != candidate_id
        || u64::try_from(stored_revision).ok() != Some(expected_candidate_revision)
        || stored_preview != approved_witness_hash
        || stored_diff != approved_diff_hash
    {
        return Err(CuratorApplicationStoreError::Conflict);
    }
    validate_existing_system_authorization(
        connection,
        application_id,
        system_policy_authorization,
    )?;
    Ok(Some(CuratorPreparedApplication {
        application: load_application(connection, application_id)?,
        binding: load_application_binding(connection, application_id)?,
        duplicate: true,
    }))
}

fn validate_existing_system_authorization(
    connection: &rusqlite::Connection,
    application_id: &str,
    expected: Option<&CuratorSystemPolicyAuthorizationV1>,
) -> Result<(), CuratorApplicationStoreError> {
    let stored = connection
        .query_row(
            "SELECT run_id,eligibility_id,eligibility_proof_hash,preflight_witness_hash,
             policy_witness_hash,rate_reservation_id,authorized_at_ms
             FROM evolution_curator_system_policy_authorizations WHERE application_id=?1",
            [application_id],
            |row| {
                Ok(CuratorSystemPolicyAuthorizationV1 {
                    run_id: row.get(0)?,
                    eligibility_id: row.get(1)?,
                    eligibility_proof_hash: row.get(2)?,
                    preflight_witness_hash: row.get(3)?,
                    policy_witness_hash: row.get(4)?,
                    rate_reservation_id: row.get(5)?,
                    authorized_at_ms: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    if stored.as_ref() != expected {
        return Err(CuratorApplicationStoreError::Conflict);
    }
    Ok(())
}
