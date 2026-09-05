use serde::Serialize;

use super::{canonical_hash, is_safe_identifier, EvolutionIntegrityError};

pub(crate) const AUTOMATIC_PREFLIGHT_POLICY_V1: &str = "automatic-preflight-v1";
pub(crate) const AUTOMATIC_PREFLIGHT_TTL_MS: i64 = 5_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AutomaticPreflightInputV1 {
    pub(crate) run_id: String,
    pub(crate) eligibility_id: String,
    pub(crate) eligibility_proof_hash: String,
    pub(crate) reservation_id: String,
    pub(crate) overlay_preview_hash: String,
    pub(crate) automatic_mode_enabled: bool,
    pub(crate) policy_current: bool,
    pub(crate) consent_current: bool,
    pub(crate) authorization_current: bool,
    pub(crate) allowlist_current: bool,
    pub(crate) assessment_current: bool,
    pub(crate) draft_current: bool,
    pub(crate) target_current: bool,
    pub(crate) skill_mutable: bool,
    pub(crate) overlay_revision_current: bool,
    pub(crate) overlay_trusted: bool,
    pub(crate) overlay_unpinned: bool,
    pub(crate) quality_current: bool,
    pub(crate) rate_reserved: bool,
    pub(crate) idle_snapshot_fresh: bool,
    pub(crate) probation_clear: bool,
    pub(crate) circuit_breakers_closed: bool,
    pub(crate) issued_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AutomaticPreflightWitnessV1 {
    pub(crate) witness_id: String,
    pub(crate) run_id: String,
    pub(crate) eligibility_id: String,
    pub(crate) eligibility_proof_hash: String,
    pub(crate) reservation_id: String,
    pub(crate) overlay_preview_hash: String,
    pub(crate) proof_hash: String,
    pub(crate) issued_at_ms: i64,
    pub(crate) expires_at_ms: i64,
    pub(crate) revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AutomaticPreflightError {
    InvalidInput,
    Failed(&'static str),
    Integrity,
}

pub(crate) fn evaluate_automatic_preflight(
    input: &AutomaticPreflightInputV1,
) -> Result<AutomaticPreflightWitnessV1, AutomaticPreflightError> {
    validate_identifiers(input)?;
    let conditions = [
        ("automatic-mode-disabled", input.automatic_mode_enabled),
        ("policy-stale", input.policy_current),
        ("consent-stale", input.consent_current),
        ("authorization-stale", input.authorization_current),
        ("allowlist-stale", input.allowlist_current),
        ("assessment-stale", input.assessment_current),
        ("draft-stale", input.draft_current),
        ("target-stale", input.target_current),
        ("skill-immutable", input.skill_mutable),
        ("overlay-revision-stale", input.overlay_revision_current),
        ("overlay-untrusted", input.overlay_trusted),
        ("target-pinned", input.overlay_unpinned),
        ("quality-stale", input.quality_current),
        ("rate-reservation-stale", input.rate_reserved),
        ("idle-snapshot-stale", input.idle_snapshot_fresh),
        ("probation-blocked", input.probation_clear),
        ("circuit-breaker-open", input.circuit_breakers_closed),
    ];
    if let Some((reason, _)) = conditions.iter().find(|(_, passed)| !passed) {
        return Err(AutomaticPreflightError::Failed(reason));
    }
    let proof_hash =
        canonical_hash(&(AUTOMATIC_PREFLIGHT_POLICY_V1, input)).map_err(map_integrity)?;
    let witness_hash =
        canonical_hash(&("preflight-witness-id", &proof_hash)).map_err(map_integrity)?;
    let expires_at_ms = input
        .issued_at_ms
        .checked_add(AUTOMATIC_PREFLIGHT_TTL_MS)
        .ok_or(AutomaticPreflightError::InvalidInput)?;
    Ok(AutomaticPreflightWitnessV1 {
        witness_id: format!("preflight-{}", witness_hash.trim_start_matches("sha256:")),
        run_id: input.run_id.clone(),
        eligibility_id: input.eligibility_id.clone(),
        eligibility_proof_hash: input.eligibility_proof_hash.clone(),
        reservation_id: input.reservation_id.clone(),
        overlay_preview_hash: input.overlay_preview_hash.clone(),
        proof_hash,
        issued_at_ms: input.issued_at_ms,
        expires_at_ms,
        revision: 0,
    })
}

fn validate_identifiers(input: &AutomaticPreflightInputV1) -> Result<(), AutomaticPreflightError> {
    if input.issued_at_ms < 0
        || !is_safe_identifier(&input.run_id, 256)
        || !is_safe_identifier(&input.eligibility_id, 256)
        || !is_safe_identifier(&input.reservation_id, 256)
        || !valid_hash(&input.eligibility_proof_hash)
        || !valid_hash(&input.overlay_preview_hash)
    {
        return Err(AutomaticPreflightError::InvalidInput);
    }
    Ok(())
}

fn valid_hash(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn map_integrity(_: EvolutionIntegrityError) -> AutomaticPreflightError {
    AutomaticPreflightError::Integrity
}
