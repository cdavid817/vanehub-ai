use serde_json::{json, Value};

use crate::contexts::skill_evolution_system_activity::{domain::*, infrastructure::*};
use crate::platform::database::NativeDatabase;

/// Read-only service surface for Skill Evolution system activity. Every method projects committed
/// state or adjusts presentation-side read/notification state; nothing here can execute an
/// evolution action, and interactive-session commands must refuse these session ids entirely.
pub(crate) struct SkillEvolutionSystemActivityApi {
    pub(super) database: NativeDatabase,
}

/// Interactive session commands call this to refuse system activity sessions before touching any
/// Agent, provider, or terminal state.
pub(crate) fn is_system_activity_session_id(session_id: &str) -> bool {
    session_id.starts_with("system-activity-v1-")
}

impl SkillEvolutionSystemActivityApi {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn list_sessions(&self) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let mut statement = connection
            .prepare(
                "SELECT s.session_id,s.scope_kind,s.canonical_scope_id,s.safe_display_identity,
                        s.active_generation_id,s.last_sequence,s.unread_count,s.attention_kind,
                        s.first_activity_at_ms,s.last_activity_at_ms,
                        COALESCE(p.visible,1)
                 FROM evolution_system_activity_sessions s
                 LEFT JOIN evolution_activity_preferences p
                   ON p.scope_kind=s.scope_kind AND p.canonical_scope_id=s.canonical_scope_id
                 ORDER BY s.last_activity_at_ms DESC,s.session_id",
            )
            .map_err(|_| storage())?;
        let rows = statement
            .query_map([], |row| {
                Ok(json!({
                    "sessionId": row.get::<_, String>(0)?,
                    "kind": "system-activity",
                    "scopeKind": row.get::<_, String>(1)?,
                    "canonicalScopeId": row.get::<_, String>(2)?,
                    "safeDisplayIdentity": row.get::<_, Option<String>>(3)?,
                    "activeGenerationId": row.get::<_, String>(4)?,
                    "lastSequence": row.get::<_, i64>(5)?,
                    "unreadCount": row.get::<_, i64>(6)?,
                    "attentionKind": row.get::<_, String>(7)?,
                    "firstActivityAtMs": row.get::<_, i64>(8)?,
                    "lastActivityAtMs": row.get::<_, i64>(9)?,
                    "visible": row.get::<_, i64>(10)? != 0,
                }))
            })
            .map_err(|_| storage())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| storage())?;
        Ok(json!({ "sessions": rows }))
    }

    pub(crate) fn read_state(&self, session_id: &str, now_ms: i64) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        let state = repository
            .project_unread(session_id, LOCAL_ACTIVITY_USER_ID, now_ms)
            .map_err(map_error)?;
        to_value(&state)
    }

    pub(crate) fn advance_read_cursor(
        &self,
        session_id: &str,
        through_sequence: u64,
        expected_revision: u64,
        now_ms: i64,
    ) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        let state = repository
            .advance_read_cursor(
                session_id,
                LOCAL_ACTIVITY_USER_ID,
                through_sequence,
                expected_revision,
                now_ms,
            )
            .map_err(map_error)?;
        to_value(&state)
    }

    pub(crate) fn mark_unread(
        &self,
        session_id: &str,
        from_sequence: u64,
        expected_revision: u64,
        now_ms: i64,
    ) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        let state = repository
            .mark_unread(
                session_id,
                LOCAL_ACTIVITY_USER_ID,
                from_sequence,
                expected_revision,
                now_ms,
            )
            .map_err(map_error)?;
        to_value(&state)
    }

    pub(crate) fn preferences(
        &self,
        scope_kind: &str,
        canonical_scope_id: &str,
    ) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        let preferences = repository
            .preferences(parse_scope(scope_kind)?, canonical_scope_id)
            .map_err(map_error)?;
        match preferences {
            Some(preferences) => to_value(&preferences),
            None => Ok(Value::Null),
        }
    }

    pub(crate) fn update_preferences(
        &self,
        requested: Value,
        now_ms: i64,
    ) -> Result<Value, String> {
        let requested: EvolutionActivityPreferences =
            serde_json::from_value(requested).map_err(|_| invalid())?;
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        let outcome = repository
            .update_preferences(&requested, now_ms)
            .map_err(map_error)?;
        Ok(match outcome {
            ActivityPreferenceUpdateOutcome::Updated(preferences) => json!({
                "outcome": "updated",
                "preferences": to_value(&preferences)?,
            }),
            ActivityPreferenceUpdateOutcome::Conflict(preferences) => json!({
                "outcome": "conflict",
                "preferences": to_value(&preferences)?,
            }),
        })
    }

    pub(crate) fn open_notification(
        &self,
        request_id: &str,
        visible_sequence: u64,
        now_ms: i64,
    ) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        let outcome = repository
            .open_notification_after_visible(
                request_id,
                LOCAL_ACTIVITY_USER_ID,
                visible_sequence,
                now_ms,
            )
            .map_err(map_error)?;
        Ok(match outcome {
            ActivityNotificationOpenOutcome::PendingTimeline => json!({ "kind": "pending" }),
            ActivityNotificationOpenOutcome::Opened {
                session_id,
                sequence,
                read_state,
            } => json!({
                "kind": "opened",
                "sessionId": session_id,
                "sequence": sequence,
                "readState": serde_json::to_value(&read_state).map_err(|_| storage())?,
            }),
        })
    }

    pub(crate) fn dismiss_notification(
        &self,
        request_id: &str,
        now_ms: i64,
    ) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        repository
            .dismiss_notification(request_id, now_ms)
            .map_err(map_error)?;
        Ok(json!({ "dismissed": true }))
    }

    pub(crate) fn claim_due_digests(&self, now_ms: i64) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        let digests = repository
            .claim_due_digest_notifications(now_ms)
            .map_err(map_error)?;
        let rows: Vec<Value> = digests
            .into_iter()
            .map(|digest| {
                Ok(json!({
                    "scopeKind": enum_value(digest.scope_kind)?,
                    "canonicalScopeId": digest.canonical_scope_id,
                    "cadence": enum_value(digest.cadence)?,
                    "windowStartedAtMs": digest.window_started_at_ms,
                    "windowEndsAtMs": digest.window_ends_at_ms,
                    "countsByEventCode": digest.counts_by_event_code,
                    "highestSeverity": enum_value(digest.highest_severity)?,
                }))
            })
            .collect::<Result<_, String>>()?;
        Ok(json!({ "digests": rows }))
    }
}

pub(super) fn storage() -> String {
    "system-activity-storage-unavailable".into()
}

pub(super) fn invalid() -> String {
    "system-activity-invalid-input".into()
}

pub(super) fn map_error(error: ActivityProjectionRepositoryError) -> String {
    match error {
        ActivityProjectionRepositoryError::InvalidInput => invalid(),
        ActivityProjectionRepositoryError::Conflict
        | ActivityProjectionRepositoryError::LeaseHeld => "system-activity-conflict".into(),
        ActivityProjectionRepositoryError::ReceiptCollision => {
            "system-activity-receipt-collision".into()
        }
        ActivityProjectionRepositoryError::Cancelled => "system-activity-cancelled".into(),
        ActivityProjectionRepositoryError::Storage => storage(),
    }
}

pub(super) fn to_value(value: &impl serde::Serialize) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|_| storage())
}

pub(super) fn enum_value(value: impl serde::Serialize) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|_| storage())
}

pub(super) fn parse_scope(scope_kind: &str) -> Result<ActivityScopeKind, String> {
    serde_json::from_value(Value::String(scope_kind.into())).map_err(|_| invalid())
}
