use chrono::Utc;
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};
use thiserror::Error;
use uuid::Uuid;

use super::sqlite_repository::persist_transaction;
use super::{EvidenceRepositoryError, SqliteEvolutionEvidenceRepository};
use crate::contexts::skill_evolution_evidence::domain::{
    canonical_workspace_scope, extract_registered_signals, EnvelopeCommon, EvidenceSanitizer,
    EvidenceSourceEnvelope, FeedbackState, SourceFidelity, TaskFingerprintBuilder,
    EVIDENCE_ENVELOPE_SCHEMA_V1, EVIDENCE_SANITIZER_V1,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SaveFeedbackRequest {
    pub(crate) message_id: String,
    pub(crate) expected_revision: u64,
    pub(crate) state: Option<FeedbackState>,
    pub(crate) correction_note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SavedFeedback {
    pub(crate) message_id: String,
    pub(crate) revision: u64,
    pub(crate) state: Option<FeedbackState>,
    pub(crate) sanitized_note: Option<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub(crate) enum FeedbackTransitionError {
    #[error("feedback revision conflict")]
    Conflict { current_revision: u64 },
    #[error("message is not eligible for feedback")]
    MessageNotEligible,
    #[error("feedback input is invalid")]
    InvalidInput,
    #[error("feedback could not be saved")]
    Storage,
}

impl From<EvidenceRepositoryError> for FeedbackTransitionError {
    fn from(_: EvidenceRepositoryError) -> Self {
        Self::Storage
    }
}

impl SqliteEvolutionEvidenceRepository {
    pub(crate) fn save_feedback(
        &self,
        request: &SaveFeedbackRequest,
        installation_key: &[u8],
    ) -> Result<SavedFeedback, FeedbackTransitionError> {
        validate_request(request)?;
        let sanitizer = EvidenceSanitizer::new(installation_key)
            .map_err(|_| FeedbackTransitionError::InvalidInput)?;
        let fingerprints = TaskFingerprintBuilder::new(installation_key)
            .map_err(|_| FeedbackTransitionError::InvalidInput)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| FeedbackTransitionError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| FeedbackTransitionError::Storage)?;
        let source = load_message_source(&transaction, &request.message_id)?;
        let current = current_feedback(&transaction, &request.message_id)?;
        let current_revision = latest_revision(&transaction, &request.message_id)?;
        if current_revision != request.expected_revision {
            return Err(FeedbackTransitionError::Conflict { current_revision });
        }
        let revision = current_revision.saturating_add(1);
        let envelope = feedback_envelope(request, revision, source, installation_key)?;
        let sanitized = envelope
            .sanitized_registered_text(&sanitizer)
            .map_err(|_| FeedbackTransitionError::InvalidInput)?;
        let sanitized_note = sanitized.as_ref().map(|value| value.text().to_string());
        let signal_id = if request.state.is_some() {
            persist_feedback_signal(&transaction, &envelope, sanitized.as_ref(), &fingerprints)?
        } else {
            None
        };
        supersede_previous(&transaction, current.as_ref(), signal_id.as_deref())?;
        record_feedback_event(
            &transaction,
            request,
            revision,
            current.as_ref(),
            sanitized_note.as_deref(),
            signal_id.as_deref(),
        )?;
        update_current(
            &transaction,
            request,
            revision,
            sanitized_note.as_deref(),
            signal_id.as_deref(),
        )?;
        transaction
            .commit()
            .map_err(|_| FeedbackTransitionError::Storage)?;
        let _ = self.rebuild_dirty_seeds();
        Ok(SavedFeedback {
            message_id: request.message_id.clone(),
            revision,
            state: request.state,
            sanitized_note,
        })
    }
}

struct MessageSource {
    session_id: String,
    agent_id: String,
    run_id: Option<String>,
    workspace: Option<String>,
}

struct CurrentFeedback {
    state: String,
    signal_id: Option<String>,
}

fn validate_request(request: &SaveFeedbackRequest) -> Result<(), FeedbackTransitionError> {
    let has_note = request
        .correction_note
        .as_ref()
        .is_some_and(|note| !note.trim().is_empty());
    match request.state {
        Some(FeedbackState::Corrected) if !has_note => Err(FeedbackTransitionError::InvalidInput),
        Some(FeedbackState::Helpful | FeedbackState::Unhelpful)
            if request.correction_note.is_some() =>
        {
            Err(FeedbackTransitionError::InvalidInput)
        }
        None if request.correction_note.is_some() => Err(FeedbackTransitionError::InvalidInput),
        _ => Ok(()),
    }
}

fn load_message_source(
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

fn current_feedback(
    transaction: &Transaction<'_>,
    message_id: &str,
) -> Result<Option<CurrentFeedback>, FeedbackTransitionError> {
    transaction
        .query_row(
            "SELECT feedback_state, active_signal_id FROM evolution_feedback_current WHERE message_id = ?1",
            [message_id],
            |row| Ok(CurrentFeedback {
                state: row.get(0)?,
                signal_id: row.get(1)?,
            }),
        )
        .optional()
        .map_err(|_| FeedbackTransitionError::Storage)
}

fn latest_revision(
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
    Ok(u64::try_from(revision).unwrap_or_default())
}

fn feedback_envelope(
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

fn persist_feedback_signal(
    transaction: &Transaction<'_>,
    envelope: &EvidenceSourceEnvelope,
    sanitized: Option<&crate::contexts::skill_evolution_evidence::domain::SanitizationResult>,
    fingerprints: &TaskFingerprintBuilder,
) -> Result<Option<String>, FeedbackTransitionError> {
    let signal = extract_registered_signals(envelope, sanitized)
        .into_iter()
        .next();
    signal
        .map(|signal| {
            persist_transaction(transaction, &signal, fingerprints)
                .map(|outcome| outcome.signal_id().to_string())
                .map_err(Into::into)
        })
        .transpose()
}

fn supersede_previous(
    transaction: &Transaction<'_>,
    current: Option<&CurrentFeedback>,
    replacement: Option<&str>,
) -> Result<(), FeedbackTransitionError> {
    if let Some(signal_id) = current.and_then(|value| value.signal_id.as_deref()) {
        transaction
            .execute(
                "UPDATE evolution_signals SET lineage_status = 'superseded', superseded_by_signal_id = ?2 WHERE signal_id = ?1",
                params![signal_id, replacement],
            )
            .map_err(|_| FeedbackTransitionError::Storage)?;
    }
    Ok(())
}

fn record_feedback_event(
    transaction: &Transaction<'_>,
    request: &SaveFeedbackRequest,
    revision: u64,
    current: Option<&CurrentFeedback>,
    note: Option<&str>,
    signal_id: Option<&str>,
) -> Result<(), FeedbackTransitionError> {
    transaction.execute(
        "INSERT INTO evolution_feedback_events (feedback_event_id, message_id, feedback_revision, previous_state, next_state, sanitized_note, sanitizer_version, signal_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
        params![Uuid::new_v4().to_string(), request.message_id, revision as i64, current.map(|value| value.state.as_str()), request.state.map(super::storage_values::feedback), note, i64::from(EVIDENCE_SANITIZER_V1), signal_id],
    ).map_err(|_| FeedbackTransitionError::Storage)?;
    Ok(())
}

fn update_current(
    transaction: &Transaction<'_>,
    request: &SaveFeedbackRequest,
    revision: u64,
    note: Option<&str>,
    signal_id: Option<&str>,
) -> Result<(), FeedbackTransitionError> {
    if let Some(state) = request.state {
        transaction.execute(
            "INSERT INTO evolution_feedback_current (message_id, feedback_state, feedback_revision, sanitized_note, sanitizer_version, active_signal_id, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) ON CONFLICT(message_id) DO UPDATE SET feedback_state = excluded.feedback_state, feedback_revision = excluded.feedback_revision, sanitized_note = excluded.sanitized_note, sanitizer_version = excluded.sanitizer_version, active_signal_id = excluded.active_signal_id, updated_at = excluded.updated_at",
            params![request.message_id, super::storage_values::feedback(state), revision as i64, note, i64::from(EVIDENCE_SANITIZER_V1), signal_id],
        ).map_err(|_| FeedbackTransitionError::Storage)?;
    } else {
        transaction
            .execute(
                "DELETE FROM evolution_feedback_current WHERE message_id = ?1",
                [&request.message_id],
            )
            .map_err(|_| FeedbackTransitionError::Storage)?;
    }
    Ok(())
}
