use crate::contexts::skill_evolution_orchestration::domain::{
    apply_policy_mutation, canonical_hash, import_policy_without_local_consent, is_safe_identifier,
    revoke_policy_consent, EvolutionOrchestrationPolicyV1, EvolutionPolicyMutationV1,
};
use rusqlite::{params, Transaction, TransactionBehavior};
use std::collections::BTreeSet;

use super::{
    load_policy, map_policy_error, persist_policy, OrchestrationPersistenceError,
    OrchestrationRepository,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PolicyWriteResult {
    pub(crate) policy: EvolutionOrchestrationPolicyV1,
    pub(crate) invalidated_eligibility: usize,
}

impl OrchestrationRepository {
    pub(crate) fn policy(
        &self,
        workspace_id: &str,
        now_ms: i64,
    ) -> Result<EvolutionOrchestrationPolicyV1, OrchestrationPersistenceError> {
        validate_policy_lookup(workspace_id, now_ms)?;
        let connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        load_policy(&connection, workspace_id)?.map_or_else(
            || {
                Ok(EvolutionOrchestrationPolicyV1::default_off(
                    workspace_id.into(),
                    now_ms,
                ))
            },
            Ok,
        )
    }

    pub(crate) fn update_policy(
        &self,
        workspace_id: &str,
        mutation: EvolutionPolicyMutationV1,
    ) -> Result<PolicyWriteResult, OrchestrationPersistenceError> {
        validate_policy_lookup(workspace_id, mutation.updated_at_ms)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let persisted = load_policy(&transaction, workspace_id)?;
        let exists = persisted.is_some();
        let current = persisted.unwrap_or_else(|| {
            EvolutionOrchestrationPolicyV1::default_off(workspace_id.into(), mutation.updated_at_ms)
        });
        let next = apply_policy_mutation(&current, mutation).map_err(map_policy_error)?;
        let removed = removed_skill_ids(&current, &next);
        persist_policy(&transaction, &next, current.revision, exists)?;
        let invalidated_eligibility = invalidate_removed_eligibility(
            &transaction,
            workspace_id,
            &removed,
            next.updated_at_ms,
        )?;
        transaction
            .commit()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        Ok(PolicyWriteResult {
            policy: next,
            invalidated_eligibility,
        })
    }

    pub(crate) fn revoke_policy_consent(
        &self,
        workspace_id: &str,
        expected_revision: u64,
        revoked_at_ms: i64,
    ) -> Result<PolicyWriteResult, OrchestrationPersistenceError> {
        let current = self.policy(workspace_id, revoked_at_ms)?;
        if current.revision != expected_revision {
            return Err(OrchestrationPersistenceError::Conflict);
        }
        let revoked = revoke_policy_consent(&current, revoked_at_ms).map_err(map_policy_error)?;
        self.replace_existing_policy(&current, revoked)
    }

    pub(crate) fn import_policy(
        &self,
        source: &EvolutionOrchestrationPolicyV1,
        workspace_id: &str,
        now_ms: i64,
    ) -> Result<PolicyWriteResult, OrchestrationPersistenceError> {
        let imported = import_policy_without_local_consent(source, workspace_id.into(), now_ms)
            .map_err(map_policy_error)?;
        self.insert_policy(imported)
    }

    fn replace_existing_policy(
        &self,
        current: &EvolutionOrchestrationPolicyV1,
        next: EvolutionOrchestrationPolicyV1,
    ) -> Result<PolicyWriteResult, OrchestrationPersistenceError> {
        let mut connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        persist_policy(&transaction, &next, current.revision, true)?;
        let removed = removed_skill_ids(current, &next);
        let invalidated_eligibility = invalidate_removed_eligibility(
            &transaction,
            &next.workspace_id,
            &removed,
            next.updated_at_ms,
        )?;
        transaction
            .commit()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        Ok(PolicyWriteResult {
            policy: next,
            invalidated_eligibility,
        })
    }

    fn insert_policy(
        &self,
        policy: EvolutionOrchestrationPolicyV1,
    ) -> Result<PolicyWriteResult, OrchestrationPersistenceError> {
        let mut connection = self
            .database
            .connection()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        persist_policy(&transaction, &policy, 0, false)?;
        transaction
            .commit()
            .map_err(|_| OrchestrationPersistenceError::Storage)?;
        Ok(PolicyWriteResult {
            policy,
            invalidated_eligibility: 0,
        })
    }
}

fn validate_policy_lookup(
    workspace_id: &str,
    now_ms: i64,
) -> Result<(), OrchestrationPersistenceError> {
    if !is_safe_identifier(workspace_id, 128) || now_ms < 0 {
        return Err(OrchestrationPersistenceError::InvalidInput);
    }
    Ok(())
}

fn removed_skill_ids(
    current: &EvolutionOrchestrationPolicyV1,
    next: &EvolutionOrchestrationPolicyV1,
) -> Vec<String> {
    let next_ids: BTreeSet<_> = next.allowed_skill_ids.iter().collect();
    current
        .allowed_skill_ids
        .iter()
        .filter(|id| !next_ids.contains(id))
        .cloned()
        .collect()
}

fn invalidate_removed_eligibility(
    transaction: &Transaction<'_>,
    workspace_id: &str,
    removed: &[String],
    now_ms: i64,
) -> Result<usize, OrchestrationPersistenceError> {
    let mut total = 0;
    for skill_id in removed {
        let proof_hash =
            canonical_hash(&(workspace_id, skill_id, now_ms, "policy-allowlist-removed"))
                .map_err(|_| OrchestrationPersistenceError::InvalidInput)?;
        total += transaction.execute("UPDATE evolution_auto_eligibility SET result='ineligible',predicates_json='[{\"condition\":\"policy_allowlist\",\"passed\":false,\"safeReasonCode\":\"policy-allowlist-removed\",\"witnessHash\":null}]',proof_hash=?1,overlay_preview_hash=NULL,evaluated_at_ms=?2,revision=revision+1 WHERE target_skill_id=?3 AND result!='ineligible' AND run_id IN (SELECT run_id FROM evolution_runs WHERE workspace_id=?4)", params![proof_hash, now_ms, skill_id, workspace_id]).map_err(|_| OrchestrationPersistenceError::Storage)?;
    }
    Ok(total)
}
