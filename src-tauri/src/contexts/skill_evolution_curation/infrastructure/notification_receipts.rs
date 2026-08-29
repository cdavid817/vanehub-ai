use crate::contexts::skill_evolution_curation::application::{
    CuratorNotificationDeliveryStatus, CuratorNotificationEvent, CuratorNotificationKind,
    CuratorNotificationNavigationTarget, CuratorNotificationStore, CuratorNotificationStoreError,
};
use crate::contexts::skill_evolution_curation::domain::{
    CuratorEventKind, CuratorRisk, CuratorRoute,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde_json::Value;

pub(super) fn queue_notification_receipt(
    transaction: &Transaction<'_>,
    candidate_id: &str,
    candidate_revision: u64,
    event_kind: CuratorEventKind,
) -> Result<(), super::CuratorRepositoryError> {
    let Some(kind) = notification_name(event_kind) else {
        return Ok(());
    };
    let policy = transaction
        .query_row(
            "SELECT p.policy_json FROM evolution_curator_candidates c
             LEFT JOIN evolution_curator_policy p ON p.workspace_id=c.workspace_id
             WHERE c.candidate_id=?1",
            [candidate_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|_| super::CuratorRepositoryError::Storage)?
        .flatten();
    let enabled = policy
        .map(|document| {
            serde_json::from_str::<Value>(&document)
                .ok()
                .and_then(|value| value.get("notificationsEnabled")?.as_bool())
                .ok_or(super::CuratorRepositoryError::Storage)
        })
        .transpose()?
        .unwrap_or(true);
    transaction
        .execute(
            "INSERT OR IGNORE INTO evolution_curator_notification_receipts
             (candidate_id,candidate_revision,event_kind,delivery_status)
             VALUES (?1,?2,?3,?4)",
            params![
                candidate_id,
                i64::try_from(candidate_revision)
                    .map_err(|_| super::CuratorRepositoryError::Storage)?,
                kind,
                if enabled { "pending" } else { "suppressed" }
            ],
        )
        .map_err(|_| super::CuratorRepositoryError::Storage)?;
    Ok(())
}

pub(crate) struct SqliteCuratorNotificationStore<'a> {
    connection: &'a Connection,
}

impl<'a> SqliteCuratorNotificationStore<'a> {
    pub(crate) fn new(connection: &'a Connection) -> Self {
        Self { connection }
    }
}

impl CuratorNotificationStore for SqliteCuratorNotificationStore<'_> {
    fn pending(
        &mut self,
        limit: usize,
    ) -> Result<Vec<CuratorNotificationEvent>, CuratorNotificationStoreError> {
        if !(1..=100).contains(&limit) {
            return Err(CuratorNotificationStoreError::InvalidProjection);
        }
        let mut statement = self
            .connection
            .prepare(
                "SELECT n.event_kind,n.candidate_id,n.candidate_revision,c.workspace_id,
             c.target_skill_id,c.overlay_scope,c.state,c.risk,c.route,
             (SELECT a.overlay_history_id FROM evolution_curator_applications a
              WHERE a.candidate_id=c.candidate_id AND a.overlay_history_id IS NOT NULL
              ORDER BY a.updated_at_ms DESC LIMIT 1)
             FROM evolution_curator_notification_receipts n
             JOIN evolution_curator_candidates c ON c.candidate_id=n.candidate_id
             WHERE n.delivery_status IN ('pending','failed')
             ORDER BY c.updated_at_ms,n.candidate_id,n.event_kind LIMIT ?1",
            )
            .map_err(|_| CuratorNotificationStoreError::Storage)?;
        let events = statement
            .query_map([limit as i64], |row| {
                Ok(NotificationRow {
                    kind: row.get(0)?,
                    candidate_id: row.get(1)?,
                    revision: row.get(2)?,
                    workspace_id: row.get(3)?,
                    skill_id: row.get(4)?,
                    overlay_scope: row.get(5)?,
                    state: row.get(6)?,
                    risk: row.get(7)?,
                    route: row.get(8)?,
                    overlay_history_id: row.get(9)?,
                })
            })
            .map_err(|_| CuratorNotificationStoreError::Storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| CuratorNotificationStoreError::Storage)?
            .into_iter()
            .map(NotificationRow::event)
            .collect();
        events
    }

    fn finish(
        &mut self,
        event: &CuratorNotificationEvent,
        status: CuratorNotificationDeliveryStatus,
        occurred_at_ms: i64,
    ) -> Result<(), CuratorNotificationStoreError> {
        let updated = self.connection.execute(
            "UPDATE evolution_curator_notification_receipts SET delivery_status=?1,delivered_at_ms=?2
             WHERE candidate_id=?3 AND candidate_revision=?4 AND event_kind=?5
             AND delivery_status IN ('pending','failed')",
            params![
                match status {
                    CuratorNotificationDeliveryStatus::Delivered => "delivered",
                    CuratorNotificationDeliveryStatus::Failed => "failed",
                },
                (status == CuratorNotificationDeliveryStatus::Delivered).then_some(occurred_at_ms),
                event.candidate_id,
                i64::try_from(event.candidate_revision)
                    .map_err(|_| CuratorNotificationStoreError::InvalidProjection)?,
                notification_kind_name(event.event_kind),
            ],
        ).map_err(|_| CuratorNotificationStoreError::Storage)?;
        if updated == 1 {
            Ok(())
        } else {
            Err(CuratorNotificationStoreError::Storage)
        }
    }
}

struct NotificationRow {
    kind: String,
    candidate_id: String,
    revision: i64,
    workspace_id: String,
    skill_id: String,
    overlay_scope: String,
    state: String,
    risk: String,
    route: String,
    overlay_history_id: Option<String>,
}

impl NotificationRow {
    fn event(self) -> Result<CuratorNotificationEvent, CuratorNotificationStoreError> {
        let kind = parse_notification_kind(&self.kind)?;
        for value in [
            &self.candidate_id,
            &self.workspace_id,
            &self.skill_id,
            &self.overlay_scope,
        ] {
            if value.trim().is_empty() || value.len() > 256 {
                return Err(CuratorNotificationStoreError::InvalidProjection);
            }
        }
        let navigation_target = match (kind, self.overlay_history_id) {
            (CuratorNotificationKind::ApplySuccess, Some(overlay_history_id))
                if !overlay_history_id.is_empty() && overlay_history_id.len() <= 256 =>
            {
                CuratorNotificationNavigationTarget::OverlayHistory {
                    candidate_id: self.candidate_id.clone(),
                    skill_id: self.skill_id.clone(),
                    overlay_history_id,
                }
            }
            (CuratorNotificationKind::ApplySuccess, _) => {
                return Err(CuratorNotificationStoreError::InvalidProjection)
            }
            _ => CuratorNotificationNavigationTarget::CandidateReview {
                candidate_id: self.candidate_id.clone(),
            },
        };
        Ok(CuratorNotificationEvent {
            schema_version: 1,
            event_kind: kind,
            candidate_id: self.candidate_id,
            candidate_revision: u64::try_from(self.revision)
                .map_err(|_| CuratorNotificationStoreError::InvalidProjection)?,
            workspace_id: self.workspace_id,
            skill_id: self.skill_id,
            overlay_scope: self.overlay_scope,
            state: super::repository_support::parse_state(&self.state)
                .map_err(|_| CuratorNotificationStoreError::InvalidProjection)?,
            risk: parse_risk(&self.risk)?,
            route: parse_route(&self.route)?,
            navigation_target,
        })
    }
}

fn notification_name(kind: CuratorEventKind) -> Option<&'static str> {
    match kind {
        CuratorEventKind::DraftAssessed => Some("pending_review"),
        CuratorEventKind::Deferred => Some("deferral_date"),
        CuratorEventKind::Superseded => Some("supersession"),
        CuratorEventKind::Rejected => Some("rejection"),
        CuratorEventKind::Applied => Some("apply_success"),
        CuratorEventKind::ApplicationFailed => Some("apply_failure"),
        _ => None,
    }
}

fn notification_kind_name(kind: CuratorNotificationKind) -> &'static str {
    match kind {
        CuratorNotificationKind::PendingReview => "pending_review",
        CuratorNotificationKind::DeferralDate => "deferral_date",
        CuratorNotificationKind::Supersession => "supersession",
        CuratorNotificationKind::Rejection => "rejection",
        CuratorNotificationKind::ApplySuccess => "apply_success",
        CuratorNotificationKind::ApplyFailure => "apply_failure",
        CuratorNotificationKind::ProbationRegression => "probation_regression",
    }
}

fn parse_notification_kind(
    value: &str,
) -> Result<CuratorNotificationKind, CuratorNotificationStoreError> {
    match value {
        "pending_review" => Ok(CuratorNotificationKind::PendingReview),
        "deferral_date" => Ok(CuratorNotificationKind::DeferralDate),
        "supersession" => Ok(CuratorNotificationKind::Supersession),
        "rejection" => Ok(CuratorNotificationKind::Rejection),
        "apply_success" => Ok(CuratorNotificationKind::ApplySuccess),
        "apply_failure" => Ok(CuratorNotificationKind::ApplyFailure),
        "probation_regression" => Ok(CuratorNotificationKind::ProbationRegression),
        _ => Err(CuratorNotificationStoreError::InvalidProjection),
    }
}

fn parse_risk(value: &str) -> Result<CuratorRisk, CuratorNotificationStoreError> {
    match value {
        "low" => Ok(CuratorRisk::Low),
        "medium" => Ok(CuratorRisk::Medium),
        "high" => Ok(CuratorRisk::High),
        _ => Err(CuratorNotificationStoreError::InvalidProjection),
    }
}

fn parse_route(value: &str) -> Result<CuratorRoute, CuratorNotificationStoreError> {
    match value {
        "advance" => Ok(CuratorRoute::Advance),
        "needs_human_review" => Ok(CuratorRoute::NeedsHumanReview),
        _ => Err(CuratorNotificationStoreError::InvalidProjection),
    }
}

#[cfg(test)]
pub(super) fn receipt_status(
    connection: &Connection,
    candidate_id: &str,
    revision: u64,
    kind: &str,
) -> Option<String> {
    connection
        .query_row(
            "SELECT delivery_status FROM evolution_curator_notification_receipts
             WHERE candidate_id=?1 AND candidate_revision=?2 AND event_kind=?3",
            params![candidate_id, revision as i64, kind],
            |row| row.get(0),
        )
        .optional()
        .ok()
        .flatten()
}
