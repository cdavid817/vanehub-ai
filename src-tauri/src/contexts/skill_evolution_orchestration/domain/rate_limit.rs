use super::RateReservationStatus;

pub(crate) const AUTOMATIC_WORKSPACE_LIMIT_24H_V1: u8 = 3;
pub(crate) const AUTOMATIC_SKILL_LIMIT_7D_V1: u8 = 1;
pub(crate) const AUTOMATIC_RUN_LIMIT_V1: u8 = 1;
pub(crate) const ROLLING_DAY_MS: i64 = 24 * 60 * 60 * 1_000;
pub(crate) const ROLLING_WEEK_MS: i64 = 7 * ROLLING_DAY_MS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RateReservationHistoryObservationV1 {
    pub(crate) automatic_application_id: Option<String>,
    pub(crate) curator_application_id: Option<String>,
    pub(crate) overlay_application_id: Option<String>,
}

pub(crate) fn reconciled_rate_status(
    observation: &RateReservationHistoryObservationV1,
) -> RateReservationStatus {
    match (
        &observation.automatic_application_id,
        &observation.curator_application_id,
        &observation.overlay_application_id,
    ) {
        (None, None, None) => RateReservationStatus::Released,
        (Some(_), Some(_), Some(_)) => RateReservationStatus::Committed,
        _ => RateReservationStatus::RecoveryRequired,
    }
}
