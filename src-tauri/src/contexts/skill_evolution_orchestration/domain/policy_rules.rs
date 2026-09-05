use super::{
    canonical_hash, is_safe_identifier, EvolutionActorProvenance, EvolutionConsentWitnessV1,
    EvolutionOrchestrationPolicyV1, EvolutionPolicyMode, ORCHESTRATION_DISCLOSURE_VERSION_V1,
    ORCHESTRATION_SCHEMA_VERSION_V1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvolutionPolicyError {
    InvalidWorkspace,
    InvalidSkillId,
    RevisionConflict,
    InvalidTime,
    ConsentRequired,
    AllowlistRequired,
    Integrity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EvolutionPolicyMutationV1 {
    pub(crate) expected_revision: u64,
    pub(crate) mode: EvolutionPolicyMode,
    pub(crate) allowed_skill_ids: Vec<String>,
    pub(crate) acknowledge_current_disclosure: bool,
    pub(crate) notify_routine_completion: bool,
    pub(crate) updated_at_ms: i64,
}

pub(crate) fn apply_policy_mutation(
    current: &EvolutionOrchestrationPolicyV1,
    mutation: EvolutionPolicyMutationV1,
) -> Result<EvolutionOrchestrationPolicyV1, EvolutionPolicyError> {
    validate_policy_identity(current)?;
    if mutation.expected_revision != current.revision {
        return Err(EvolutionPolicyError::RevisionConflict);
    }
    if mutation.updated_at_ms < current.updated_at_ms {
        return Err(EvolutionPolicyError::InvalidTime);
    }
    let revision = current
        .revision
        .checked_add(1)
        .ok_or(EvolutionPolicyError::RevisionConflict)?;
    let allowed_skill_ids = normalize_skill_ids(mutation.allowed_skill_ids)?;
    let consent = if mutation.acknowledge_current_disclosure {
        Some(consent_witness(
            &current.workspace_id,
            revision,
            mutation.updated_at_ms,
        )?)
    } else {
        current.consent.clone()
    };
    if mutation.mode == EvolutionPolicyMode::Enabled {
        if allowed_skill_ids.is_empty() {
            return Err(EvolutionPolicyError::AllowlistRequired);
        }
        if !consent.as_ref().is_some_and(current_consent) {
            return Err(EvolutionPolicyError::ConsentRequired);
        }
    }
    Ok(EvolutionOrchestrationPolicyV1 {
        schema_version: ORCHESTRATION_SCHEMA_VERSION_V1,
        workspace_id: current.workspace_id.clone(),
        mode: mutation.mode,
        allowed_skill_ids,
        consent,
        automatic_budget: current.automatic_budget.clone(),
        manual_budget: current.manual_budget.clone(),
        user_idle_ms: current.user_idle_ms,
        maximum_idle_wait_ms: current.maximum_idle_wait_ms,
        notify_routine_completion: mutation.notify_routine_completion,
        revision,
        created_at_ms: current.created_at_ms,
        updated_at_ms: mutation.updated_at_ms,
    })
}

pub(crate) fn revoke_policy_consent(
    current: &EvolutionOrchestrationPolicyV1,
    revoked_at_ms: i64,
) -> Result<EvolutionOrchestrationPolicyV1, EvolutionPolicyError> {
    validate_policy_identity(current)?;
    if revoked_at_ms < current.updated_at_ms {
        return Err(EvolutionPolicyError::InvalidTime);
    }
    let mut revoked = current.clone();
    revoked.mode = EvolutionPolicyMode::Off;
    if let Some(consent) = &mut revoked.consent {
        consent.revoked_at_ms = Some(revoked_at_ms);
    }
    revoked.revision = revoked
        .revision
        .checked_add(1)
        .ok_or(EvolutionPolicyError::RevisionConflict)?;
    revoked.updated_at_ms = revoked_at_ms;
    Ok(revoked)
}

pub(crate) fn import_policy_without_local_consent(
    source: &EvolutionOrchestrationPolicyV1,
    workspace_id: String,
    now_ms: i64,
) -> Result<EvolutionOrchestrationPolicyV1, EvolutionPolicyError> {
    validate_policy_integrity(source)?;
    if !is_safe_identifier(&workspace_id, 128) || now_ms < 0 {
        return Err(EvolutionPolicyError::InvalidWorkspace);
    }
    let allowed_skill_ids = normalize_skill_ids(source.allowed_skill_ids.clone())?;
    Ok(EvolutionOrchestrationPolicyV1 {
        schema_version: ORCHESTRATION_SCHEMA_VERSION_V1,
        workspace_id,
        mode: if source.mode == EvolutionPolicyMode::Enabled {
            EvolutionPolicyMode::Observe
        } else {
            source.mode
        },
        allowed_skill_ids,
        consent: None,
        automatic_budget: source.automatic_budget.clone(),
        manual_budget: source.manual_budget.clone(),
        user_idle_ms: source.user_idle_ms,
        maximum_idle_wait_ms: source.maximum_idle_wait_ms,
        notify_routine_completion: source.notify_routine_completion,
        revision: 0,
        created_at_ms: now_ms,
        updated_at_ms: now_ms,
    })
}

pub(crate) fn policy_allows_skill(policy: &EvolutionOrchestrationPolicyV1, skill_id: &str) -> bool {
    policy.mode == EvolutionPolicyMode::Enabled
        && policy.consent.as_ref().is_some_and(current_consent)
        && policy
            .allowed_skill_ids
            .iter()
            .any(|allowed| allowed == skill_id)
}

pub(crate) fn validate_policy_integrity(
    policy: &EvolutionOrchestrationPolicyV1,
) -> Result<(), EvolutionPolicyError> {
    validate_policy_identity(policy)?;
    if policy.created_at_ms < 0
        || policy.updated_at_ms < policy.created_at_ms
        || policy.maximum_idle_wait_ms == 0
        || normalize_skill_ids(policy.allowed_skill_ids.clone())? != policy.allowed_skill_ids
    {
        return Err(EvolutionPolicyError::Integrity);
    }
    if policy.mode == EvolutionPolicyMode::Enabled
        && (policy.allowed_skill_ids.is_empty()
            || !policy.consent.as_ref().is_some_and(current_consent))
    {
        return Err(EvolutionPolicyError::Integrity);
    }
    if let Some(consent) = &policy.consent {
        if consent.actor != EvolutionActorProvenance::InteractiveUser
            || consent.acknowledged_policy_revision > policy.revision
            || consent.acknowledged_at_ms < policy.created_at_ms
            || consent
                .revoked_at_ms
                .is_some_and(|revoked| revoked < consent.acknowledged_at_ms)
            || consent.witness_hash
                != consent_hash(
                    &policy.workspace_id,
                    consent.acknowledged_policy_revision,
                    &consent.disclosure_version,
                    consent.acknowledged_at_ms,
                )?
        {
            return Err(EvolutionPolicyError::Integrity);
        }
    }
    Ok(())
}

fn validate_policy_identity(
    policy: &EvolutionOrchestrationPolicyV1,
) -> Result<(), EvolutionPolicyError> {
    if policy.schema_version != ORCHESTRATION_SCHEMA_VERSION_V1
        || !is_safe_identifier(&policy.workspace_id, 128)
    {
        return Err(EvolutionPolicyError::InvalidWorkspace);
    }
    Ok(())
}

fn normalize_skill_ids(mut skill_ids: Vec<String>) -> Result<Vec<String>, EvolutionPolicyError> {
    if skill_ids
        .iter()
        .any(|skill_id| !is_safe_identifier(skill_id, 128) || skill_id.contains('*'))
    {
        return Err(EvolutionPolicyError::InvalidSkillId);
    }
    skill_ids.sort();
    skill_ids.dedup();
    Ok(skill_ids)
}

fn current_consent(consent: &EvolutionConsentWitnessV1) -> bool {
    consent.disclosure_version == ORCHESTRATION_DISCLOSURE_VERSION_V1
        && consent.revoked_at_ms.is_none()
        && consent.actor == EvolutionActorProvenance::InteractiveUser
}

fn consent_witness(
    workspace_id: &str,
    policy_revision: u64,
    acknowledged_at_ms: i64,
) -> Result<EvolutionConsentWitnessV1, EvolutionPolicyError> {
    let witness_hash = consent_hash(
        workspace_id,
        policy_revision,
        ORCHESTRATION_DISCLOSURE_VERSION_V1,
        acknowledged_at_ms,
    )?;
    Ok(EvolutionConsentWitnessV1 {
        disclosure_version: ORCHESTRATION_DISCLOSURE_VERSION_V1.into(),
        acknowledged_policy_revision: policy_revision,
        actor: EvolutionActorProvenance::InteractiveUser,
        acknowledged_at_ms,
        revoked_at_ms: None,
        witness_hash,
    })
}

fn consent_hash(
    workspace_id: &str,
    policy_revision: u64,
    disclosure_version: &str,
    acknowledged_at_ms: i64,
) -> Result<String, EvolutionPolicyError> {
    canonical_hash(&(
        workspace_id,
        policy_revision,
        disclosure_version,
        acknowledged_at_ms,
        "interactive_user",
    ))
    .map_err(|_| EvolutionPolicyError::Integrity)
}
