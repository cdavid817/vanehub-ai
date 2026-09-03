use super::*;
use crate::contexts::skill_evolution_curation::domain::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub(crate) struct CuratorDraftService<'a, S, V> {
    store: &'a mut S,
    overlay: &'a V,
}

impl<'a, S, V> CuratorDraftService<'a, S, V>
where
    S: CuratorDraftStore,
    V: CuratorOverlayDraftValidationPort,
{
    pub(crate) fn new(store: &'a mut S, overlay: &'a V) -> Self {
        Self { store, overlay }
    }

    pub(crate) fn create_revision(
        &mut self,
        request: &CuratorDraftRequestV1,
        occurred_at_ms: i64,
    ) -> Result<CuratorDraftRevision, CuratorDraftServiceError> {
        let binding = self.store.candidate_binding(&request.candidate_id)?;
        if let Err(reason) = validate_request_shape(request, &binding) {
            self.reject(
                request,
                reason,
                CURATOR_DRAFT_POLICY_VERSION,
                occurred_at_ms,
            )?;
            return Err(CuratorDraftServiceError::Rejected(reason.to_string()));
        }
        let receipt = match self.overlay.dry_validate(&binding, &request.mutation) {
            Ok(receipt) => receipt,
            Err(error) => {
                self.reject(
                    request,
                    &error.reason_code,
                    &error.scanner_version,
                    occurred_at_ms,
                )?;
                return Err(CuratorDraftServiceError::Rejected(error.reason_code));
            }
        };
        let body = normalized_body(&request.mutation);
        let body_hash = hash_json(&body)?;
        let draft = CuratorDraftRevision {
            draft_id: draft_id(&binding.candidate_id),
            candidate_id: binding.candidate_id,
            revision: binding.next_draft_revision,
            kind: request.mutation.kind(),
            target_skill_id: binding.target_skill_id,
            target_revision: binding.target_revision,
            overlay_scope: binding.overlay_scope,
            body_hash,
            evidence_ids: binding.evidence_ids,
            rationale: request.rationale.clone(),
            expected_effective_change: request.expected_effective_change.clone(),
            base_hash: receipt.base_hash,
            base_package_hash: receipt.base_package_hash,
            effective_hash: receipt.effective_hash,
            overlay_revision: receipt.overlay_revision,
            pin_witness: receipt.pin_witness,
            trust_witness: receipt.trust_witness,
            conflict_witness: receipt.conflict_witness,
            created_at_ms: occurred_at_ms,
        };
        let prepared = PreparedCuratorDraft {
            draft: draft.clone(),
            body,
            scanner_version: receipt.scanner_version,
            expected_candidate_revision: request.expected_candidate_revision,
        };
        self.store
            .persist_prepared_draft(&prepared, occurred_at_ms)?;
        Ok(draft)
    }

    fn reject(
        &mut self,
        request: &CuratorDraftRequestV1,
        reason: &str,
        scanner_version: &str,
        occurred_at_ms: i64,
    ) -> Result<(), CuratorDraftServiceError> {
        self.store.record_draft_rejection(
            &request.candidate_id,
            request.expected_candidate_revision,
            reason,
            scanner_version,
            occurred_at_ms,
        )?;
        Ok(())
    }
}

fn normalized_body(mutation: &CuratorDraftMutationInput) -> Value {
    match mutation {
        CuratorDraftMutationInput::LearnedGuidance { guidance } => json!({ "guidance": guidance }),
        CuratorDraftMutationInput::ExactPatch {
            old_string,
            new_string,
            replace_all,
        } => {
            json!({ "oldString": old_string, "newString": new_string, "replaceAll": replace_all })
        }
    }
}

fn draft_id(candidate_id: &str) -> String {
    let digest = Sha256::digest(format!("curator-draft-v1|{candidate_id}").as_bytes());
    format!(
        "draft-{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn hash_json(value: &Value) -> Result<String, CuratorDraftServiceError> {
    let bytes = serde_json::to_vec(value).map_err(|_| CuratorDraftServiceError::InvalidInput)?;
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
pub(crate) enum CuratorDraftServiceError {
    #[error("curator draft was rejected: {0}")]
    Rejected(String),
    #[error("curator draft input is invalid")]
    InvalidInput,
    #[error(transparent)]
    Store(#[from] CuratorDraftStoreError),
}
