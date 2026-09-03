use rusqlite::{params, OptionalExtension};

use super::{ActivityProjectionRepositoryError, SqliteActivityProjectionRepository};
use crate::contexts::skill_evolution_system_activity::domain::{
    sanitize_text, ActivityDeliveryStatus, ActivityTargetReceipt,
};

impl SqliteActivityProjectionRepository<'_> {
    pub(crate) fn target_receipt(
        &self,
        event_id: &str,
        target_kind: crate::contexts::skill_evolution_system_activity::domain::ActivityTargetKind,
        target_scope: &str,
    ) -> Result<Option<ActivityTargetReceipt>, ActivityProjectionRepositoryError> {
        let target_kind_text = enum_text(target_kind)?;
        self.connection
            .query_row(
                "SELECT delivery_status,delivered_at_ms FROM evolution_activity_target_receipts
                 WHERE event_id=?1 AND target_kind=?2 AND target_scope=?3",
                params![event_id, target_kind_text, target_scope],
                |row| {
                    Ok(ActivityTargetReceipt {
                        event_id: event_id.to_owned(),
                        target_kind,
                        target_scope: target_scope.to_owned(),
                        status: parse_status(&row.get::<_, String>(0)?)?,
                        delivered_at_ms: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn record_target_receipt(
        &self,
        receipt: &ActivityTargetReceipt,
    ) -> Result<bool, ActivityProjectionRepositoryError> {
        validate_target_receipt(receipt)?;
        let target_kind = enum_text(receipt.target_kind)?;
        let delivery_status = enum_text(receipt.status)?;
        let changed = self.connection.execute(
            "INSERT OR IGNORE INTO evolution_activity_target_receipts
             (event_id,target_kind,target_scope,delivery_status,delivered_at_ms,detail_code)
             VALUES (?1,?2,?3,?4,?5,NULL)",
            params![
                receipt.event_id,
                target_kind,
                receipt.target_scope,
                delivery_status,
                receipt.delivered_at_ms
            ],
        )?;
        if changed == 1 {
            return Ok(true);
        }
        let existing = self
            .connection
            .query_row(
                "SELECT delivery_status,delivered_at_ms FROM evolution_activity_target_receipts
                 WHERE event_id=?1 AND target_kind=?2 AND target_scope=?3",
                params![receipt.event_id, target_kind, receipt.target_scope],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?)),
            )
            .optional()?;
        if existing == Some((delivery_status.clone(), receipt.delivered_at_ms)) {
            return Ok(false);
        }
        if existing
            .as_ref()
            .is_some_and(|(status, _)| status == "failed")
            && delivery_status != "failed"
        {
            return self.recover_failed_target_receipt(receipt, &target_kind, &delivery_status);
        }
        Err(ActivityProjectionRepositoryError::ReceiptCollision)
    }

    fn recover_failed_target_receipt(
        &self,
        receipt: &ActivityTargetReceipt,
        target_kind: &str,
        delivery_status: &str,
    ) -> Result<bool, ActivityProjectionRepositoryError> {
        let changed = self.connection.execute(
            "UPDATE evolution_activity_target_receipts
             SET delivery_status=?1,delivered_at_ms=?2
             WHERE event_id=?3 AND target_kind=?4 AND target_scope=?5
               AND delivery_status='failed'",
            params![
                delivery_status,
                receipt.delivered_at_ms,
                receipt.event_id,
                target_kind,
                receipt.target_scope
            ],
        )?;
        Ok(changed == 1)
    }
}

fn parse_status(value: &str) -> rusqlite::Result<ActivityDeliveryStatus> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn validate_target_receipt(
    receipt: &ActivityTargetReceipt,
) -> Result<(), ActivityProjectionRepositoryError> {
    if sanitize_text(&receipt.event_id, "target_receipt.event_id", 160).is_err()
        || sanitize_text(&receipt.target_scope, "target_receipt.target_scope", 160).is_err()
        || (receipt.status == ActivityDeliveryStatus::Delivered
            && receipt.delivered_at_ms.is_none())
    {
        return Err(ActivityProjectionRepositoryError::InvalidInput);
    }
    Ok(())
}

fn enum_text<T: serde::Serialize>(value: T) -> Result<String, ActivityProjectionRepositoryError> {
    serde_json::to_value(value)
        .map_err(|_| ActivityProjectionRepositoryError::Storage)?
        .as_str()
        .map(str::to_owned)
        .ok_or(ActivityProjectionRepositoryError::Storage)
}
