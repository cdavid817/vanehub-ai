use serde::Serialize;

use crate::contexts::skill_evolution_generation::{
    application::canonical_hash,
    domain::{
        GenerationConsentState, GenerationPolicyV1, GenerationProviderReadinessV1,
        GENERATION_DISCLOSURE_VERSION_V1,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationPolicyChangeSource {
    LocalInteractiveUser,
    ImportedSettings,
}

pub(crate) struct GenerationPolicyChangeV1<'a> {
    pub(crate) workspace_id: &'a str,
    pub(crate) expected_revision: u64,
    pub(crate) requested_state: GenerationConsentState,
    pub(crate) disclosure_acknowledgement: Option<&'a str>,
    pub(crate) source: GenerationPolicyChangeSource,
    pub(crate) allowed_artifact_kinds:
        Option<&'a [crate::contexts::skill_evolution_generation::domain::GeneratedArtifactKind]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationPolicyError {
    InvalidInput,
    Conflict,
    ImportedConsentForbidden,
    DisclosureRequired,
    ProviderUnavailable,
    StalePolicy,
    ConsentDenied,
    Serialization,
    Storage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenerationOutboundAuthorization {
    Allowed,
    Disabled,
    Revoked,
    DisclosureStale,
    ProviderUnavailable,
    WitnessStale,
}

pub(crate) trait GenerationPolicyPort: Send + Sync {
    fn load(&self, workspace_id: &str)
        -> Result<Option<GenerationPolicyV1>, GenerationPolicyError>;

    fn save(
        &self,
        policy: &GenerationPolicyV1,
        expected_revision: u64,
    ) -> Result<(), GenerationPolicyError>;
}

pub(crate) fn update_generation_policy(
    port: &dyn GenerationPolicyPort,
    change: &GenerationPolicyChangeV1<'_>,
    readiness: Option<&GenerationProviderReadinessV1>,
    now_ms: i64,
) -> Result<GenerationPolicyV1, GenerationPolicyError> {
    let current = port.load(change.workspace_id)?;
    let policy = evolve_generation_policy(current.as_ref(), change, readiness, now_ms)?;
    port.save(&policy, change.expected_revision)?;
    Ok(policy)
}

pub(crate) fn evolve_generation_policy(
    current: Option<&GenerationPolicyV1>,
    change: &GenerationPolicyChangeV1<'_>,
    readiness: Option<&GenerationProviderReadinessV1>,
    now_ms: i64,
) -> Result<GenerationPolicyV1, GenerationPolicyError> {
    validate_change(current, change, now_ms)?;
    if change.source == GenerationPolicyChangeSource::ImportedSettings
        && change.requested_state == GenerationConsentState::Enabled
    {
        return Err(GenerationPolicyError::ImportedConsentForbidden);
    }
    let mut policy = current
        .cloned()
        .unwrap_or_else(|| GenerationPolicyV1::default_disabled(change.workspace_id.into()));
    policy.workspace_id = change.workspace_id.into();
    policy.revision = change.expected_revision + 1;
    policy.updated_at_ms = now_ms;
    policy.consent_state = change.requested_state;
    if let Some(kinds) = change.allowed_artifact_kinds {
        if kinds.is_empty() || kinds.len() > 3 {
            return Err(GenerationPolicyError::InvalidInput);
        }
        let mut unique = kinds.to_vec();
        unique.sort_by_key(|kind| *kind as u8);
        unique.dedup();
        if unique.len() != kinds.len() {
            return Err(GenerationPolicyError::InvalidInput);
        }
        policy.allowed_artifact_kinds = kinds.to_vec();
    }
    if change.requested_state == GenerationConsentState::Enabled {
        if change.disclosure_acknowledgement != Some(GENERATION_DISCLOSURE_VERSION_V1) {
            return Err(GenerationPolicyError::DisclosureRequired);
        }
        let readiness = readiness
            .filter(|value| provider_is_ready(value))
            .ok_or(GenerationPolicyError::ProviderUnavailable)?;
        policy.disclosure_version = GENERATION_DISCLOSURE_VERSION_V1.into();
        policy.provider_profile_id = Some(readiness.profile_id.clone());
        policy.model_id = Some(readiness.model_id.clone());
    }
    policy.consent_hash = consent_hash(&policy)?;
    policy.policy_hash = policy_hash(&policy)?;
    Ok(policy)
}

pub(crate) fn authorize_outbound_generation(
    policy: &GenerationPolicyV1,
    readiness: Option<&GenerationProviderReadinessV1>,
    frozen_policy_hash: &str,
    frozen_consent_hash: &str,
) -> GenerationOutboundAuthorization {
    match policy.consent_state {
        GenerationConsentState::Disabled => return GenerationOutboundAuthorization::Disabled,
        GenerationConsentState::Revoked => return GenerationOutboundAuthorization::Revoked,
        GenerationConsentState::DisclosureStale => {
            return GenerationOutboundAuthorization::DisclosureStale;
        }
        GenerationConsentState::Enabled => {}
    }
    if policy.disclosure_version != GENERATION_DISCLOSURE_VERSION_V1 {
        return GenerationOutboundAuthorization::DisclosureStale;
    }
    if policy.policy_hash != frozen_policy_hash || policy.consent_hash != frozen_consent_hash {
        return GenerationOutboundAuthorization::WitnessStale;
    }
    if !readiness.is_some_and(|value| {
        provider_is_ready(value)
            && policy.provider_profile_id.as_deref() == Some(value.profile_id.as_str())
            && policy.model_id.as_deref() == Some(value.model_id.as_str())
    }) {
        return GenerationOutboundAuthorization::ProviderUnavailable;
    }
    GenerationOutboundAuthorization::Allowed
}

fn validate_change(
    current: Option<&GenerationPolicyV1>,
    change: &GenerationPolicyChangeV1<'_>,
    now_ms: i64,
) -> Result<(), GenerationPolicyError> {
    if change.workspace_id.trim().is_empty()
        || change.workspace_id.len() > 512
        || now_ms < 0
        || current.is_some_and(|policy| {
            policy.workspace_id != change.workspace_id
                || policy.revision != change.expected_revision
        })
        || (current.is_none() && change.expected_revision != 0)
    {
        return Err(if current.is_some() {
            GenerationPolicyError::Conflict
        } else {
            GenerationPolicyError::InvalidInput
        });
    }
    Ok(())
}

fn provider_is_ready(readiness: &GenerationProviderReadinessV1) -> bool {
    readiness.enabled
        && readiness.credentials_available
        && readiness.structured_json_supported
        && !readiness.profile_id.trim().is_empty()
        && !readiness.model_id.trim().is_empty()
        && matches!(
            readiness.provider_protocol.as_str(),
            "openai_responses" | "anthropic_messages" | "gemini_generate_content"
        )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConsentWitness<'a> {
    workspace_id: &'a str,
    consent_state: GenerationConsentState,
    disclosure_version: &'a str,
    provider_profile_id: &'a Option<String>,
    model_id: &'a Option<String>,
    revision: u64,
}

fn consent_hash(policy: &GenerationPolicyV1) -> Result<String, GenerationPolicyError> {
    canonical_hash(&ConsentWitness {
        workspace_id: &policy.workspace_id,
        consent_state: policy.consent_state,
        disclosure_version: &policy.disclosure_version,
        provider_profile_id: &policy.provider_profile_id,
        model_id: &policy.model_id,
        revision: policy.revision,
    })
    .map_err(|_| GenerationPolicyError::Serialization)
}

fn policy_hash(policy: &GenerationPolicyV1) -> Result<String, GenerationPolicyError> {
    canonical_hash(&(
        policy.schema_version,
        &policy.workspace_id,
        policy.consent_state,
        &policy.disclosure_version,
        &policy.provider_profile_id,
        &policy.model_id,
        &policy.allowed_artifact_kinds,
        &policy.job_budget,
        &policy.daily_budget,
        &policy.retention,
        &policy.consent_hash,
        policy.revision,
    ))
    .map_err(|_| GenerationPolicyError::Serialization)
}

pub(crate) fn policy_integrity_is_valid(policy: &GenerationPolicyV1) -> bool {
    consent_hash(policy).is_ok_and(|hash| hash == policy.consent_hash)
        && policy_hash(policy).is_ok_and(|hash| hash == policy.policy_hash)
}
