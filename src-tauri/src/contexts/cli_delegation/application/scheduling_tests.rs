use super::*;

fn request(id: &str, session: &str, generation: &str, at: u64) -> DelegationQueueSnapshot {
    DelegationQueueSnapshot {
        id: id.to_owned(),
        session_id: session.to_owned(),
        generation_id: generation.to_owned(),
        enqueued_at_millis: at,
    }
}

#[test]
fn scheduler_enforces_global_session_queue_and_generation_limits() {
    let mut scheduler = DelegationScheduler::new(DelegationLimitProfile::HARD_CEILING);
    assert_eq!(
        scheduler.admit(request("a", "s1", "g1", 0)),
        Ok(DelegationAdmission::StartNow)
    );
    assert_eq!(
        scheduler.admit(request("b", "s1", "g1", 0)),
        Ok(DelegationAdmission::Queued)
    );
    assert_eq!(
        scheduler.admit(request("c", "s2", "g1", 0)),
        Ok(DelegationAdmission::StartNow)
    );
    assert_eq!(
        scheduler.admit(request("d", "s3", "g1", 0)),
        Err(DelegationLimitError::GenerationAttemptLimit)
    );
    assert_eq!(
        scheduler
            .complete("a", 1)
            .expect("complete")
            .expect("next")
            .id,
        "b"
    );
}

#[test]
fn expired_queue_entries_and_observed_resource_overflow_are_explicit() {
    let mut limits = DelegationLimitProfile::HARD_CEILING;
    limits.global_active = 1;
    limits.maximum_queue_wait = Duration::from_millis(10);
    let mut scheduler = DelegationScheduler::new(limits);
    scheduler
        .admit(request("active", "s1", "g1", 0))
        .expect("active");
    scheduler
        .admit(request("queued", "s2", "g2", 0))
        .expect("queued");
    assert_eq!(scheduler.expire_queued(11), vec!["queued"]);

    assert_eq!(
        DelegationObservedUsage {
            events: limits.attempt_events + 1,
            ..Default::default()
        }
        .enforce(DelegationMode::Analyze, limits),
        Err(DelegationLimitError::EventLimit)
    );
    assert_eq!(
        DelegationObservedUsage {
            transcript_summary_bytes: limits.transcript_summary_bytes + 1,
            ..Default::default()
        }
        .enforce(DelegationMode::Analyze, limits),
        Err(DelegationLimitError::TranscriptSummaryLimit)
    );
    assert_eq!(
        DelegationObservedUsage {
            elapsed: limits.edit_wall_time + Duration::from_millis(1),
            ..Default::default()
        }
        .enforce(DelegationMode::Edit, limits),
        Err(DelegationLimitError::DurationLimit)
    );
    assert_eq!(
        DelegationObservedUsage {
            result_bytes: limits.result_bytes + 1,
            ..Default::default()
        }
        .enforce(DelegationMode::Analyze, limits),
        Err(DelegationLimitError::ResultSizeLimit)
    );
}
