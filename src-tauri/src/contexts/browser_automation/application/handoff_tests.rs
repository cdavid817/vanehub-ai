use super::*;

fn ownership() -> BrowserOwnership {
    BrowserOwnership {
        session_id: "session-1".to_owned(),
        generation_id: "generation-1".to_owned(),
    }
}

#[test]
fn handoff_pauses_every_automation_action_until_explicit_resume() {
    let manager = BrowserHandoffManager::default();
    let now = Instant::now();
    let handoff = manager
        .begin(
            ownership(),
            "page-1".to_owned(),
            now,
            Duration::from_secs(60),
        )
        .expect("handoff");
    assert_eq!(
        manager.ensure_automation_allowed(&ownership(), BrowserAction::Inspect, now),
        Err(BrowserHandoffError::AutomationPaused)
    );
    assert_eq!(
        manager.resume(&ownership(), &handoff.handoff_id, now, false),
        Err(BrowserHandoffError::StaleHandoff)
    );
    assert_eq!(
        manager.resume(&ownership(), &handoff.handoff_id, now, true),
        Ok(1)
    );
}

#[test]
fn resume_invalidates_references_until_a_fresh_inspection_completes() {
    let manager = BrowserHandoffManager::default();
    let now = Instant::now();
    let handoff = manager
        .begin(
            ownership(),
            "page-1".to_owned(),
            now,
            Duration::from_secs(60),
        )
        .expect("handoff");
    manager
        .resume(&ownership(), &handoff.handoff_id, now, true)
        .expect("resume");
    assert_eq!(
        manager.ensure_automation_allowed(&ownership(), BrowserAction::Click, now),
        Err(BrowserHandoffError::FreshInspectionRequired)
    );
    manager
        .ensure_automation_allowed(&ownership(), BrowserAction::Inspect, now)
        .expect("inspection admitted");
    manager
        .record_completed(&ownership(), BrowserAction::Inspect)
        .expect("inspection recorded");
    manager
        .ensure_automation_allowed(&ownership(), BrowserAction::Click, now)
        .expect("new references are usable");
}

#[test]
fn stale_ids_and_expired_handoffs_cannot_resume() {
    let manager = BrowserHandoffManager::default();
    let now = Instant::now();
    let handoff = manager
        .begin(
            ownership(),
            "page-1".to_owned(),
            now,
            Duration::from_secs(1),
        )
        .expect("handoff");
    assert_eq!(
        manager.resume(&ownership(), "forged", now, true),
        Err(BrowserHandoffError::StaleHandoff)
    );
    assert_eq!(
        manager.resume(
            &ownership(),
            &handoff.handoff_id,
            now + Duration::from_secs(2),
            true,
        ),
        Err(BrowserHandoffError::Expired)
    );
}
