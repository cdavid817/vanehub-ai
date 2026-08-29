use std::collections::BTreeMap;

use rusqlite::{params, OptionalExtension};

use super::{ActivityProjectionRepositoryError, SqliteActivityProjectionRepository};
use crate::contexts::skill_evolution_system_activity::{application::*, domain::*};

impl ActivityNotificationDeliveryPort for SqliteActivityProjectionRepository<'_> {
    fn notification_is_eligible(
        &self,
        envelope: &EvolutionActivityEnvelopeV1,
    ) -> Result<bool, ActivityTargetProjectionError> {
        Ok(self.plan(envelope).map_err(map_target_error)? != ActivityNotificationPlan::Suppressed)
    }

    fn deliver_notification(
        &self,
        envelope: &EvolutionActivityEnvelopeV1,
        target_scope: &str,
        projected_at_ms: i64,
    ) -> Result<(), ActivityTargetProjectionError> {
        if projected_at_ms < 0 {
            return Err(ActivityTargetProjectionError::InvalidInput);
        }
        match self.plan(envelope).map_err(map_target_error)? {
            ActivityNotificationPlan::Immediate => self
                .persist_immediate_request(envelope, target_scope, projected_at_ms)
                .map_err(map_target_error),
            ActivityNotificationPlan::Digest(cadence) => self
                .persist_digest(envelope, target_scope, cadence, projected_at_ms)
                .map_err(map_target_error),
            ActivityNotificationPlan::Suppressed => {
                Err(ActivityTargetProjectionError::InvalidInput)
            }
        }
    }
}

impl SqliteActivityProjectionRepository<'_> {
    fn plan(
        &self,
        envelope: &EvolutionActivityEnvelopeV1,
    ) -> Result<ActivityNotificationPlan, ActivityProjectionRepositoryError> {
        let preferences = self
            .preferences(envelope.scope_kind, &envelope.canonical_scope_id)?
            .unwrap_or_else(|| default_preferences(envelope));
        Ok(notification_plan(envelope, &preferences))
    }

    fn persist_immediate_request(
        &self,
        envelope: &EvolutionActivityEnvelopeV1,
        target_scope: &str,
        projected_at_ms: i64,
    ) -> Result<(), ActivityProjectionRepositoryError> {
        sanitize_text(target_scope, "notification.target_scope", 200)
            .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
        self.connection.execute(
            "INSERT OR IGNORE INTO evolution_activity_notification_requests
             (request_id,event_id,target_scope,request_kind,status,created_at_ms,updated_at_ms)
             VALUES (?1,?2,?3,'immediate','pending',?4,?4)",
            params![
                format!("activity-notification:{}", envelope.event_id),
                envelope.event_id,
                target_scope,
                projected_at_ms,
            ],
        )?;
        Ok(())
    }

    fn persist_digest(
        &self,
        envelope: &EvolutionActivityEnvelopeV1,
        target_scope: &str,
        cadence: ActivityDigestCadence,
        projected_at_ms: i64,
    ) -> Result<(), ActivityProjectionRepositoryError> {
        let window_ms = match cadence {
            ActivityDigestCadence::Hourly => 3_600_000,
            ActivityDigestCadence::Daily => 86_400_000,
            ActivityDigestCadence::Off => {
                return Err(ActivityProjectionRepositoryError::InvalidInput);
            }
        };
        let started_at_ms = projected_at_ms - projected_at_ms.rem_euclid(window_ms);
        let cadence_text = enum_text(cadence)?;
        let bucket_id = format!("digest:{target_scope}:{cadence_text}:{started_at_ms}");
        let transaction = self.connection.unchecked_transaction()?;
        let existing = transaction
            .query_row(
                "SELECT counts_json,highest_severity FROM evolution_activity_digest_buckets
                 WHERE bucket_id=?1",
                [&bucket_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        let mut counts = existing
            .as_ref()
            .map(|(json, _)| serde_json::from_str(json))
            .transpose()
            .map_err(|_| ActivityProjectionRepositoryError::Storage)?
            .unwrap_or_else(BTreeMap::<String, u32>::new);
        let count = counts.entry(enum_text(envelope.event_code)?).or_default();
        *count = count.saturating_add(1);
        let prior_severity = existing
            .as_ref()
            .map(|(_, severity)| parse_severity(severity))
            .transpose()?;
        let highest = prior_severity
            .map(|prior| higher_severity(prior, envelope.severity))
            .unwrap_or(envelope.severity);
        transaction.execute(
            "INSERT INTO evolution_activity_digest_buckets
             (bucket_id,scope_kind,canonical_scope_id,cadence,window_started_at_ms,
              window_ends_at_ms,counts_json,highest_severity)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(bucket_id) DO UPDATE SET counts_json=excluded.counts_json,
               highest_severity=excluded.highest_severity",
            params![
                bucket_id,
                enum_text(envelope.scope_kind)?,
                envelope.canonical_scope_id,
                cadence_text,
                started_at_ms,
                started_at_ms.saturating_add(window_ms),
                serde_json::to_string(&counts)
                    .map_err(|_| ActivityProjectionRepositoryError::Storage)?,
                enum_text(highest)?,
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

fn default_preferences(envelope: &EvolutionActivityEnvelopeV1) -> EvolutionActivityPreferences {
    EvolutionActivityPreferences {
        scope_kind: envelope.scope_kind,
        canonical_scope_id: envelope.canonical_scope_id.clone(),
        visible: true,
        minimum_timeline_severity: ActivitySeverity::Info,
        notification_threshold: ActivitySeverity::Warning,
        digest_cadence: ActivityDigestCadence::Off,
        read_retention_days: 180,
        detail_retention_days: 180,
        export_item_limit: 1_000,
        export_size_limit_bytes: 10 * 1024 * 1024,
        revision: 0,
    }
}

fn higher_severity(left: ActivitySeverity, right: ActivitySeverity) -> ActivitySeverity {
    if severity_at_least(left, right) {
        left
    } else {
        right
    }
}

fn parse_severity(value: &str) -> Result<ActivitySeverity, ActivityProjectionRepositoryError> {
    serde_json::from_value(serde_json::Value::String(value.into()))
        .map_err(|_| ActivityProjectionRepositoryError::Storage)
}

fn enum_text(value: impl serde::Serialize) -> Result<String, ActivityProjectionRepositoryError> {
    serde_json::to_value(value)
        .map_err(|_| ActivityProjectionRepositoryError::Storage)?
        .as_str()
        .map(str::to_owned)
        .ok_or(ActivityProjectionRepositoryError::Storage)
}

fn map_target_error(error: ActivityProjectionRepositoryError) -> ActivityTargetProjectionError {
    match error {
        ActivityProjectionRepositoryError::InvalidInput => {
            ActivityTargetProjectionError::InvalidInput
        }
        ActivityProjectionRepositoryError::ReceiptCollision => {
            ActivityTargetProjectionError::ReceiptCollision
        }
        _ => ActivityTargetProjectionError::Storage,
    }
}
