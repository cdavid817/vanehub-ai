use rusqlite::OptionalExtension;

use super::{
    ActivityProjectionRepositoryError, SqliteActivityProjectionRepository, LOCAL_ACTIVITY_USER_ID,
};
use crate::contexts::skill_evolution_system_activity::{application::*, domain::*};

pub(crate) struct SqliteActivityTargetDeliveryAdapter<'ports, 'connection> {
    repository: &'ports SqliteActivityProjectionRepository<'connection>,
    notification: &'ports dyn ActivityNotificationDeliveryPort,
}

impl<'ports, 'connection> SqliteActivityTargetDeliveryAdapter<'ports, 'connection> {
    pub(crate) fn new(
        repository: &'ports SqliteActivityProjectionRepository<'connection>,
        notification: &'ports dyn ActivityNotificationDeliveryPort,
    ) -> Self {
        Self {
            repository,
            notification,
        }
    }
}

impl ActivityTargetDeliveryPort for SqliteActivityTargetDeliveryAdapter<'_, '_> {
    fn load_envelope(
        &self,
        event_id: &str,
    ) -> Result<EvolutionActivityEnvelopeV1, ActivityTargetProjectionError> {
        let json = self
            .repository
            .connection
            .query_row(
                "SELECT envelope_json FROM evolution_activity_envelopes WHERE event_id=?1",
                [event_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| ActivityTargetProjectionError::Storage)?
            .ok_or(ActivityTargetProjectionError::InvalidInput)?;
        serde_json::from_str(&json).map_err(|_| ActivityTargetProjectionError::InvalidEnvelope)
    }

    fn receipt(
        &self,
        event_id: &str,
        target_kind: ActivityTargetKind,
        target_scope: &str,
    ) -> Result<Option<ActivityTargetReceipt>, ActivityTargetProjectionError> {
        self.repository
            .target_receipt(event_id, target_kind, target_scope)
            .map_err(map_error)
    }

    fn policy_allows(
        &self,
        target_kind: ActivityTargetKind,
        envelope: &EvolutionActivityEnvelopeV1,
    ) -> Result<bool, ActivityTargetProjectionError> {
        if target_kind == ActivityTargetKind::Notification {
            return self.notification.notification_is_eligible(envelope);
        }
        if !matches!(
            target_kind,
            ActivityTargetKind::SystemTimeline | ActivityTargetKind::UnreadState
        ) {
            return Ok(true);
        }
        let minimum = self
            .repository
            .preferences(envelope.scope_kind, &envelope.canonical_scope_id)
            .map_err(map_error)?
            .map(|preferences| preferences.minimum_timeline_severity)
            .unwrap_or(ActivitySeverity::Info);
        Ok(timeline_policy_allows(envelope, minimum))
    }

    fn deliver(
        &self,
        target_kind: ActivityTargetKind,
        envelope: &EvolutionActivityEnvelopeV1,
        target_scope: &str,
        projected_at_ms: i64,
    ) -> Result<(), ActivityTargetProjectionError> {
        match target_kind {
            ActivityTargetKind::SystemTimeline => self
                .repository
                .deliver_timeline(&envelope.event_id, projected_at_ms)
                .map(|_| ())
                .map_err(map_error),
            ActivityTargetKind::SkillDashboard => self
                .repository
                .materialize_dashboard(&envelope.event_id, projected_at_ms)
                .map(|_| ())
                .map_err(map_error),
            ActivityTargetKind::UnreadState => self
                .repository
                .project_unread(target_scope, LOCAL_ACTIVITY_USER_ID, projected_at_ms)
                .map(|_| ())
                .map_err(map_error),
            ActivityTargetKind::Notification => {
                self.notification
                    .deliver_notification(envelope, target_scope, projected_at_ms)
            }
        }
    }

    fn record_receipt(
        &self,
        receipt: &ActivityTargetReceipt,
    ) -> Result<(), ActivityTargetProjectionError> {
        self.repository
            .record_target_receipt(receipt)
            .map(|_| ())
            .map_err(map_error)
    }
}

fn map_error(error: ActivityProjectionRepositoryError) -> ActivityTargetProjectionError {
    match error {
        ActivityProjectionRepositoryError::InvalidInput => {
            ActivityTargetProjectionError::InvalidInput
        }
        ActivityProjectionRepositoryError::ReceiptCollision => {
            ActivityTargetProjectionError::ReceiptCollision
        }
        ActivityProjectionRepositoryError::LeaseHeld
        | ActivityProjectionRepositoryError::Conflict
        | ActivityProjectionRepositoryError::Storage => ActivityTargetProjectionError::Storage,
    }
}
