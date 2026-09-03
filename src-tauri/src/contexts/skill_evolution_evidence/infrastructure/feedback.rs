use super::{
    create_authorization, current_authorization, revoke_authorizations,
    AuthorizedCorrectionGuidance, EvidenceRepositoryError, ReusableGuidanceAuthorizationSummary,
    SqliteEvolutionEvidenceRepository,
};
use crate::contexts::skill_evolution_evidence::domain::{
    canonical_workspace_scope, EvidenceSanitizer, FeedbackState, TaskFingerprintBuilder,
};
use rusqlite::{params_from_iter, TransactionBehavior};
use std::collections::BTreeMap;
use thiserror::Error;

use super::feedback_storage::{
    current_feedback, feedback_envelope, latest_revision, load_message_source,
    parse_feedback_state, persist_feedback_signal, record_feedback_event, supersede_previous,
    update_current, validate_request,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SaveFeedbackRequest {
    pub(crate) message_id: String,
    pub(crate) expected_revision: u64,
    pub(crate) state: Option<FeedbackState>,
    pub(crate) correction_note: Option<String>,
    pub(crate) authorize_reusable_guidance: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SavedFeedback {
    pub(crate) message_id: String,
    pub(crate) revision: u64,
    pub(crate) state: Option<FeedbackState>,
    pub(crate) sanitized_note: Option<String>,
    pub(crate) reusable_guidance_authorization: Option<ReusableGuidanceAuthorizationSummary>,
    pub(crate) workspace_id: Option<String>,
    pub(crate) authorization_event_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredFeedbackSummary {
    pub(crate) state: Option<FeedbackState>,
    pub(crate) revision: u64,
    pub(crate) sanitized_note: Option<String>,
    pub(crate) reusable_guidance_authorization: Option<ReusableGuidanceAuthorizationSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RevokeReusableGuidanceAuthorizationRequest {
    pub(crate) message_id: String,
    pub(crate) expected_feedback_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RevokedReusableGuidanceAuthorization {
    pub(crate) message_id: String,
    pub(crate) feedback_revision: u64,
    pub(crate) workspace_id: Option<String>,
    pub(crate) authorization_event_id: String,
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
    pub(crate) fn authorized_correction_guidance(
        &self,
        authorization_id: &str,
    ) -> Result<Option<AuthorizedCorrectionGuidance>, FeedbackTransitionError> {
        let connection = self
            .database
            .connection()
            .map_err(|_| FeedbackTransitionError::Storage)?;
        super::authorized_correction_guidance(&connection, authorization_id)
    }

    pub(crate) fn feedback_for_messages(
        &self,
        message_ids: &[String],
    ) -> Result<BTreeMap<String, StoredFeedbackSummary>, EvidenceRepositoryError> {
        const QUERY_CHUNK: usize = 400;
        let connection = self
            .database
            .connection()
            .map_err(|_| EvidenceRepositoryError::Storage)?;
        let mut summaries = BTreeMap::new();
        for chunk in message_ids.chunks(QUERY_CHUNK) {
            let placeholders = std::iter::repeat_n("?", chunk.len())
                .collect::<Vec<_>>()
                .join(",");
            let current_sql = format!(
                "SELECT message_id, feedback_state, feedback_revision, sanitized_note \
                 FROM evolution_feedback_current WHERE message_id IN ({placeholders})"
            );
            let mut statement = connection.prepare(&current_sql)?;
            let rows = statement.query_map(params_from_iter(chunk.iter()), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?;
            for row in rows {
                let (message_id, state, revision, sanitized_note) = row?;
                let state =
                    parse_feedback_state(&state).ok_or(EvidenceRepositoryError::CorruptFeedback)?;
                let revision = u64::try_from(revision).unwrap_or_default();
                let reusable_guidance_authorization =
                    current_authorization(&connection, &message_id, revision)
                        .map_err(|_| EvidenceRepositoryError::Storage)?;
                summaries.insert(
                    message_id,
                    StoredFeedbackSummary {
                        state: Some(state),
                        revision,
                        sanitized_note,
                        reusable_guidance_authorization,
                    },
                );
            }

            let revision_sql = format!(
                "SELECT message_id, MAX(feedback_revision) FROM evolution_feedback_events \
                 WHERE message_id IN ({placeholders}) GROUP BY message_id"
            );
            let mut statement = connection.prepare(&revision_sql)?;
            let rows = statement.query_map(params_from_iter(chunk.iter()), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })?;
            for row in rows {
                let (message_id, revision) = row?;
                if revision > 0 && !summaries.contains_key(&message_id) {
                    summaries.insert(
                        message_id,
                        StoredFeedbackSummary {
                            state: None,
                            revision: u64::try_from(revision).unwrap_or_default(),
                            sanitized_note: None,
                            reusable_guidance_authorization: None,
                        },
                    );
                }
            }
        }
        Ok(summaries)
    }

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
        let workspace_id = source
            .workspace
            .as_deref()
            .map(|value| canonical_workspace_scope(installation_key, value))
            .transpose()
            .map_err(|_| FeedbackTransitionError::Storage)?;
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
        let revoked_authorization_ids =
            revoke_authorizations(&transaction, &request.message_id, None)?;
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
        let reusable_guidance_authorization = if request.authorize_reusable_guidance {
            Some(create_authorization(
                &transaction,
                &request.message_id,
                revision,
                sanitized_note
                    .as_deref()
                    .ok_or(FeedbackTransitionError::InvalidInput)?,
            )?)
        } else {
            None
        };
        transaction
            .commit()
            .map_err(|_| FeedbackTransitionError::Storage)?;
        let _ = self.rebuild_dirty_seeds();
        let authorization_event_id = reusable_guidance_authorization
            .as_ref()
            .map(|authorization| format!("{}:granted", authorization.authorization_id))
            .or_else(|| {
                revoked_authorization_ids
                    .first()
                    .map(|authorization_id| format!("{authorization_id}:revoked"))
            });
        Ok(SavedFeedback {
            message_id: request.message_id.clone(),
            revision,
            state: request.state,
            sanitized_note,
            reusable_guidance_authorization,
            workspace_id,
            authorization_event_id,
        })
    }

    pub(crate) fn revoke_reusable_guidance_authorization(
        &self,
        request: &RevokeReusableGuidanceAuthorizationRequest,
        installation_key: &[u8],
    ) -> Result<RevokedReusableGuidanceAuthorization, FeedbackTransitionError> {
        if request.message_id.trim().is_empty() {
            return Err(FeedbackTransitionError::InvalidInput);
        }
        let mut connection = self
            .database
            .connection()
            .map_err(|_| FeedbackTransitionError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| FeedbackTransitionError::Storage)?;
        let source = load_message_source(&transaction, &request.message_id)?;
        let current_revision = latest_revision(&transaction, &request.message_id)?;
        if current_revision != request.expected_feedback_revision {
            return Err(FeedbackTransitionError::Conflict { current_revision });
        }
        let current = current_feedback(&transaction, &request.message_id)?
            .ok_or(FeedbackTransitionError::InvalidInput)?;
        let revoked_ids =
            revoke_authorizations(&transaction, &request.message_id, Some(current_revision))?;
        if current.state != "corrected" || revoked_ids.len() != 1 {
            return Err(FeedbackTransitionError::InvalidInput);
        }
        let workspace_id = source
            .workspace
            .as_deref()
            .map(|value| canonical_workspace_scope(installation_key, value))
            .transpose()
            .map_err(|_| FeedbackTransitionError::Storage)?;
        transaction
            .commit()
            .map_err(|_| FeedbackTransitionError::Storage)?;
        Ok(RevokedReusableGuidanceAuthorization {
            message_id: request.message_id.clone(),
            feedback_revision: current_revision,
            workspace_id,
            authorization_event_id: format!("{}:revoked", revoked_ids[0]),
        })
    }
}
