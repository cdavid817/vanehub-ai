use super::repository_support::*;
use super::*;
use crate::contexts::skill_evolution_curation::{application::*, domain::*};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

impl CuratorDraftStore for SqliteCuratorRepository<'_> {
    fn candidate_binding(
        &mut self,
        candidate_id: &str,
    ) -> Result<CuratorDraftCandidateBinding, CuratorDraftStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CuratorDraftStoreError::Storage)?;
        let binding = load_binding(&transaction, candidate_id).map_err(map_repository_error)?;
        transaction
            .commit()
            .map_err(|_| CuratorDraftStoreError::Storage)?;
        Ok(binding)
    }

    fn persist_prepared_draft(
        &mut self,
        prepared: &PreparedCuratorDraft,
        occurred_at_ms: i64,
    ) -> Result<u64, CuratorDraftStoreError> {
        let document = ValidatedDraftDocument::from_validated_value(
            prepared.draft.kind,
            &prepared.body,
            &prepared.scanner_version,
        )
        .map_err(|_| CuratorDraftStoreError::InvalidInput)?;
        if document.body_hash() != prepared.draft.body_hash {
            return Err(CuratorDraftStoreError::InvalidInput);
        }
        self.persist_validated_draft(&DraftPersistence {
            draft: &prepared.draft,
            document: &document,
            expected_candidate_revision: prepared.expected_candidate_revision,
            occurred_at_ms,
        })
        .map_err(map_repository_error)
    }

    fn record_draft_rejection(
        &mut self,
        candidate_id: &str,
        expected_revision: u64,
        reason_code: &str,
        scanner_version: &str,
        occurred_at_ms: i64,
    ) -> Result<(), CuratorDraftStoreError> {
        if !valid_reason(reason_code) || !valid_reason(scanner_version) {
            return Err(CuratorDraftStoreError::InvalidInput);
        }
        let audit_reason = format!("{scanner_version}__{reason_code}");
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CuratorDraftStoreError::Storage)?;
        let (state, revision) =
            current_candidate(&transaction, candidate_id).map_err(map_repository_error)?;
        if revision != expected_revision {
            return Err(CuratorDraftStoreError::Conflict);
        }
        if is_terminal(state) {
            return Err(CuratorDraftStoreError::InvalidInput);
        }
        append_audit_event(
            &transaction,
            &CandidateTransitionRequest {
                candidate_id,
                expected_revision,
                transition: CuratorTransition::DraftChanged,
                event_kind: CuratorEventKind::DraftRejected,
                reason_code: Some(&audit_reason),
                audit: TrustedAuditContext::local_interactive_user(occurred_at_ms),
            },
            state,
            state,
            revision,
        )
        .map_err(map_repository_error)?;
        transaction
            .commit()
            .map_err(|_| CuratorDraftStoreError::Storage)
    }
}

impl SqliteCuratorRepository<'_> {
    pub(crate) fn persist_validated_draft(
        &mut self,
        input: &DraftPersistence<'_>,
    ) -> Result<u64, CuratorRepositoryError> {
        validate_draft(input)?;
        let evidence_json = serde_json::to_string(&input.draft.evidence_ids)
            .map_err(|_| CuratorRepositoryError::InvalidInput)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CuratorRepositoryError::Storage)?;
        let binding = load_binding(&transaction, &input.draft.candidate_id)?;
        validate_binding(input, &binding)?;
        transaction
            .execute(
                "INSERT INTO evolution_curator_drafts
            (draft_id,candidate_id,revision,kind,target_skill_id,target_revision,overlay_scope,
             validated_body_json,body_hash,rationale,expected_effective_change,evidence_ids_json,
             scanner_version,base_hash,base_package_hash,effective_hash,overlay_revision,
             pin_witness,trust_witness,conflict_witness,created_at_ms)
            VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21)",
                params![
                    input.draft.draft_id,
                    input.draft.candidate_id,
                    sql_u64(input.draft.revision)?,
                    draft_kind_name(input.draft.kind),
                    input.draft.target_skill_id,
                    input.draft.target_revision,
                    input.draft.overlay_scope,
                    input.document.body_json(),
                    input.document.body_hash(),
                    input.draft.rationale,
                    input.draft.expected_effective_change,
                    evidence_json,
                    input.document.scanner_version(),
                    input.draft.base_hash,
                    input.draft.base_package_hash,
                    input.draft.effective_hash,
                    input.draft.overlay_revision.map(sql_u64).transpose()?,
                    input.draft.pin_witness,
                    input.draft.trust_witness,
                    input.draft.conflict_witness,
                    input.draft.created_at_ms,
                ],
            )
            .map_err(|_| CuratorRepositoryError::Storage)?;
        invalidate_previous(&transaction, input)?;
        let next_revision = binding.candidate_revision + 1;
        let updated = transaction.execute("UPDATE evolution_curator_candidates SET current_draft_id=?1,
            current_preview_id=NULL,state='awaiting_draft',staleness_json='[\"draft_changed\"]',revision=?2,updated_at_ms=?3
            WHERE candidate_id=?4 AND revision=?5", params![input.draft.draft_id,sql_u64(next_revision)?,
            input.occurred_at_ms,input.draft.candidate_id,sql_u64(binding.candidate_revision)?])
            .map_err(|_| CuratorRepositoryError::Storage)?;
        if updated != 1 {
            return Err(CuratorRepositoryError::Storage);
        }
        append_audit_event(
            &transaction,
            &CandidateTransitionRequest {
                candidate_id: &input.draft.candidate_id,
                expected_revision: binding.candidate_revision,
                transition: CuratorTransition::DraftChanged,
                event_kind: CuratorEventKind::DraftRevised,
                reason_code: Some("validated_draft_revision_created"),
                audit: TrustedAuditContext::local_interactive_user(input.occurred_at_ms),
            },
            binding.state,
            CuratorCandidateState::AwaitingDraft,
            next_revision,
        )?;
        transaction
            .commit()
            .map_err(|_| CuratorRepositoryError::Storage)?;
        Ok(next_revision)
    }
}

fn load_binding(
    transaction: &rusqlite::Transaction<'_>,
    candidate_id: &str,
) -> Result<CuratorDraftCandidateBinding, CuratorRepositoryError> {
    let row = transaction
        .query_row(
            "SELECT revision,target_skill_id,target_revision,overlay_scope,workspace_id,state
        FROM evolution_curator_candidates WHERE candidate_id=?1",
            [candidate_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => CuratorRepositoryError::NotFound,
            _ => CuratorRepositoryError::Storage,
        })?;
    let state = parse_state(&row.5)?;
    if !matches!(
        state,
        CuratorCandidateState::AwaitingDraft
            | CuratorCandidateState::ReadyForReview
            | CuratorCandidateState::ApplyFailed
    ) {
        return Err(CuratorRepositoryError::InvalidInput);
    }
    let mut statement = transaction.prepare("SELECT evidence_id FROM evolution_curator_candidate_sources WHERE candidate_id=?1 ORDER BY evidence_id")
        .map_err(|_| CuratorRepositoryError::Storage)?;
    let evidence_ids = statement
        .query_map([candidate_id], |row| row.get(0))
        .map_err(|_| CuratorRepositoryError::Storage)?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|_| CuratorRepositoryError::Storage)?;
    let next = transaction
        .query_row(
            "SELECT MAX(revision) FROM evolution_curator_drafts WHERE candidate_id=?1",
            [candidate_id],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(|_| CuratorRepositoryError::Storage)?
        .flatten()
        .unwrap_or(0)
        + 1;
    Ok(CuratorDraftCandidateBinding {
        candidate_id: candidate_id.to_owned(),
        candidate_revision: from_sql_u64(row.0)?,
        state,
        target_skill_id: row.1,
        target_revision: row.2,
        overlay_scope: row.3,
        workspace_id: row.4,
        evidence_ids,
        next_draft_revision: from_sql_u64(next)?,
    })
}

fn validate_draft(input: &DraftPersistence<'_>) -> Result<(), CuratorRepositoryError> {
    let draft = input.draft;
    if draft.revision == 0
        || draft.rationale.is_empty()
        || draft.rationale.len() > 2_048
        || draft.expected_effective_change.is_empty()
        || draft.expected_effective_change.len() > 2_048
        || draft.base_hash.is_empty()
        || draft.effective_hash.is_empty()
        || draft.body_hash != input.document.body_hash()
        || draft.created_at_ms != input.occurred_at_ms
    {
        return Err(CuratorRepositoryError::InvalidInput);
    }
    Ok(())
}

fn validate_binding(
    input: &DraftPersistence<'_>,
    binding: &CuratorDraftCandidateBinding,
) -> Result<(), CuratorRepositoryError> {
    if binding.candidate_revision != input.expected_candidate_revision {
        return Err(CuratorRepositoryError::Conflict(CandidateConflict {
            current_revision: binding.candidate_revision,
            current_state: CuratorCandidateState::AwaitingDraft,
        }));
    }
    if input.draft.revision != binding.next_draft_revision
        || input.draft.target_skill_id != binding.target_skill_id
        || input.draft.target_revision != binding.target_revision
        || input.draft.overlay_scope != binding.overlay_scope
        || input.draft.evidence_ids != binding.evidence_ids
    {
        return Err(CuratorRepositoryError::InvalidInput);
    }
    Ok(())
}

fn invalidate_previous(
    transaction: &rusqlite::Transaction<'_>,
    input: &DraftPersistence<'_>,
) -> Result<(), CuratorRepositoryError> {
    transaction.execute("UPDATE evolution_curator_previews SET invalidated_at_ms=?1 WHERE candidate_id=?2 AND invalidated_at_ms IS NULL",
        params![input.occurred_at_ms,input.draft.candidate_id]).map_err(|_| CuratorRepositoryError::Storage)?;
    transaction.execute("UPDATE evolution_curator_draft_assessments SET invalidated_at_ms=?1 WHERE candidate_id=?2 AND invalidated_at_ms IS NULL",
        params![input.occurred_at_ms,input.draft.candidate_id]).map_err(|_| CuratorRepositoryError::Storage)?;
    Ok(())
}

fn valid_reason(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}
fn map_repository_error(error: CuratorRepositoryError) -> CuratorDraftStoreError {
    match error {
        CuratorRepositoryError::NotFound => CuratorDraftStoreError::NotFound,
        CuratorRepositoryError::Conflict(_) => CuratorDraftStoreError::Conflict,
        CuratorRepositoryError::InvalidInput
        | CuratorRepositoryError::Transition(_)
        | CuratorRepositoryError::UnsafeDocument(_) => CuratorDraftStoreError::InvalidInput,
        CuratorRepositoryError::Storage => CuratorDraftStoreError::Storage,
    }
}
