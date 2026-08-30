use super::repository_support::{from_sql_u64, parse_state, sql_u64};
use super::CuratorRepositoryError;
use crate::contexts::skill_evolution_curation::{application::*, domain::*};
use rusqlite::{OptionalExtension, Transaction};
use serde_json::Value;

pub(super) struct BindingRow {
    candidate_revision: i64,
    candidate_hash: String,
    policy_hash: String,
    state: String,
    workspace_id: String,
    target_skill_id: String,
    target_revision: String,
    overlay_scope: String,
    draft_id: String,
    draft_revision: i64,
    draft_hash: String,
    draft_kind: String,
    body_json: String,
    base_instruction_hash: String,
    base_package_hash: String,
    current_effective_hash: String,
    overlay_revision: Option<i64>,
    pin_witness: String,
    trust_witness: String,
    conflict_witness: String,
    assessment_id: String,
    assessment_hash: String,
}

pub(super) fn load_binding_row(
    transaction: &Transaction<'_>,
    candidate_id: &str,
) -> Result<BindingRow, CuratorPreviewStoreError> {
    transaction.query_row(
        "SELECT c.revision,c.witness_hash,c.policy_witness_hash,c.state,c.workspace_id,c.target_skill_id,
         c.target_revision,c.overlay_scope,d.draft_id,d.revision,d.body_hash,d.kind,d.validated_body_json,
         d.base_hash,d.base_package_hash,d.effective_hash,d.overlay_revision,d.pin_witness,d.trust_witness,
         d.conflict_witness,a.draft_assessment_id,a.witness_hash FROM evolution_curator_candidates c
         JOIN evolution_curator_drafts d ON d.draft_id=c.current_draft_id
         JOIN evolution_curator_draft_assessments a ON a.candidate_id=c.candidate_id AND a.draft_id=d.draft_id
          AND a.draft_revision=d.revision AND a.draft_hash=d.body_hash
         WHERE c.candidate_id=?1 AND a.approvable=1 AND a.invalidated_at_ms IS NULL
         ORDER BY d.revision DESC,a.created_at_ms DESC LIMIT 1",
        [candidate_id],
        |row| Ok(BindingRow {
            candidate_revision: row.get(0)?, candidate_hash: row.get(1)?, policy_hash: row.get(2)?,
            state: row.get(3)?, workspace_id: row.get(4)?, target_skill_id: row.get(5)?,
            target_revision: row.get(6)?, overlay_scope: row.get(7)?, draft_id: row.get(8)?,
            draft_revision: row.get(9)?, draft_hash: row.get(10)?, draft_kind: row.get(11)?,
            body_json: row.get(12)?, base_instruction_hash: row.get(13)?, base_package_hash: row.get(14)?,
            current_effective_hash: row.get(15)?, overlay_revision: row.get(16)?, pin_witness: row.get(17)?,
            trust_witness: row.get(18)?, conflict_witness: row.get(19)?, assessment_id: row.get(20)?,
            assessment_hash: row.get(21)?,
        }),
    ).optional().map_err(|_| CuratorPreviewStoreError::Storage)?
        .ok_or(CuratorPreviewStoreError::NotFound)
}

impl BindingRow {
    pub(super) fn try_into_binding(
        self,
        candidate_id: &str,
    ) -> Result<CuratorPreviewBinding, CuratorPreviewStoreError> {
        Ok(CuratorPreviewBinding {
            candidate_id: candidate_id.to_string(),
            candidate_revision: from_sql_u64(self.candidate_revision).map_err(map_error)?,
            candidate_hash: self.candidate_hash,
            policy_hash: self.policy_hash,
            state: parse_state(&self.state).map_err(map_error)?,
            workspace_id: self.workspace_id,
            target_skill_id: self.target_skill_id,
            target_revision: self.target_revision,
            overlay_scope: self.overlay_scope,
            draft_id: self.draft_id,
            draft_revision: from_sql_u64(self.draft_revision).map_err(map_error)?,
            draft_hash: self.draft_hash,
            mutation: parse_mutation(&self.draft_kind, &self.body_json)?,
            base_instruction_hash: self.base_instruction_hash,
            base_package_hash: self.base_package_hash,
            current_effective_hash: self.current_effective_hash,
            overlay_revision: self
                .overlay_revision
                .map(from_sql_u64)
                .transpose()
                .map_err(map_error)?,
            pin_witness: self.pin_witness,
            trust_witness: self.trust_witness,
            conflict_witness: self.conflict_witness,
            assessment_id: self.assessment_id,
            assessment_hash: self.assessment_hash,
        })
    }
}

pub(super) fn validate_current(
    transaction: &Transaction<'_>,
    preview: &CuratorPreview,
) -> Result<(), CuratorPreviewStoreError> {
    let row = load_binding_row(transaction, &preview.candidate_id)?;
    if row.candidate_revision.checked_add(1)
        != Some(sql_u64(preview.candidate_revision).map_err(map_error)?)
        || row.state != "ready_for_review"
        || row.draft_id != preview.draft_id
        || row.draft_revision != sql_u64(preview.draft_revision).map_err(map_error)?
        || row.assessment_id != preview.assessment_id
        || row.draft_hash != preview.witnesses.draft_hash
        || row.candidate_hash != preview.witnesses.candidate_hash
        || row.policy_hash != preview.witnesses.policy_hash
    {
        return Err(CuratorPreviewStoreError::Conflict);
    }
    Ok(())
}

fn parse_mutation(
    kind: &str,
    body: &str,
) -> Result<CuratorDraftMutationInput, CuratorPreviewStoreError> {
    let value: Value = serde_json::from_str(body).map_err(|_| CuratorPreviewStoreError::Storage)?;
    match kind {
        "learn_block" => Ok(CuratorDraftMutationInput::LearnedGuidance {
            guidance: string_field(&value, "guidance")?,
        }),
        "exact_patch" => Ok(CuratorDraftMutationInput::ExactPatch {
            old_string: string_field(&value, "oldString")?,
            new_string: string_field(&value, "newString")?,
            replace_all: value
                .get("replaceAll")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }),
        _ => Err(CuratorPreviewStoreError::Storage),
    }
}

fn string_field(value: &Value, key: &str) -> Result<String, CuratorPreviewStoreError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(CuratorPreviewStoreError::Storage)
}

fn map_error(error: CuratorRepositoryError) -> CuratorPreviewStoreError {
    match error {
        CuratorRepositoryError::NotFound => CuratorPreviewStoreError::NotFound,
        CuratorRepositoryError::Conflict(_) => CuratorPreviewStoreError::Conflict,
        CuratorRepositoryError::Storage => CuratorPreviewStoreError::Storage,
        _ => CuratorPreviewStoreError::InvalidInput,
    }
}
