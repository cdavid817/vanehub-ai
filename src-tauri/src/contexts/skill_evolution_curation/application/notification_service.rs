use crate::contexts::skill_evolution_curation::domain::{
    CuratorCandidateState, CuratorRisk, CuratorRoute,
};
use serde::Serialize;
use thiserror::Error;

pub(crate) const CURATOR_NOTIFICATION_PAGE_LIMIT: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CuratorNotificationKind {
    PendingReview,
    DeferralDate,
    Supersession,
    Rejection,
    ApplySuccess,
    ApplyFailure,
    ProbationRegression,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum CuratorNotificationNavigationTarget {
    CandidateReview {
        candidate_id: String,
    },
    OverlayHistory {
        candidate_id: String,
        skill_id: String,
        overlay_history_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CuratorNotificationEvent {
    pub(crate) schema_version: u16,
    pub(crate) event_kind: CuratorNotificationKind,
    pub(crate) candidate_id: String,
    pub(crate) candidate_revision: u64,
    pub(crate) workspace_id: String,
    pub(crate) skill_id: String,
    pub(crate) overlay_scope: String,
    pub(crate) state: CuratorCandidateState,
    pub(crate) risk: CuratorRisk,
    pub(crate) route: CuratorRoute,
    pub(crate) navigation_target: CuratorNotificationNavigationTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CuratorNotificationDeliveryStatus {
    Delivered,
    Failed,
}

pub(crate) trait CuratorNotificationStore {
    fn pending(
        &mut self,
        limit: usize,
    ) -> Result<Vec<CuratorNotificationEvent>, CuratorNotificationStoreError>;

    fn finish(
        &mut self,
        event: &CuratorNotificationEvent,
        status: CuratorNotificationDeliveryStatus,
        occurred_at_ms: i64,
    ) -> Result<(), CuratorNotificationStoreError>;
}

pub(crate) trait CuratorNotificationPort: Send + Sync {
    fn publish(&self, event: &CuratorNotificationEvent) -> Result<(), ()>;
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CuratorNotificationDispatchReport {
    pub(crate) delivered: usize,
    pub(crate) failed: usize,
}

pub(crate) struct CuratorNotificationService<'a, S, P: ?Sized> {
    store: &'a mut S,
    publisher: &'a P,
}

impl<'a, S, P> CuratorNotificationService<'a, S, P>
where
    S: CuratorNotificationStore,
    P: CuratorNotificationPort + ?Sized,
{
    pub(crate) fn new(store: &'a mut S, publisher: &'a P) -> Self {
        Self { store, publisher }
    }

    pub(crate) fn dispatch(
        &mut self,
        occurred_at_ms: i64,
    ) -> Result<CuratorNotificationDispatchReport, CuratorNotificationStoreError> {
        let events = self.store.pending(CURATOR_NOTIFICATION_PAGE_LIMIT)?;
        let mut report = CuratorNotificationDispatchReport::default();
        for event in events {
            let status = if self.publisher.publish(&event).is_ok() {
                report.delivered += 1;
                CuratorNotificationDeliveryStatus::Delivered
            } else {
                report.failed += 1;
                CuratorNotificationDeliveryStatus::Failed
            };
            self.store.finish(&event, status, occurred_at_ms)?;
        }
        Ok(report)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum CuratorNotificationStoreError {
    #[error("curator notification storage failed")]
    Storage,
    #[error("curator notification projection is invalid")]
    InvalidProjection,
}
