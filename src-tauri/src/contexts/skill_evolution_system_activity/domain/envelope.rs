use super::sanitization::{sanitize_navigation, sanitize_payload, sanitize_text};
use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use thiserror::Error;

const MAX_IDENTITY_CHARS: usize = 160;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EvolutionActivityEnvelopeV1 {
    pub(crate) schema_version: u16,
    pub(crate) event_id: String,
    pub(crate) event_code: ActivityEventCode,
    pub(crate) source_domain: String,
    pub(crate) source_id: String,
    pub(crate) source_revision: String,
    pub(crate) source_sequence: u64,
    pub(crate) scope_kind: ActivityScopeKind,
    pub(crate) canonical_scope_id: String,
    pub(crate) occurred_at_ms: i64,
    pub(crate) committed_at_ms: i64,
    pub(crate) severity: ActivitySeverity,
    pub(crate) status: ActivityStatus,
    pub(crate) attention_kind: ActivityAttentionKind,
    pub(crate) safe_actor_kind: ActivityActorKind,
    pub(crate) safe_identities: Vec<SafeIdentity>,
    pub(crate) metrics: BTreeMap<ActivityMetricCode, i64>,
    pub(crate) reason_codes: Vec<ActivityReasonCode>,
    pub(crate) navigation: Option<ActivityNavigation>,
    pub(crate) supersedes_event_id: Option<String>,
    pub(crate) payload: Option<ActivityPayloadV1>,
    pub(crate) projection_policy_version: u16,
    pub(crate) content_hash: String,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum ActivityEnvelopeError {
    #[error("unsupported activity envelope schema version: {0}")]
    UnsupportedSchemaVersion(u16),
    #[error("activity envelope field is invalid: {0}")]
    InvalidField(&'static str),
    #[error("activity envelope exceeds {MAX_ENVELOPE_BYTES} bytes")]
    EnvelopeTooLarge,
    #[error("activity payload exceeds {MAX_PAYLOAD_BYTES} bytes")]
    PayloadTooLarge,
    #[error("activity envelope serialization failed")]
    Serialization,
    #[error("activity envelope content hash mismatch")]
    HashMismatch,
}

impl EvolutionActivityEnvelopeV1 {
    pub(crate) fn seal(mut self) -> Result<Self, ActivityEnvelopeError> {
        self.sanitize_fields()?;
        self.content_hash.clear();
        self.validate_fields()?;
        self.content_hash = hash_bytes(&canonical_bytes(&self)?);
        self.validate()?;
        Ok(self)
    }

    pub(crate) fn validate(&self) -> Result<(), ActivityEnvelopeError> {
        self.validate_fields()?;
        let payload_size = self
            .payload
            .as_ref()
            .map(serialized_size)
            .transpose()?
            .unwrap_or(0);
        if payload_size > MAX_PAYLOAD_BYTES {
            return Err(ActivityEnvelopeError::PayloadTooLarge);
        }
        if serialized_size(self)? > MAX_ENVELOPE_BYTES {
            return Err(ActivityEnvelopeError::EnvelopeTooLarge);
        }
        let mut unsigned = self.clone();
        let actual = std::mem::take(&mut unsigned.content_hash);
        if actual != hash_bytes(&canonical_bytes(&unsigned)?) {
            return Err(ActivityEnvelopeError::HashMismatch);
        }
        Ok(())
    }

    fn validate_fields(&self) -> Result<(), ActivityEnvelopeError> {
        if self.schema_version != ACTIVITY_SCHEMA_VERSION_V1 {
            return Err(ActivityEnvelopeError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        for (value, field) in [
            (&self.event_id, "event_id"),
            (&self.source_domain, "source_domain"),
            (&self.source_id, "source_id"),
            (&self.source_revision, "source_revision"),
            (&self.canonical_scope_id, "canonical_scope_id"),
        ] {
            validate_canonical_text(value, field, MAX_IDENTITY_CHARS)?;
        }
        if self.safe_identities.len() > MAX_SAFE_IDENTITIES {
            return Err(ActivityEnvelopeError::InvalidField("safe_identities"));
        }
        if self.metrics.len() > MAX_METRICS {
            return Err(ActivityEnvelopeError::InvalidField("metrics"));
        }
        if self.reason_codes.len() > MAX_REASON_CODES {
            return Err(ActivityEnvelopeError::InvalidField("reason_codes"));
        }
        for identity in &self.safe_identities {
            validate_canonical_text(&identity.value, "safe_identity.value", MAX_IDENTITY_CHARS)?;
        }
        if let Some(navigation) = &self.navigation {
            validate_canonical_text(
                &navigation.stable_id,
                "navigation.stable_id",
                MAX_IDENTITY_CHARS,
            )?;
            if let Some(child) = &navigation.child_id {
                validate_canonical_text(child, "navigation.child_id", MAX_IDENTITY_CHARS)?;
            }
        }
        if let Some(supersedes) = &self.supersedes_event_id {
            validate_canonical_text(supersedes, "supersedes_event_id", MAX_IDENTITY_CHARS)?;
        }
        if let Some(payload) = &self.payload {
            let mut canonical = payload.clone();
            sanitize_payload(&mut canonical, MAX_IDENTITY_CHARS)?;
            if &canonical != payload {
                return Err(ActivityEnvelopeError::InvalidField("payload.text"));
            }
        }
        Ok(())
    }

    fn sanitize_fields(&mut self) -> Result<(), ActivityEnvelopeError> {
        for (value, field) in [
            (&mut self.event_id, "event_id"),
            (&mut self.source_domain, "source_domain"),
            (&mut self.source_id, "source_id"),
            (&mut self.source_revision, "source_revision"),
            (&mut self.canonical_scope_id, "canonical_scope_id"),
        ] {
            *value = sanitize_text(value, field, MAX_IDENTITY_CHARS)?;
        }
        for identity in &mut self.safe_identities {
            identity.value =
                sanitize_text(&identity.value, "safe_identity.value", MAX_IDENTITY_CHARS)?;
        }
        if let Some(navigation) = &mut self.navigation {
            sanitize_navigation(navigation, MAX_IDENTITY_CHARS)?;
        }
        if let Some(supersedes) = &mut self.supersedes_event_id {
            *supersedes = sanitize_text(supersedes, "supersedes_event_id", MAX_IDENTITY_CHARS)?;
        }
        if let Some(payload) = &mut self.payload {
            sanitize_payload(payload, MAX_IDENTITY_CHARS)?;
        }
        Ok(())
    }
}

fn validate_canonical_text(
    value: &str,
    field: &'static str,
    max: usize,
) -> Result<(), ActivityEnvelopeError> {
    if sanitize_text(value, field, max)? != value {
        return Err(ActivityEnvelopeError::InvalidField(field));
    }
    Ok(())
}

fn serialized_size<T: Serialize>(value: &T) -> Result<usize, ActivityEnvelopeError> {
    Ok(serde_json::to_vec(value)
        .map_err(|_| ActivityEnvelopeError::Serialization)?
        .len())
}

fn canonical_bytes(value: &EvolutionActivityEnvelopeV1) -> Result<Vec<u8>, ActivityEnvelopeError> {
    serde_json::to_vec(value).map_err(|_| ActivityEnvelopeError::Serialization)
}

fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{}", hex_digest(&digest))
}

fn hex_digest(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
