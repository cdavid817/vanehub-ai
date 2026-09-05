use rusqlite::{params_from_iter, OptionalExtension};

use super::query_builder::build_query;
use super::{ActivityProjectionRepositoryError, SqliteActivityProjectionRepository};
use crate::contexts::skill_evolution_system_activity::domain::*;

impl SqliteActivityProjectionRepository<'_> {
    pub(crate) fn query_timeline(
        &self,
        query: &ActivityTimelineQuery,
    ) -> Result<ActivityTimelineQueryResult, ActivityProjectionRepositoryError> {
        query
            .validate()
            .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
        let active_generation_id = self
            .connection
            .query_row(
                "SELECT active_generation_id FROM evolution_system_activity_sessions
                 WHERE session_id=?1",
                [&query.session_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(ActivityProjectionRepositoryError::InvalidInput)?;
        let before_sequence = if let Some(cursor) = &query.cursor {
            let (generation_id, sequence) = decode_activity_page_cursor(cursor)
                .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
            if generation_id != active_generation_id {
                return Ok(ActivityTimelineQueryResult::StaleGeneration {
                    requested_generation_id: generation_id,
                    active_generation_id,
                });
            }
            Some(sequence)
        } else {
            None
        };
        let (sql, values) = build_query(query, &active_generation_id, before_sequence)?;
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(params_from_iter(values), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        })?;
        let mut entries = rows
            .map(|row| {
                let (json, sequence, reason) = row?;
                let envelope = serde_json::from_str(&json)
                    .map_err(|_| ActivityProjectionRepositoryError::Storage)?;
                let sequence = u64::try_from(sequence)
                    .map_err(|_| ActivityProjectionRepositoryError::Storage)?;
                let detail_unavailable_reason = reason.map(parse_reason_code).transpose()?;
                Ok(ActivityTimelineEntry {
                    sequence,
                    envelope,
                    detail_unavailable_reason,
                })
            })
            .collect::<Result<Vec<_>, ActivityProjectionRepositoryError>>()?;
        let complete = entries.len() <= usize::from(query.page_size);
        if !complete {
            entries.truncate(usize::from(query.page_size));
        }
        let next_cursor = if complete {
            None
        } else {
            entries
                .last()
                .map(|entry| {
                    encode_activity_page_cursor(&active_generation_id, entry.sequence)
                        .map_err(invalid)
                })
                .transpose()?
        };
        Ok(ActivityTimelineQueryResult::Page(ActivityTimelinePage {
            active_generation_id,
            entries,
            next_cursor,
            complete,
        }))
    }
}

fn parse_reason_code(
    value: String,
) -> Result<ActivityReasonCode, ActivityProjectionRepositoryError> {
    serde_json::from_value(serde_json::Value::String(value))
        .map_err(|_| ActivityProjectionRepositoryError::Storage)
}

fn invalid(_: ActivityEnvelopeError) -> ActivityProjectionRepositoryError {
    ActivityProjectionRepositoryError::InvalidInput
}
