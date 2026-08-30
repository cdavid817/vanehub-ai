use crate::contexts::{
    skill_evolution_curation::api::SkillEvolutionCurationApi,
    skill_evolution_orchestration::domain::{
        AutoRateReservationV1, RateReservationHistoryObservationV1,
    },
    tooling::skills::api::{OverlayKey, OverlayScope, SkillApi, SkillId},
};

use super::{RateReservationError, SqliteRateReservationRepository};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RateHistoryReconciliationRequestV1 {
    pub(crate) reservation_id: String,
    pub(crate) expected_revision: u64,
    pub(crate) automatic_application_id: String,
    pub(crate) curator_application_id: String,
    pub(crate) overlay_history_id: String,
    pub(crate) skill_id: String,
    pub(crate) workspace_id: String,
    pub(crate) overlay_scope: String,
    pub(crate) now_ms: i64,
}

pub(crate) struct CuratorOverlayRateHistoryReconciler<'a> {
    repository: &'a SqliteRateReservationRepository,
    curator: &'a SkillEvolutionCurationApi,
    skills: &'a SkillApi,
}

impl<'a> CuratorOverlayRateHistoryReconciler<'a> {
    pub(crate) fn new(
        repository: &'a SqliteRateReservationRepository,
        curator: &'a SkillEvolutionCurationApi,
        skills: &'a SkillApi,
    ) -> Self {
        Self {
            repository,
            curator,
            skills,
        }
    }

    pub(crate) fn reconcile(
        &self,
        request: &RateHistoryReconciliationRequestV1,
    ) -> Result<AutoRateReservationV1, RateReservationError> {
        let curator = self
            .curator
            .application_recovery_status(&request.curator_application_id)
            .map_err(|_| RateReservationError::Storage)?;
        let overlay = self.overlay_history(request)?;
        let curator_found = curator.as_ref().is_some_and(|status| {
            status.application_id == request.curator_application_id
                && status.overlay_history_id.as_deref() == Some(request.overlay_history_id.as_str())
        });
        let overlay_found = overlay.as_ref().is_some_and(|entry| {
            entry.event_id == request.overlay_history_id
                && entry.curator_application_id.as_deref()
                    == Some(request.curator_application_id.as_str())
        });
        let automatic_application_id = (curator.is_some() || overlay.is_some())
            .then(|| request.automatic_application_id.clone());
        self.repository.reconcile(
            &request.reservation_id,
            request.expected_revision,
            &RateReservationHistoryObservationV1 {
                automatic_application_id,
                curator_application_id: curator_found
                    .then(|| request.curator_application_id.clone()),
                overlay_application_id: overlay_found.then(|| request.overlay_history_id.clone()),
            },
            request.now_ms,
        )
    }

    fn overlay_history(
        &self,
        request: &RateHistoryReconciliationRequestV1,
    ) -> Result<
        Option<crate::contexts::tooling::skills::api::OverlayHistoryEntry>,
        RateReservationError,
    > {
        let canonical_skill_id =
            SkillId::parse(&request.skill_id).map_err(|_| RateReservationError::InvalidInput)?;
        let scope = match request.overlay_scope.as_str() {
            "user" => OverlayScope::User,
            "project" => OverlayScope::Project,
            _ => return Err(RateReservationError::InvalidInput),
        };
        let key = OverlayKey {
            canonical_skill_id,
            scope,
            workspace_identity: (scope == OverlayScope::Project)
                .then(|| request.workspace_id.clone()),
        };
        self.skills
            .overlay_history_by_application(&key, &request.curator_application_id)
            .map_err(|_| RateReservationError::Storage)
    }
}
