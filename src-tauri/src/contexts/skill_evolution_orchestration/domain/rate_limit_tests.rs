use super::*;

#[test]
fn reconciliation_releases_only_when_both_authoritative_histories_are_empty() {
    let observation = |automatic: Option<&str>, curator: Option<&str>, overlay: Option<&str>| {
        RateReservationHistoryObservationV1 {
            automatic_application_id: automatic.map(str::to_string),
            curator_application_id: curator.map(str::to_string),
            overlay_application_id: overlay.map(str::to_string),
        }
    };
    assert_eq!(
        reconciled_rate_status(&observation(None, None, None)),
        RateReservationStatus::Released
    );
    assert_eq!(
        reconciled_rate_status(&observation(Some("auto"), Some("curator"), Some("overlay"))),
        RateReservationStatus::Committed
    );
    for partial in [
        observation(None, Some("curator"), None),
        observation(None, None, Some("overlay")),
        observation(Some("auto"), Some("curator"), None),
    ] {
        assert_eq!(
            reconciled_rate_status(&partial),
            RateReservationStatus::RecoveryRequired
        );
    }
}
