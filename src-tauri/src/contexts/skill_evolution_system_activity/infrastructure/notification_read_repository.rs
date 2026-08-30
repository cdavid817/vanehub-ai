use rusqlite::{params, OptionalExtension, Transaction};

use super::retention_repository::refresh_session_summary;
use super::{ActivityProjectionRepositoryError, SqliteActivityProjectionRepository};
use crate::contexts::skill_evolution_system_activity::domain::*;

impl SqliteActivityProjectionRepository<'_> {
    pub(crate) fn dismiss_notification(
        &self,
        request_id: &str,
        updated_at_ms: i64,
    ) -> Result<(), ActivityProjectionRepositoryError> {
        validate_text_time(request_id, "notification.request_id", updated_at_ms)?;
        let changed = self.connection.execute(
            "UPDATE evolution_activity_notification_requests
             SET status='dismissed',updated_at_ms=MAX(updated_at_ms,?2)
             WHERE request_id=?1 AND status='pending'",
            params![request_id, updated_at_ms],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(ActivityProjectionRepositoryError::Conflict)
        }
    }

    pub(crate) fn open_notification_after_visible(
        &self,
        request_id: &str,
        user_id: &str,
        expected_read_revision: u64,
        seen_at_ms: i64,
    ) -> Result<ActivityNotificationOpenOutcome, ActivityProjectionRepositoryError> {
        validate_text_time(request_id, "notification.request_id", seen_at_ms)?;
        validate_text_time(user_id, "notification.user_id", seen_at_ms)?;
        let transaction = self.connection.unchecked_transaction()?;
        let visible = transaction
            .query_row(
                "SELECT i.session_id,i.sequence FROM evolution_activity_notification_requests n
                 JOIN evolution_activity_items i ON i.event_id=n.event_id
                 JOIN evolution_system_activity_sessions s ON s.session_id=i.session_id
                 WHERE n.request_id=?1 AND n.status!='dismissed'
                   AND i.generation_id=s.active_generation_id",
                [request_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?;
        let Some((session_id, sequence)) = visible else {
            return Ok(ActivityNotificationOpenOutcome::PendingTimeline);
        };
        let sequence =
            u64::try_from(sequence).map_err(|_| ActivityProjectionRepositoryError::Storage)?;
        let read_state = advance_visible_read(
            &transaction,
            &session_id,
            user_id,
            sequence,
            expected_read_revision,
            seen_at_ms,
        )?;
        refresh_session_summary(&transaction, &session_id)?;
        transaction.execute(
            "UPDATE evolution_activity_notification_requests
             SET status='opened',updated_at_ms=MAX(updated_at_ms,?2) WHERE request_id=?1",
            params![request_id, seen_at_ms],
        )?;
        transaction.commit()?;
        Ok(ActivityNotificationOpenOutcome::Opened {
            session_id,
            sequence,
            read_state,
        })
    }
}

fn advance_visible_read(
    transaction: &Transaction<'_>,
    session_id: &str,
    user_id: &str,
    sequence: u64,
    expected_revision: u64,
    seen_at_ms: i64,
) -> Result<SystemActivityReadState, ActivityProjectionRepositoryError> {
    transaction.execute(
        "INSERT OR IGNORE INTO evolution_activity_read_state
         (session_id,user_id,highest_read_sequence,last_seen_at_ms,revision)
         VALUES (?1,?2,0,?3,1)",
        params![session_id, user_id, seen_at_ms],
    )?;
    let current_revision = transaction.query_row(
        "SELECT revision FROM evolution_activity_read_state WHERE session_id=?1 AND user_id=?2",
        params![session_id, user_id],
        |row| row.get::<_, i64>(0),
    )?;
    if expected_revision != 0 && i64::try_from(expected_revision).ok() != Some(current_revision) {
        return Err(ActivityProjectionRepositoryError::Conflict);
    }
    let sequence =
        i64::try_from(sequence).map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
    transaction.execute(
        "UPDATE evolution_activity_read_state
         SET highest_read_sequence=MAX(highest_read_sequence,?1),
           mark_unread_sequence=CASE WHEN mark_unread_sequence<=?1 THEN NULL
             ELSE mark_unread_sequence END,
           last_seen_at_ms=MAX(last_seen_at_ms,?2),revision=revision+1
         WHERE session_id=?3 AND user_id=?4 AND revision=?5",
        params![sequence, seen_at_ms, session_id, user_id, current_revision],
    )?;
    load_read_state(transaction, session_id, user_id)
}

fn load_read_state(
    transaction: &Transaction<'_>,
    session_id: &str,
    user_id: &str,
) -> Result<SystemActivityReadState, ActivityProjectionRepositoryError> {
    transaction
        .query_row(
            "SELECT highest_read_sequence,mark_unread_sequence,last_seen_at_ms,revision
             FROM evolution_activity_read_state WHERE session_id=?1 AND user_id=?2",
            params![session_id, user_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, Option<i64>>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .map_err(Into::into)
        .and_then(|(highest, marked, seen, revision)| {
            Ok(SystemActivityReadState {
                session_id: session_id.into(),
                user_id: user_id.into(),
                highest_read_sequence: from_i64(highest)?,
                mark_unread_sequence: marked.map(from_i64).transpose()?,
                last_seen_at_ms: seen,
                revision: from_i64(revision)?,
            })
        })
}

fn from_i64(value: i64) -> Result<u64, ActivityProjectionRepositoryError> {
    u64::try_from(value).map_err(|_| ActivityProjectionRepositoryError::Storage)
}

fn validate_text_time(
    value: &str,
    field: &'static str,
    timestamp: i64,
) -> Result<(), ActivityProjectionRepositoryError> {
    if timestamp < 0 || sanitize_text(value, field, 200).is_err() {
        Err(ActivityProjectionRepositoryError::InvalidInput)
    } else {
        Ok(())
    }
}
