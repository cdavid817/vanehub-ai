use rusqlite::{params, OptionalExtension};

use super::{ActivityProjectionRepositoryError, SqliteActivityProjectionRepository};
use crate::contexts::skill_evolution_system_activity::domain::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DashboardDeliveryOutcome {
    pub(crate) kind: Option<ActivityDashboardKind>,
    pub(crate) materialized: bool,
}

impl SqliteActivityProjectionRepository<'_> {
    pub(crate) fn materialize_dashboard(
        &self,
        event_id: &str,
        updated_at_ms: i64,
    ) -> Result<DashboardDeliveryOutcome, ActivityProjectionRepositoryError> {
        if updated_at_ms < 0 || sanitize_text(event_id, "dashboard.event_id", 160).is_err() {
            return Err(ActivityProjectionRepositoryError::InvalidInput);
        }
        let envelope = self.load_dashboard_envelope(event_id)?;
        envelope
            .validate()
            .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
        let Some(state) = DashboardMaterializationV1::from_envelope(&envelope) else {
            return Ok(DashboardDeliveryOutcome {
                kind: None,
                materialized: false,
            });
        };
        let session_id = stable_system_activity_session_id(
            ActivityKind::SkillEvolution,
            envelope.scope_kind,
            &envelope.canonical_scope_id,
        )
        .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
        let generation_id =
            stable_activity_generation_id(&session_id, envelope.projection_policy_version);
        let state_json = serde_json::to_string(&state)
            .map_err(|_| ActivityProjectionRepositoryError::Storage)?;
        let changed = self.connection.execute(
            "INSERT INTO evolution_activity_dashboard_state
             (scope_kind,canonical_scope_id,generation_id,materialization_kind,state_json,
              last_event_id,updated_at_ms,revision) VALUES (?1,?2,?3,?4,?5,?6,?7,1)
             ON CONFLICT(scope_kind,canonical_scope_id,generation_id,materialization_kind)
             DO UPDATE SET state_json=excluded.state_json,last_event_id=excluded.last_event_id,
               updated_at_ms=excluded.updated_at_ms,revision=revision+1
             WHERE (
               (SELECT committed_at_ms FROM evolution_activity_envelopes
                WHERE event_id=excluded.last_event_id),
               (SELECT source_sequence FROM evolution_activity_envelopes
                WHERE event_id=excluded.last_event_id),excluded.last_event_id
             ) > (
               (SELECT committed_at_ms FROM evolution_activity_envelopes
                WHERE event_id=evolution_activity_dashboard_state.last_event_id),
               (SELECT source_sequence FROM evolution_activity_envelopes
                WHERE event_id=evolution_activity_dashboard_state.last_event_id),
               evolution_activity_dashboard_state.last_event_id
             )",
            params![
                enum_text(envelope.scope_kind)?,
                envelope.canonical_scope_id,
                generation_id,
                state.kind.as_str(),
                state_json,
                state.event_id,
                updated_at_ms,
            ],
        )?;
        Ok(DashboardDeliveryOutcome {
            kind: Some(state.kind),
            materialized: changed == 1,
        })
    }

    fn load_dashboard_envelope(
        &self,
        event_id: &str,
    ) -> Result<EvolutionActivityEnvelopeV1, ActivityProjectionRepositoryError> {
        let json = self
            .connection
            .query_row(
                "SELECT envelope_json FROM evolution_activity_envelopes WHERE event_id=?1",
                [event_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(ActivityProjectionRepositoryError::InvalidInput)?;
        serde_json::from_str(&json).map_err(|_| ActivityProjectionRepositoryError::Storage)
    }
}

fn enum_text<T: serde::Serialize>(value: T) -> Result<String, ActivityProjectionRepositoryError> {
    serde_json::to_value(value)
        .map_err(|_| ActivityProjectionRepositoryError::Storage)?
        .as_str()
        .map(str::to_owned)
        .ok_or(ActivityProjectionRepositoryError::Storage)
}
