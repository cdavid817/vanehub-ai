use rusqlite::{params, OptionalExtension, Transaction};

use super::{ActivityProjectionRepositoryError, SqliteActivityProjectionRepository};
use crate::contexts::skill_evolution_system_activity::domain::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TimelineDeliveryOutcome {
    pub(crate) session_id: String,
    pub(crate) generation_id: String,
    pub(crate) sequence: u64,
    pub(crate) session_created: bool,
    pub(crate) item_created: bool,
}

impl SqliteActivityProjectionRepository<'_> {
    pub(crate) fn deliver_timeline(
        &self,
        event_id: &str,
        projected_at_ms: i64,
    ) -> Result<TimelineDeliveryOutcome, ActivityProjectionRepositoryError> {
        if projected_at_ms < 0 || sanitize_text(event_id, "timeline.event_id", 160).is_err() {
            return Err(ActivityProjectionRepositoryError::InvalidInput);
        }
        let transaction = self.connection.unchecked_transaction()?;
        let envelope = load_envelope(&transaction, event_id)?;
        envelope
            .validate()
            .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
        let session_id = stable_system_activity_session_id(
            ActivityKind::SkillEvolution,
            envelope.scope_kind,
            &envelope.canonical_scope_id,
        )
        .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
        let generation_id =
            stable_activity_generation_id(&session_id, envelope.projection_policy_version);
        let session_created = ensure_session(
            &transaction,
            &session_id,
            &generation_id,
            &envelope,
            projected_at_ms,
        )?;
        ensure_default_preferences(&transaction, &envelope, projected_at_ms)?;
        let item_id = stable_activity_item_id(&session_id, &generation_id, event_id);
        let existing_sequence = transaction
            .query_row(
                "SELECT sequence FROM evolution_activity_items WHERE item_id=?1",
                [&item_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        let (sequence, item_created) = if let Some(sequence) = existing_sequence {
            (from_i64(sequence)?, false)
        } else {
            validate_supersession(&transaction, &session_id, &generation_id, &envelope)?;
            let sequence = next_sequence(
                &transaction,
                &session_id,
                envelope.committed_at_ms,
                projected_at_ms,
            )?;
            transaction.execute(
                "INSERT INTO evolution_activity_items
                 (item_id,session_id,generation_id,sequence,event_id,supersedes_event_id,created_at_ms)
                 VALUES (?1,?2,?3,?4,?5,?6,?7)",
                params![
                    item_id,
                    session_id,
                    generation_id,
                    to_i64(sequence)?,
                    envelope.event_id,
                    envelope.supersedes_event_id,
                    projected_at_ms,
                ],
            )?;
            (sequence, true)
        };
        transaction.commit()?;
        Ok(TimelineDeliveryOutcome {
            session_id,
            generation_id,
            sequence,
            session_created,
            item_created,
        })
    }
}

fn load_envelope(
    transaction: &Transaction<'_>,
    event_id: &str,
) -> Result<EvolutionActivityEnvelopeV1, ActivityProjectionRepositoryError> {
    let json = transaction
        .query_row(
            "SELECT envelope_json FROM evolution_activity_envelopes WHERE event_id=?1",
            [event_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .ok_or(ActivityProjectionRepositoryError::InvalidInput)?;
    serde_json::from_str(&json).map_err(|_| ActivityProjectionRepositoryError::Storage)
}

fn ensure_session(
    transaction: &Transaction<'_>,
    session_id: &str,
    generation_id: &str,
    envelope: &EvolutionActivityEnvelopeV1,
    projected_at_ms: i64,
) -> Result<bool, ActivityProjectionRepositoryError> {
    let display_identity = envelope
        .safe_identities
        .iter()
        .find(|identity| identity.kind == ActivitySafeIdentityKind::Workspace)
        .map(|identity| identity.value.as_str());
    let created = transaction.execute(
        "INSERT OR IGNORE INTO evolution_system_activity_sessions
         (session_id,schema_version,activity_kind,scope_kind,canonical_scope_id,
          safe_display_identity,active_generation_id,last_sequence,unread_count,attention_kind,
          preference_revision,created_at_ms,first_activity_at_ms,last_activity_at_ms,last_projected_at_ms)
         VALUES (?1,1,'skill_evolution',?2,?3,?4,?5,0,0,'none',1,?6,?7,?7,?6)",
        params![
            session_id,
            enum_text(envelope.scope_kind)?,
            envelope.canonical_scope_id,
            display_identity,
            generation_id,
            projected_at_ms,
            envelope.committed_at_ms,
        ],
    )?;
    Ok(created == 1)
}

fn ensure_default_preferences(
    transaction: &Transaction<'_>,
    envelope: &EvolutionActivityEnvelopeV1,
    projected_at_ms: i64,
) -> Result<(), ActivityProjectionRepositoryError> {
    transaction.execute(
        "INSERT OR IGNORE INTO evolution_activity_preferences
         (scope_kind,canonical_scope_id,updated_at_ms) VALUES (?1,?2,?3)",
        params![
            enum_text(envelope.scope_kind)?,
            envelope.canonical_scope_id,
            projected_at_ms
        ],
    )?;
    Ok(())
}

fn next_sequence(
    transaction: &Transaction<'_>,
    session_id: &str,
    activity_at_ms: i64,
    projected_at_ms: i64,
) -> Result<u64, ActivityProjectionRepositoryError> {
    let changed = transaction.execute(
        "UPDATE evolution_system_activity_sessions
         SET last_sequence=last_sequence+1,last_activity_at_ms=MAX(last_activity_at_ms,?2),
             last_projected_at_ms=MAX(last_projected_at_ms,?3)
         WHERE session_id=?1",
        params![session_id, activity_at_ms, projected_at_ms],
    )?;
    if changed != 1 {
        return Err(ActivityProjectionRepositoryError::Conflict);
    }
    let sequence = transaction.query_row(
        "SELECT last_sequence FROM evolution_system_activity_sessions WHERE session_id=?1",
        [session_id],
        |row| row.get::<_, i64>(0),
    )?;
    from_i64(sequence)
}

fn validate_supersession(
    transaction: &Transaction<'_>,
    session_id: &str,
    generation_id: &str,
    envelope: &EvolutionActivityEnvelopeV1,
) -> Result<(), ActivityProjectionRepositoryError> {
    let Some(prior_event_id) = &envelope.supersedes_event_id else {
        return Ok(());
    };
    let exists = transaction.query_row(
        "SELECT EXISTS(SELECT 1 FROM evolution_activity_items
         WHERE session_id=?1 AND generation_id=?2 AND event_id=?3)",
        params![session_id, generation_id, prior_event_id],
        |row| row.get::<_, bool>(0),
    )?;
    if exists {
        Ok(())
    } else {
        Err(ActivityProjectionRepositoryError::InvalidInput)
    }
}

fn enum_text<T: serde::Serialize>(value: T) -> Result<String, ActivityProjectionRepositoryError> {
    serde_json::to_value(value)
        .map_err(|_| ActivityProjectionRepositoryError::Storage)?
        .as_str()
        .map(str::to_owned)
        .ok_or(ActivityProjectionRepositoryError::Storage)
}

fn to_i64(value: u64) -> Result<i64, ActivityProjectionRepositoryError> {
    i64::try_from(value).map_err(|_| ActivityProjectionRepositoryError::InvalidInput)
}

fn from_i64(value: i64) -> Result<u64, ActivityProjectionRepositoryError> {
    u64::try_from(value).map_err(|_| ActivityProjectionRepositoryError::Storage)
}
