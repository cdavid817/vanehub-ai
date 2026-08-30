use super::*;
use crate::contexts::skill_evolution_curation::domain::*;
use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub(crate) struct CuratorPreviewService<'a, S, O> {
    store: &'a mut S,
    overlay: &'a O,
}

impl<'a, S, O> CuratorPreviewService<'a, S, O>
where
    S: CuratorPreviewStore,
    O: CuratorOverlayPreviewPort,
{
    pub(crate) fn new(store: &'a mut S, overlay: &'a O) -> Self {
        Self { store, overlay }
    }

    pub(crate) fn create(
        &mut self,
        request: CuratorPreviewRequest<'_>,
        issued_at_ms: i64,
    ) -> Result<CuratorPreview, CuratorPreviewServiceError> {
        let binding = self.store.preview_binding(request.candidate_id)?;
        validate_request(request, &binding, issued_at_ms)?;
        let receipt = match self.overlay.preview(&binding) {
            Ok(receipt) => receipt,
            Err(error) => {
                if let Some(reason) = error.staleness {
                    self.store.invalidate_preview(&CuratorPreviewInvalidation {
                        candidate_id: binding.candidate_id,
                        expected_candidate_revision: binding.candidate_revision,
                        reason,
                        occurred_at_ms: issued_at_ms,
                    })?;
                }
                return Err(CuratorPreviewServiceError::Overlay(error));
            }
        };
        validate_preview_receipt(&binding, &receipt)
            .map_err(CuratorPreviewServiceError::InvalidReceipt)?;
        let preview = build_preview(&binding, receipt, issued_at_ms)?;
        self.store.persist_preview(&preview)?;
        Ok(preview)
    }
}

fn validate_request(
    request: CuratorPreviewRequest<'_>,
    binding: &CuratorPreviewBinding,
    issued_at_ms: i64,
) -> Result<(), CuratorPreviewServiceError> {
    if issued_at_ms < 0
        || binding.state != CuratorCandidateState::ReadyForReview
        || request.expected_candidate_revision != binding.candidate_revision
        || request.expected_draft_revision != binding.draft_revision
        || request.expected_assessment_id != binding.assessment_id
    {
        return Err(CuratorPreviewServiceError::Conflict);
    }
    Ok(())
}

fn build_preview(
    binding: &CuratorPreviewBinding,
    receipt: CuratorOverlayPreviewReceipt,
    issued_at_ms: i64,
) -> Result<CuratorPreview, CuratorPreviewServiceError> {
    let effective_diff_hash = hash(&receipt.diffs)?;
    let preview_candidate_revision = binding
        .candidate_revision
        .checked_add(1)
        .ok_or(CuratorPreviewServiceError::InvalidInput)?;
    let witness_hash = hash(&PreviewHashMaterial {
        candidate_revision: preview_candidate_revision,
        draft_revision: binding.draft_revision,
        assessment_id: &binding.assessment_id,
        effective_diff_hash: &effective_diff_hash,
        witnesses: &receipt.witnesses,
    })?;
    let expires_at_ms = issued_at_ms
        .checked_add(CURATOR_PREVIEW_TTL_MS)
        .ok_or(CuratorPreviewServiceError::InvalidInput)?;
    Ok(CuratorPreview {
        preview_id: format!("curator-preview-{witness_hash}"),
        candidate_id: binding.candidate_id.clone(),
        candidate_revision: preview_candidate_revision,
        draft_id: binding.draft_id.clone(),
        draft_revision: binding.draft_revision,
        assessment_id: binding.assessment_id.clone(),
        witness_hash,
        effective_diff_hash,
        witnesses: receipt.witnesses,
        diffs: receipt.diffs,
        validation: receipt.validation,
        issued_at_ms,
        expires_at_ms,
        invalidated_at_ms: None,
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewHashMaterial<'a> {
    candidate_revision: u64,
    draft_revision: u64,
    assessment_id: &'a str,
    effective_diff_hash: &'a str,
    witnesses: &'a CuratorPreviewWitnesses,
}

fn hash(value: &impl Serialize) -> Result<String, CuratorPreviewServiceError> {
    let bytes = serde_json::to_vec(value).map_err(|_| CuratorPreviewServiceError::InvalidInput)?;
    let digest = Sha256::digest(bytes);
    Ok(format!(
        "sha256:{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    ))
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum CuratorPreviewServiceError {
    #[error("curator preview changed concurrently")]
    Conflict,
    #[error("curator preview input is invalid")]
    InvalidInput,
    #[error("curator overlay preview receipt is invalid: {0}")]
    InvalidReceipt(&'static str),
    #[error(transparent)]
    Store(#[from] CuratorPreviewStoreError),
    #[error(transparent)]
    Overlay(#[from] CuratorOverlayPreviewError),
}
