use rusqlite::{params, OptionalExtension};

use super::{ActivityProjectionRepositoryError, SqliteActivityProjectionRepository};
use crate::contexts::skill_evolution_system_activity::domain::*;

pub(crate) const LOCAL_ACTIVITY_USER_ID: &str = "local";

impl SqliteActivityProjectionRepository<'_> {
    pub(crate) fn project_unread(
        &self,
        session_id: &str,
        user_id: &str,
        projected_at_ms: i64,
    ) -> Result<SystemActivityReadState, ActivityProjectionRepositoryError> {
        validate_read_input(session_id, user_id, projected_at_ms)?;
        self.ensure_read_state(session_id, user_id, projected_at_ms)?;
        self.refresh_unread_summary(session_id, user_id, projected_at_ms)?;
        self.read_state(session_id, user_id)?
            .ok_or(ActivityProjectionRepositoryError::Storage)
    }

    pub(crate) fn advance_read_cursor(
        &self,
        session_id: &str,
        user_id: &str,
        through_sequence: u64,
        expected_revision: u64,
        seen_at_ms: i64,
    ) -> Result<SystemActivityReadState, ActivityProjectionRepositoryError> {
        validate_read_input(session_id, user_id, seen_at_ms)?;
        if through_sequence > self.session_last_sequence(session_id)? {
            return Err(ActivityProjectionRepositoryError::InvalidInput);
        }
        let changed = if expected_revision == 0 {
            self.connection.execute(
                "INSERT OR IGNORE INTO evolution_activity_read_state
                 (session_id,user_id,highest_read_sequence,mark_unread_sequence,last_seen_at_ms,revision)
                 VALUES (?1,?2,?3,NULL,?4,1)",
                params![session_id, user_id, to_i64(through_sequence)?, seen_at_ms],
            )?
        } else {
            self.connection.execute(
                "UPDATE evolution_activity_read_state
                 SET highest_read_sequence=MAX(highest_read_sequence,?1),
                   mark_unread_sequence=CASE
                     WHEN mark_unread_sequence IS NOT NULL AND ?1>=mark_unread_sequence THEN NULL
                     ELSE mark_unread_sequence END,
                   last_seen_at_ms=MAX(last_seen_at_ms,?2),revision=revision+1
                 WHERE session_id=?3 AND user_id=?4 AND revision=?5",
                params![
                    to_i64(through_sequence)?,
                    seen_at_ms,
                    session_id,
                    user_id,
                    to_i64(expected_revision)?
                ],
            )?
        };
        if changed != 1 {
            return Err(ActivityProjectionRepositoryError::Conflict);
        }
        self.refresh_unread_summary(session_id, user_id, seen_at_ms)?;
        self.read_state(session_id, user_id)?
            .ok_or(ActivityProjectionRepositoryError::Storage)
    }

    pub(crate) fn mark_unread(
        &self,
        session_id: &str,
        user_id: &str,
        from_sequence: u64,
        expected_revision: u64,
        seen_at_ms: i64,
    ) -> Result<SystemActivityReadState, ActivityProjectionRepositoryError> {
        validate_read_input(session_id, user_id, seen_at_ms)?;
        let last_sequence = self.session_last_sequence(session_id)?;
        if from_sequence == 0 || from_sequence > last_sequence || expected_revision == 0 {
            return Err(ActivityProjectionRepositoryError::InvalidInput);
        }
        let changed = self.connection.execute(
            "UPDATE evolution_activity_read_state
             SET mark_unread_sequence=?1,last_seen_at_ms=MAX(last_seen_at_ms,?2),revision=revision+1
             WHERE session_id=?3 AND user_id=?4 AND revision=?5",
            params![
                to_i64(from_sequence)?,
                seen_at_ms,
                session_id,
                user_id,
                to_i64(expected_revision)?
            ],
        )?;
        if changed != 1 {
            return Err(ActivityProjectionRepositoryError::Conflict);
        }
        self.refresh_unread_summary(session_id, user_id, seen_at_ms)?;
        self.read_state(session_id, user_id)?
            .ok_or(ActivityProjectionRepositoryError::Storage)
    }

    pub(crate) fn read_state(
        &self,
        session_id: &str,
        user_id: &str,
    ) -> Result<Option<SystemActivityReadState>, ActivityProjectionRepositoryError> {
        self.connection
            .query_row(
                "SELECT session_id,user_id,highest_read_sequence,mark_unread_sequence,
                        last_seen_at_ms,revision
                 FROM evolution_activity_read_state WHERE session_id=?1 AND user_id=?2",
                params![session_id, user_id],
                |row| {
                    Ok(SystemActivityReadState {
                        session_id: row.get(0)?,
                        user_id: row.get(1)?,
                        highest_read_sequence: from_i64(row.get(2)?)?,
                        mark_unread_sequence: row
                            .get::<_, Option<i64>>(3)?
                            .map(from_i64)
                            .transpose()?,
                        last_seen_at_ms: row.get(4)?,
                        revision: from_i64(row.get(5)?)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    fn ensure_read_state(
        &self,
        session_id: &str,
        user_id: &str,
        seen_at_ms: i64,
    ) -> Result<(), ActivityProjectionRepositoryError> {
        self.connection.execute(
            "INSERT OR IGNORE INTO evolution_activity_read_state
             (session_id,user_id,highest_read_sequence,mark_unread_sequence,last_seen_at_ms,revision)
             VALUES (?1,?2,0,NULL,?3,1)",
            params![session_id, user_id, seen_at_ms],
        )?;
        Ok(())
    }

    fn session_last_sequence(
        &self,
        session_id: &str,
    ) -> Result<u64, ActivityProjectionRepositoryError> {
        let value = self
            .connection
            .query_row(
                "SELECT last_sequence FROM evolution_system_activity_sessions WHERE session_id=?1",
                [session_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()?
            .ok_or(ActivityProjectionRepositoryError::InvalidInput)?;
        from_i64(value).map_err(|_| ActivityProjectionRepositoryError::Storage)
    }

    fn refresh_unread_summary(
        &self,
        session_id: &str,
        user_id: &str,
        projected_at_ms: i64,
    ) -> Result<(), ActivityProjectionRepositoryError> {
        let state = self
            .read_state(session_id, user_id)?
            .ok_or(ActivityProjectionRepositoryError::Storage)?;
        let effective_read = state
            .mark_unread_sequence
            .map_or(state.highest_read_sequence, |sequence| {
                state.highest_read_sequence.min(sequence.saturating_sub(1))
            });
        let (count, attention_rank) = self.connection.query_row(
            "SELECT COUNT(*),COALESCE(MAX(CASE e.attention_kind
               WHEN 'security' THEN 7 WHEN 'integrity' THEN 6 WHEN 'breaker' THEN 5
               WHEN 'application_failure' THEN 4 WHEN 'regression' THEN 3
               WHEN 'review' THEN 2 ELSE 0 END),0)
             FROM evolution_activity_items i
             JOIN evolution_system_activity_sessions s ON s.session_id=i.session_id
             JOIN evolution_activity_envelopes e ON e.event_id=i.event_id
             WHERE i.session_id=?1 AND i.generation_id=s.active_generation_id AND i.sequence>?2",
            params![session_id, to_i64(effective_read)?],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;
        let changed = self.connection.execute(
            "UPDATE evolution_system_activity_sessions
             SET unread_count=?1,attention_kind=?2,last_projected_at_ms=MAX(last_projected_at_ms,?3)
             WHERE session_id=?4",
            params![
                count,
                attention_from_rank(attention_rank),
                projected_at_ms,
                session_id
            ],
        )?;
        if changed == 1 {
            Ok(())
        } else {
            Err(ActivityProjectionRepositoryError::Conflict)
        }
    }
}

fn validate_read_input(
    session_id: &str,
    user_id: &str,
    timestamp_ms: i64,
) -> Result<(), ActivityProjectionRepositoryError> {
    if timestamp_ms < 0
        || sanitize_text(session_id, "read.session_id", 160).is_err()
        || sanitize_text(user_id, "read.user_id", 160).is_err()
    {
        return Err(ActivityProjectionRepositoryError::InvalidInput);
    }
    Ok(())
}

fn attention_from_rank(rank: i64) -> &'static str {
    match rank {
        7 => "security",
        6 => "integrity",
        5 => "breaker",
        4 => "application_failure",
        3 => "regression",
        2 => "review",
        _ => "none",
    }
}

fn to_i64(value: u64) -> Result<i64, ActivityProjectionRepositoryError> {
    i64::try_from(value).map_err(|_| ActivityProjectionRepositoryError::InvalidInput)
}

fn from_i64(value: i64) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}
