use thiserror::Error;

use crate::contexts::skill_evolution_system_activity::domain::*;

const TARGET_KINDS: [ActivityTargetKind; 4] = [
    ActivityTargetKind::SystemTimeline,
    ActivityTargetKind::SkillDashboard,
    ActivityTargetKind::UnreadState,
    ActivityTargetKind::Notification,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityTargetProjectionOutcome {
    pub(crate) target_kind: ActivityTargetKind,
    pub(crate) status: Result<ActivityDeliveryStatus, ActivityTargetProjectionError>,
    pub(crate) replayed: bool,
}

pub(crate) trait ActivityTargetDeliveryPort {
    fn load_envelope(
        &self,
        event_id: &str,
    ) -> Result<EvolutionActivityEnvelopeV1, ActivityTargetProjectionError>;

    fn receipt(
        &self,
        event_id: &str,
        target_kind: ActivityTargetKind,
        target_scope: &str,
    ) -> Result<Option<ActivityTargetReceipt>, ActivityTargetProjectionError>;

    fn policy_allows(
        &self,
        target_kind: ActivityTargetKind,
        envelope: &EvolutionActivityEnvelopeV1,
    ) -> Result<bool, ActivityTargetProjectionError> {
        Ok(target_kind != ActivityTargetKind::Notification
            || envelope.attention_kind != ActivityAttentionKind::None)
    }

    fn deliver(
        &self,
        target_kind: ActivityTargetKind,
        envelope: &EvolutionActivityEnvelopeV1,
        target_scope: &str,
        projected_at_ms: i64,
    ) -> Result<(), ActivityTargetProjectionError>;

    fn record_receipt(
        &self,
        receipt: &ActivityTargetReceipt,
    ) -> Result<(), ActivityTargetProjectionError>;
}

pub(crate) trait ActivityNotificationDeliveryPort {
    fn notification_is_eligible(
        &self,
        envelope: &EvolutionActivityEnvelopeV1,
    ) -> Result<bool, ActivityTargetProjectionError> {
        Ok(envelope.attention_kind != ActivityAttentionKind::None)
    }

    fn deliver_notification(
        &self,
        envelope: &EvolutionActivityEnvelopeV1,
        target_scope: &str,
        projected_at_ms: i64,
    ) -> Result<(), ActivityTargetProjectionError>;
}

pub(crate) struct ActivityTargetProjector<'port> {
    port: &'port dyn ActivityTargetDeliveryPort,
}

impl<'port> ActivityTargetProjector<'port> {
    pub(crate) fn new(port: &'port dyn ActivityTargetDeliveryPort) -> Self {
        Self { port }
    }

    pub(crate) fn project(
        &self,
        event_id: &str,
        projected_at_ms: i64,
    ) -> Result<Vec<ActivityTargetProjectionOutcome>, ActivityTargetProjectionError> {
        if projected_at_ms < 0 {
            return Err(ActivityTargetProjectionError::InvalidInput);
        }
        let envelope = self.port.load_envelope(event_id)?;
        envelope
            .validate()
            .map_err(|_| ActivityTargetProjectionError::InvalidEnvelope)?;
        TARGET_KINDS
            .into_iter()
            .map(|kind| self.project_target(kind, &envelope, projected_at_ms))
            .collect()
    }

    fn project_target(
        &self,
        kind: ActivityTargetKind,
        envelope: &EvolutionActivityEnvelopeV1,
        projected_at_ms: i64,
    ) -> Result<ActivityTargetProjectionOutcome, ActivityTargetProjectionError> {
        let target_scope = target_scope(kind, envelope)?;
        if let Some(receipt) = self.port.receipt(&envelope.event_id, kind, &target_scope)? {
            if receipt.status != ActivityDeliveryStatus::Failed {
                return Ok(ActivityTargetProjectionOutcome {
                    target_kind: kind,
                    status: Ok(receipt.status),
                    replayed: true,
                });
            }
        }

        if !target_is_eligible(kind, envelope) || !self.port.policy_allows(kind, envelope)? {
            self.persist_receipt(
                kind,
                envelope,
                target_scope,
                ActivityDeliveryStatus::Suppressed,
                None,
            )?;
            return Ok(ActivityTargetProjectionOutcome {
                target_kind: kind,
                status: Ok(ActivityDeliveryStatus::Suppressed),
                replayed: false,
            });
        }

        let delivery = self
            .port
            .deliver(kind, envelope, &target_scope, projected_at_ms);
        let status = if delivery.is_ok() {
            ActivityDeliveryStatus::Delivered
        } else {
            ActivityDeliveryStatus::Failed
        };
        self.persist_receipt(
            kind,
            envelope,
            target_scope,
            status,
            (status == ActivityDeliveryStatus::Delivered).then_some(projected_at_ms),
        )?;
        Ok(ActivityTargetProjectionOutcome {
            target_kind: kind,
            status: delivery.map(|_| status),
            replayed: false,
        })
    }

    fn persist_receipt(
        &self,
        kind: ActivityTargetKind,
        envelope: &EvolutionActivityEnvelopeV1,
        target_scope: String,
        status: ActivityDeliveryStatus,
        delivered_at_ms: Option<i64>,
    ) -> Result<(), ActivityTargetProjectionError> {
        self.port.record_receipt(&ActivityTargetReceipt {
            event_id: envelope.event_id.clone(),
            target_kind: kind,
            target_scope,
            status,
            delivered_at_ms,
        })
    }
}

fn target_is_eligible(kind: ActivityTargetKind, envelope: &EvolutionActivityEnvelopeV1) -> bool {
    match kind {
        ActivityTargetKind::SystemTimeline | ActivityTargetKind::UnreadState => true,
        ActivityTargetKind::SkillDashboard => {
            DashboardMaterializationV1::from_envelope(envelope).is_some()
        }
        ActivityTargetKind::Notification => true,
    }
}

fn target_scope(
    kind: ActivityTargetKind,
    envelope: &EvolutionActivityEnvelopeV1,
) -> Result<String, ActivityTargetProjectionError> {
    match kind {
        ActivityTargetKind::SystemTimeline | ActivityTargetKind::UnreadState => {
            stable_system_activity_session_id(
                ActivityKind::SkillEvolution,
                envelope.scope_kind,
                &envelope.canonical_scope_id,
            )
            .map_err(|_| ActivityTargetProjectionError::InvalidInput)
        }
        ActivityTargetKind::SkillDashboard | ActivityTargetKind::Notification => Ok(format!(
            "{}:{}",
            match envelope.scope_kind {
                ActivityScopeKind::Global => "global",
                ActivityScopeKind::Workspace => "workspace",
            },
            envelope.canonical_scope_id
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum ActivityTargetProjectionError {
    #[error("activity target input is invalid")]
    InvalidInput,
    #[error("activity target envelope is invalid")]
    InvalidEnvelope,
    #[error("activity target delivery is unavailable")]
    Unavailable,
    #[error("activity target storage failed")]
    Storage,
    #[error("activity target receipt conflicts with committed delivery")]
    ReceiptCollision,
}
