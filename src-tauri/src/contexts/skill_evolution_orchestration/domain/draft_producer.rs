use serde::{Deserialize, Serialize};
use unicode_normalization::UnicodeNormalization;

use super::{
    canonical_hash, is_safe_identifier, sha256_bytes, DeterministicCorrectionDraftV1,
    EvolutionIntegrityError,
};

pub(crate) const AUTHORIZED_CORRECTION_PRODUCER_V1: &str =
    "authorized-correction-overlay-learn-block-v1";
pub(crate) const DETERMINISTIC_CORRECTION_PROVENANCE: &str = "deterministic_authorized_correction";
const MAX_DRAFT_BYTES: usize = 2 * 1024;
const MAX_FIELD_CHARS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AutomaticDraftProvenance {
    DeterministicAuthorizedCorrection,
    UserAuthored,
    Edited,
    ModelGenerated,
    Imported,
    ExactPatch,
    File,
    Script,
    Unknown,
}

impl AutomaticDraftProvenance {
    pub(crate) fn eligible(self) -> bool {
        matches!(self, Self::DeterministicAuthorizedCorrection)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthorizedCorrectionDraftInputV1 {
    pub(crate) workspace_id: String,
    pub(crate) target_skill_id: String,
    pub(crate) target_revision: String,
    pub(crate) authorization_id: String,
    pub(crate) authorization_witness_hash: String,
    pub(crate) assessment_id: String,
    pub(crate) sanitizer_version: u16,
    pub(crate) authorization_current: bool,
    pub(crate) trigger: String,
    pub(crate) guidance: String,
    pub(crate) verification: String,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProducedCorrectionDraftV1 {
    pub(crate) record: DeterministicCorrectionDraftV1,
    pub(crate) content: String,
    pub(crate) trigger: String,
    pub(crate) guidance: String,
    pub(crate) verification: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CorrectionDraftError {
    InvalidIdentifier,
    AuthorizationUnavailable,
    SanitizationUnavailable,
    IncompleteShape,
    UnsafeControl,
    FieldLimit,
    OutputLimit,
    Integrity,
}

pub(crate) fn produce_authorized_correction_draft(
    input: &AuthorizedCorrectionDraftInputV1,
) -> Result<ProducedCorrectionDraftV1, CorrectionDraftError> {
    validate_witnesses(input)?;
    let trigger = canonical_field(&input.trigger)?;
    let guidance = canonical_field(&input.guidance)?;
    let verification = canonical_field(&input.verification)?;
    let content = format!(
        "### Verified correction guidance\n\n- Trigger: {trigger}\n- Guidance: {guidance}\n- Verify: {verification}\n"
    );
    if content.len() > MAX_DRAFT_BYTES {
        return Err(CorrectionDraftError::OutputLimit);
    }
    let source_witness_hash = canonical_hash(&(
        AUTHORIZED_CORRECTION_PRODUCER_V1,
        &input.workspace_id,
        &input.target_skill_id,
        &input.target_revision,
        &input.authorization_id,
        &input.authorization_witness_hash,
        &input.assessment_id,
        input.sanitizer_version,
        &trigger,
        &guidance,
        &verification,
    ))
    .map_err(map_integrity)?;
    let draft_id_hash = canonical_hash(&(
        "automatic-draft",
        AUTHORIZED_CORRECTION_PRODUCER_V1,
        &source_witness_hash,
    ))
    .map_err(map_integrity)?;
    let record = DeterministicCorrectionDraftV1 {
        draft_id: format!("draft-{}", draft_id_hash.trim_start_matches("sha256:")),
        workspace_id: input.workspace_id.clone(),
        target_skill_id: input.target_skill_id.clone(),
        authorization_id: input.authorization_id.clone(),
        assessment_id: input.assessment_id.clone(),
        producer_version: AUTHORIZED_CORRECTION_PRODUCER_V1.into(),
        content_hash: sha256_bytes(content.as_bytes()),
        content_size_bytes: u16::try_from(content.len())
            .map_err(|_| CorrectionDraftError::OutputLimit)?,
        provenance: DETERMINISTIC_CORRECTION_PROVENANCE.into(),
        source_witness_hash,
        created_at_ms: input.created_at_ms,
    };
    Ok(ProducedCorrectionDraftV1 {
        record,
        content,
        trigger,
        guidance,
        verification,
    })
}

fn validate_witnesses(
    input: &AuthorizedCorrectionDraftInputV1,
) -> Result<(), CorrectionDraftError> {
    if !input.authorization_current {
        return Err(CorrectionDraftError::AuthorizationUnavailable);
    }
    if input.sanitizer_version == 0 {
        return Err(CorrectionDraftError::SanitizationUnavailable);
    }
    if !is_safe_identifier(&input.workspace_id, 256)
        || !is_safe_identifier(&input.target_skill_id, 256)
        || !is_safe_identifier(&input.target_revision, 256)
        || !is_safe_identifier(&input.authorization_id, 256)
        || !is_safe_identifier(&input.assessment_id, 256)
    {
        return Err(CorrectionDraftError::InvalidIdentifier);
    }
    Ok(())
}

fn canonical_field(value: &str) -> Result<String, CorrectionDraftError> {
    if value.chars().any(|character| {
        character == '\0' || (character.is_control() && !character.is_whitespace())
    }) {
        return Err(CorrectionDraftError::UnsafeControl);
    }
    let canonical = value.nfkc().collect::<String>();
    let canonical = canonical.split_whitespace().collect::<Vec<_>>().join(" ");
    if canonical.is_empty() {
        return Err(CorrectionDraftError::IncompleteShape);
    }
    if canonical.chars().count() > MAX_FIELD_CHARS {
        return Err(CorrectionDraftError::FieldLimit);
    }
    Ok(canonical)
}

fn map_integrity(_: EvolutionIntegrityError) -> CorrectionDraftError {
    CorrectionDraftError::Integrity
}
