use super::decision_binding_store::load_decision_binding;
use super::repository_support::from_sql_u64;
use crate::contexts::skill_evolution_curation::{application::*, domain::*};
use rusqlite::{Connection, OptionalExtension};
use serde_json::Value;

pub(super) fn load_current_application_binding(
    connection: &Connection,
    candidate_id: &str,
) -> Result<CuratorApplicationBinding, CuratorApplicationStoreError> {
    let decision = load_decision_binding(connection, candidate_id).map_err(map_decision_error)?;
    let row = connection
        .query_row(
            "SELECT c.workspace_id,c.target_skill_id,c.overlay_scope,d.kind,d.validated_body_json,
                    p.witnesses_json
             FROM evolution_curator_candidates c
             JOIN evolution_curator_drafts d ON d.draft_id=c.current_draft_id
             JOIN evolution_curator_previews p ON p.preview_id=c.current_preview_id
             WHERE c.candidate_id=?1 AND p.invalidated_at_ms IS NULL",
            [candidate_id],
            read_binding_row,
        )
        .optional()
        .map_err(|_| CuratorApplicationStoreError::Storage)?
        .ok_or(CuratorApplicationStoreError::NotFound)?;
    binding_from_row(decision, row)
}

pub(super) fn load_application_binding(
    connection: &Connection,
    application_id: &str,
) -> Result<CuratorApplicationBinding, CuratorApplicationStoreError> {
    let (candidate_id, row) = connection
        .query_row(
            "SELECT a.candidate_id,c.workspace_id,c.target_skill_id,c.overlay_scope,d.kind,
                    d.validated_body_json,p.witnesses_json
             FROM evolution_curator_applications a
             JOIN evolution_curator_candidates c ON c.candidate_id=a.candidate_id
             JOIN evolution_curator_decisions decision ON decision.decision_id=a.decision_id
             JOIN evolution_curator_previews p ON p.candidate_id=a.candidate_id
              AND p.witness_hash=a.approved_witness_hash
             JOIN evolution_curator_drafts d ON d.draft_id=p.draft_id AND d.revision=p.draft_revision
             WHERE a.application_id=?1",
            [application_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    BindingRow {
                        workspace_id: row.get(1)?,
                        target_skill_id: row.get(2)?,
                        overlay_scope: row.get(3)?,
                        draft_kind: row.get(4)?,
                        body_json: row.get(5)?,
                        witnesses_json: row.get(6)?,
                    },
                ))
            },
        )
        .optional()
        .map_err(|_| CuratorApplicationStoreError::Storage)?
        .ok_or(CuratorApplicationStoreError::NotFound)?;
    let decision = load_decision_binding(connection, &candidate_id).map_err(map_decision_error)?;
    binding_from_row(decision, row)
}

struct BindingRow {
    workspace_id: String,
    target_skill_id: String,
    overlay_scope: String,
    draft_kind: String,
    body_json: String,
    witnesses_json: String,
}

fn read_binding_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BindingRow> {
    Ok(BindingRow {
        workspace_id: row.get(0)?,
        target_skill_id: row.get(1)?,
        overlay_scope: row.get(2)?,
        draft_kind: row.get(3)?,
        body_json: row.get(4)?,
        witnesses_json: row.get(5)?,
    })
}

fn binding_from_row(
    decision: CuratorDecisionBinding,
    row: BindingRow,
) -> Result<CuratorApplicationBinding, CuratorApplicationStoreError> {
    let witnesses: CuratorPreviewWitnesses = serde_json::from_str(&row.witnesses_json)
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    let expected_pinned = match witnesses.pin_witness.as_str() {
        "pin-v1:false" => false,
        "pin-v1:true" => true,
        _ => return Err(CuratorApplicationStoreError::Storage),
    };
    Ok(CuratorApplicationBinding {
        decision,
        workspace_id: row.workspace_id,
        target_skill_id: row.target_skill_id,
        overlay_scope: row.overlay_scope,
        mutation: parse_mutation(&row.draft_kind, &row.body_json)?,
        overlay_witnesses: CuratorApplicationOverlayWitnesses {
            expected_overlay_revision: witnesses.overlay_revision,
            base_instruction_hash: witnesses.base_instruction_hash,
            base_package_hash: witnesses.base_package_hash,
            proposed_effective_hash: witnesses.proposed_effective_hash,
            expected_pinned,
        },
    })
}

fn parse_mutation(
    kind: &str,
    body: &str,
) -> Result<CuratorDraftMutationInput, CuratorApplicationStoreError> {
    let value: Value =
        serde_json::from_str(body).map_err(|_| CuratorApplicationStoreError::Storage)?;
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
        _ => Err(CuratorApplicationStoreError::Storage),
    }
}

fn string_field(value: &Value, key: &str) -> Result<String, CuratorApplicationStoreError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(CuratorApplicationStoreError::Storage)
}

pub(super) fn load_application(
    connection: &Connection,
    application_id: &str,
) -> Result<CuratorApplication, CuratorApplicationStoreError> {
    connection
        .query_row(
            "SELECT application_id,candidate_id,decision_id,status,approved_witness_hash,
                    overlay_revision,overlay_history_id,failure_code,revision
             FROM evolution_curator_applications WHERE application_id=?1",
            [application_id],
            |row| {
                Ok(CuratorApplication {
                    application_id: row.get(0)?,
                    candidate_id: row.get(1)?,
                    decision_id: row.get(2)?,
                    status: parse_status(&row.get::<_, String>(3)?)?,
                    approved_witness_hash: row.get(4)?,
                    overlay_revision: row.get(5)?,
                    overlay_history_id: row.get(6)?,
                    failure_code: row.get(7)?,
                    revision: from_sql_u64(row.get(8)?).map_err(sql_error)?,
                })
            },
        )
        .optional()
        .map_err(|_| CuratorApplicationStoreError::Storage)?
        .ok_or(CuratorApplicationStoreError::NotFound)
}

fn parse_status(value: &str) -> rusqlite::Result<CuratorApplicationStatus> {
    match value {
        "intent_recorded" => Ok(CuratorApplicationStatus::IntentRecorded),
        "applying" => Ok(CuratorApplicationStatus::Applying),
        "applied" => Ok(CuratorApplicationStatus::Applied),
        "failed" => Ok(CuratorApplicationStatus::Failed),
        "reconciled" => Ok(CuratorApplicationStatus::Reconciled),
        _ => Err(sql_error(super::CuratorRepositoryError::Storage)),
    }
}

fn sql_error(_: super::CuratorRepositoryError) -> rusqlite::Error {
    rusqlite::Error::InvalidQuery
}

fn map_decision_error(error: CuratorDecisionStoreError) -> CuratorApplicationStoreError {
    match error {
        CuratorDecisionStoreError::NotFound => CuratorApplicationStoreError::NotFound,
        CuratorDecisionStoreError::Conflict => CuratorApplicationStoreError::Conflict,
        CuratorDecisionStoreError::Storage => CuratorApplicationStoreError::Storage,
        CuratorDecisionStoreError::InvalidInput => CuratorApplicationStoreError::InvalidInput,
    }
}
