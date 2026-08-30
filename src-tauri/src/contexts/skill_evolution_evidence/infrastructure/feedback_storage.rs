use chrono::Utc;
use rusqlite::{params, OptionalExtension, Transaction};
use uuid::Uuid;

use super::{sqlite_repository::persist_transaction, FeedbackTransitionError, SaveFeedbackRequest};
use crate::contexts::skill_evolution_evidence::domain::{
    canonical_workspace_scope, extract_registered_signals, EnvelopeCommon, EvidenceSourceEnvelope,
    FeedbackState, SourceFidelity, TaskFingerprintBuilder, EVIDENCE_ENVELOPE_SCHEMA_V1,
    EVIDENCE_SANITIZER_V1,
};

pub(super) fn parse_feedback_state(value: &str) -> Option<FeedbackState> {
    match value {
        "helpful" => Some(FeedbackState::Helpful),
        "unhelpful" => Some(FeedbackState::Unhelpful),
        "corrected" => Some(FeedbackState::Corrected),
        _ => None,
    }
}

pub(super) struct MessageSource {
    session_id: String,
    agent_id: String,
    run_id: Option<String>,
    pub(super) workspace: Option<String>,
}

pub(super) struct CurrentFeedback {
    pub(super) state: String,
    signal_id: Option<String>,
}

pub(super) fn validate_request(
    request: &SaveFeedbackRequest,
) -> Result<(), FeedbackTransitionError> {
    let has_note = request
        .correction_note
        .as_ref()
        .is_some_and(|note| !note.trim().is_empty());
    match request.state {
        Some(FeedbackState::Corrected) if !has_note => Err(FeedbackTransitionError::InvalidInput),
        Some(FeedbackState::Helpful | FeedbackState::Unhelpful)
            if request.correction_note.is_some() || request.authorize_reusable_guidance =>
        {
            Err(FeedbackTransitionError::InvalidInput)
        }
        None if request.correction_note.is_some() || request.authorize_reusable_guidance => {
            Err(FeedbackTransitionError::InvalidInput)
        }
        _ => Ok(()),
    }
}

pub(super) fn load_message_source(
    transaction: &Transaction<'_>,
    message_id: &str,
) -> Result<MessageSource, FeedbackTransitionError> {
    transaction
        .query_row(
            r#"SELECT messages.session_id, sessions.agent_id, messages.execution_run_id,
                      COALESCE(sessions.worktree_path, sessions.project_path, sessions.folder)
               FROM messages JOIN sessions ON sessions.id = messages.session_id
               WHERE messages.id = ?1 AND messages.role = 'assistant'
                 AND messages.status = 'completed'"#,
            [message_id],
            |row| {
                Ok(MessageSource {
                    session_id: row.get(0)?,
                    agent_id: row.get(1)?,
                    run_id: row.get(2)?,
                    workspace: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(|_| FeedbackTransitionError::Storage)?
        .ok_or(FeedbackTransitionError::MessageNotEligible)
}

pub(super) fn current_feedback(
    transaction: &Transaction<'_>,
    message_id: &str,
) -> Result<Option<CurrentFeedback>, FeedbackTransitionError> {
    transaction
        .query_row(
            "SELECT feedback_state, active_signal_id FROM evolution_feedback_current WHERE message_id = ?1",
            [message_id],
            |row| Ok(CurrentFeedback { state: row.get(0)?, signal_id: row.get(1)? }),
        )
        .optional()
        .map_err(|_| FeedbackTransitionError::Storage)
}

pub(super) fn latest_revision(
    transaction: &Transaction<'_>,
    message_id: &str,
) -> Result<u64, FeedbackTransitionError> {
    let revision: i64 = transaction
        .query_row(
            "SELECT COALESCE(MAX(feedback_revision), 0) FROM evolution_feedback_events WHERE message_id = ?1",
            [message_id],
            |row| row.get(0),
        )
        .map_err(|_| FeedbackTransitionError::Storage)?;
    u64::try_from(revision).map_err(|_| FeedbackTransitionError::Storage)
}

pub(super) fn feedback_envelope(
    request: &SaveFeedbackRequest,
    revision: u64,
    source: MessageSource,
    installation_key: &[u8],
) -> Result<EvidenceSourceEnvelope, FeedbackTransitionError> {
    let workspace = source
        .workspace
        .as_deref()
        .map(|value| canonical_workspace_scope(installation_key, value))
        .transpose()
        .map_err(|_| FeedbackTransitionError::Storage)?;
    Ok(EvidenceSourceEnvelope::ExplicitFeedback {
        schema_version: EVIDENCE_ENVELOPE_SCHEMA_V1,
        common: EnvelopeCommon {
            source_event_id: format!("feedback:{}:{revision}", request.message_id),
            occurred_at: Utc::now().to_rfc3339(),
            stable_agent_id: Some(source.agent_id),
            session_id: Some(source.session_id),
            message_id: Some(request.message_id.clone()),
            run_id: source.run_id,
            attempt_id: None,
            workspace,
            fidelity: SourceFidelity::Native,
            observed_skill_revisions: Vec::new(),
        },
        feedback: request.state.unwrap_or(FeedbackState::Helpful),
        feedback_revision: revision,
        correction_note: request.correction_note.clone(),
    })
}

pub(super) fn persist_feedback_signal(
    transaction: &Transaction<'_>,
    envelope: &EvidenceSourceEnvelope,
    sanitized: Option<&crate::contexts::skill_evolution_evidence::domain::SanitizationResult>,
    fingerprints: &TaskFingerprintBuilder,
) -> Result<Option<String>, FeedbackTransitionError> {
    extract_registered_signals(envelope, sanitized)
        .into_iter()
        .next()
        .map(|signal| {
            persist_transaction(transaction, &signal, fingerprints)
                .map(|outcome| outcome.signal_id().to_string())
                .map_err(Into::into)
        })
        .transpose()
}

pub(super) fn supersede_previous(
    transaction: &Transaction<'_>,
    current: Option<&CurrentFeedback>,
    replacement: Option<&str>,
) -> Result<(), FeedbackTransitionError> {
    if let Some(signal_id) = current.and_then(|value| value.signal_id.as_deref()) {
        transaction.execute("UPDATE evolution_signals SET lineage_status='superseded',superseded_by_signal_id=?2 WHERE signal_id=?1", params![signal_id, replacement]).map_err(|_| FeedbackTransitionError::Storage)?;
    }
    Ok(())
}

pub(super) fn record_feedback_event(
    transaction: &Transaction<'_>,
    request: &SaveFeedbackRequest,
    revision: u64,
    current: Option<&CurrentFeedback>,
    note: Option<&str>,
    signal_id: Option<&str>,
) -> Result<(), FeedbackTransitionError> {
    transaction.execute(
        "INSERT INTO evolution_feedback_events (feedback_event_id,message_id,feedback_revision,previous_state,next_state,sanitized_note,sanitizer_version,signal_id,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
        params![Uuid::new_v4().to_string(), request.message_id, revision as i64, current.map(|value| value.state.as_str()), request.state.map(super::storage_values::feedback), note, i64::from(EVIDENCE_SANITIZER_V1), signal_id],
    ).map_err(|_| FeedbackTransitionError::Storage)?;
    Ok(())
}

pub(super) fn update_current(
    transaction: &Transaction<'_>,
    request: &SaveFeedbackRequest,
    revision: u64,
    note: Option<&str>,
    signal_id: Option<&str>,
) -> Result<(), FeedbackTransitionError> {
    if let Some(state) = request.state {
        transaction.execute(
            "INSERT INTO evolution_feedback_current (message_id,feedback_state,feedback_revision,sanitized_note,sanitizer_version,active_signal_id,updated_at) VALUES (?1,?2,?3,?4,?5,?6,strftime('%Y-%m-%dT%H:%M:%fZ','now')) ON CONFLICT(message_id) DO UPDATE SET feedback_state=excluded.feedback_state,feedback_revision=excluded.feedback_revision,sanitized_note=excluded.sanitized_note,sanitizer_version=excluded.sanitizer_version,active_signal_id=excluded.active_signal_id,updated_at=excluded.updated_at",
            params![request.message_id, super::storage_values::feedback(state), revision as i64, note, i64::from(EVIDENCE_SANITIZER_V1), signal_id],
        ).map_err(|_| FeedbackTransitionError::Storage)?;
    } else {
        transaction
            .execute(
                "DELETE FROM evolution_feedback_current WHERE message_id=?1",
                [&request.message_id],
            )
            .map_err(|_| FeedbackTransitionError::Storage)?;
    }
    Ok(())
}
