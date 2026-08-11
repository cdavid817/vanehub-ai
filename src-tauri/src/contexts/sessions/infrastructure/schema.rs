use crate::contexts::sessions::domain::{decode_seats, encode_seats};
use rusqlite::Connection;
use std::collections::HashMap;

pub(crate) fn apply_configuration_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    if !crate::platform::database::table_has_column(connection, "sessions", "chat_preferences")? {
        connection.execute("ALTER TABLE sessions ADD COLUMN chat_preferences TEXT", [])?;
    }
    Ok(())
}

pub(crate) fn apply_loop_ownership_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    for column in ["loop_run_id", "loop_iteration_id", "loop_role"] {
        if !crate::platform::database::table_has_column(connection, "sessions", column)? {
            connection.execute(
                &format!("ALTER TABLE sessions ADD COLUMN {column} TEXT"),
                [],
            )?;
        }
    }
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_loop_ownership ON sessions(loop_run_id, loop_iteration_id, loop_role)",
        [],
    )?;
    Ok(())
}

/// Seats are stored as a JSON list on the session rather than in a joined table: `SESSION_SELECT`
/// is the hot path for list, search, and get, and a join there would cost every read for a feature
/// most sessions do not use. Existing rows default to `[]`, which readers present as the one-seat
/// case built from `agent_id`.
/// Which seat spoke a message, so a thread can attribute its replies.
///
/// Nullable rather than defaulted: a user message has no seat, and neither do the messages of every
/// session that predates seats. A default of 0 would attribute all of them to the first seat.
pub(crate) fn apply_message_speaker_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    if !crate::platform::database::table_has_column(connection, "messages", "seat_index")? {
        connection.execute("ALTER TABLE messages ADD COLUMN seat_index INTEGER", [])?;
    }
    Ok(())
}

pub(crate) fn apply_session_seat_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    let mut statement = connection.prepare("PRAGMA table_info(sessions)")?;
    let existing: Vec<String> = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(Result::ok)
        .collect();
    if !existing.iter().any(|column| column == "seats") {
        connection.execute(
            "ALTER TABLE sessions ADD COLUMN seats TEXT NOT NULL DEFAULT '[]'",
            [],
        )?;
    }
    Ok(())
}

pub(crate) fn apply_stable_participant_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    if !crate::platform::database::table_has_column(connection, "messages", "speaker_seat_id")? {
        connection.execute("ALTER TABLE messages ADD COLUMN speaker_seat_id TEXT", [])?;
    }

    let sessions = {
        let mut statement = connection.prepare(
            "SELECT id, agent_id, created_at, seats FROM sessions ORDER BY created_at, id",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };

    let mut seats_by_session = HashMap::new();
    for (session_id, agent_id, created_at, stored) in sessions {
        let seats = decode_seats(&stored, &session_id, &agent_id, &created_at);
        connection.execute(
            "UPDATE sessions SET seats = ?1 WHERE id = ?2",
            rusqlite::params![encode_seats(&seats), session_id],
        )?;
        seats_by_session.insert(session_id, seats);
    }

    let legacy_messages = {
        let mut statement = connection.prepare(
            "SELECT id, session_id, seat_index FROM messages WHERE speaker_seat_id IS NULL AND seat_index IS NOT NULL",
        )?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };

    for (message_id, session_id, seat_index) in legacy_messages {
        let Some(index) = usize::try_from(seat_index).ok() else {
            continue;
        };
        let Some(seat_id) = seats_by_session
            .get(&session_id)
            .and_then(|seats| seats.get(index))
            .map(|seat| seat.seat_id.as_str())
        else {
            continue;
        };
        connection.execute(
            "UPDATE messages SET speaker_seat_id = ?1 WHERE id = ?2",
            rusqlite::params![seat_id, message_id],
        )?;
    }
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_messages_speaker_seat ON messages(session_id, speaker_seat_id, created_at)",
        [],
    )?;
    Ok(())
}
