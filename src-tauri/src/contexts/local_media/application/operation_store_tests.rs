use super::*;
use crate::contexts::local_media::domain::{LocalMediaErrorCode, PlaybackId, SpeechPlaybackResult};

fn playback(duration_ms: u64) -> LocalMediaOperationResult {
    LocalMediaOperationResult::Tts(SpeechPlaybackResult {
        playback_id: PlaybackId::new("lmp-0123456789abcdef0123456789abcdef"),
        sample_rate: 22_050,
        duration_ms,
        device_summary: None,
    })
}

fn store() -> LocalMediaOperationStore {
    LocalMediaOperationStore::new(60_000, 4)
}

#[test]
fn an_accepted_operation_has_no_result_yet() {
    let store = store();
    store.accept("op-1", LocalMediaOperationKind::Tts, 0);
    assert_eq!(store.result("op-1", 0), Ok(None));
    assert_eq!(store.phase("op-1"), Some(LocalMediaPhase::Accepted));
}

#[test]
fn an_unknown_operation_is_absent_rather_than_expired() {
    // The distinction matters to the composer: "expired" tells the user their result was dropped,
    // "absent" means they are polling something that never existed.
    assert_eq!(store().result("op-missing", 0), Ok(None));
}

#[test]
fn a_terminal_result_reads_identically_every_time() {
    let store = store();
    store.accept("op-1", LocalMediaOperationKind::Tts, 0);
    store.succeed("op-1", playback(100), 10);

    let first = store.result("op-1", 20).expect("first read");
    let second = store.result("op-1", 30).expect("second read");
    assert_eq!(first, second);
    assert_eq!(first, Some(playback(100)));
    assert_eq!(store.phase("op-1"), Some(LocalMediaPhase::Succeeded));
}

#[test]
fn a_result_expires_at_the_retention_boundary() {
    let store = store();
    store.accept("op-1", LocalMediaOperationKind::Tts, 0);
    store.succeed("op-1", playback(100), 1_000);

    assert!(store.result("op-1", 1_000 + 59_999).is_ok());
    assert_eq!(
        store
            .result("op-1", 1_000 + 60_000)
            .map_err(|error| error.code()),
        Err(LocalMediaErrorCode::OperationResultExpired)
    );
}

#[test]
fn expiry_is_measured_from_completion_not_acceptance() {
    let store = store();
    store.accept("op-1", LocalMediaOperationKind::Ocr, 0);
    // A long-running operation must not have its result expire the moment it finishes.
    store.succeed("op-1", playback(100), 500_000);
    assert!(store.result("op-1", 500_001).is_ok());
}

#[test]
fn a_failed_operation_reports_its_code_not_a_result() {
    let store = store();
    store.accept("op-1", LocalMediaOperationKind::Stt, 0);
    store.fail(
        "op-1",
        LocalMediaError::new(LocalMediaErrorCode::WorkerCrashed),
        10,
    );

    assert_eq!(
        store.result("op-1", 20).map_err(|error| error.code()),
        Err(LocalMediaErrorCode::WorkerCrashed)
    );
    assert_eq!(store.phase("op-1"), Some(LocalMediaPhase::Failed));
}

#[test]
fn a_cancelled_operation_is_cancelled_not_failed() {
    let store = store();
    store.accept("op-1", LocalMediaOperationKind::Stt, 0);
    store.cancel("op-1", 10);
    assert_eq!(
        store.result("op-1", 20).map_err(|error| error.code()),
        Err(LocalMediaErrorCode::OperationCancelled)
    );
    assert_eq!(store.phase("op-1"), Some(LocalMediaPhase::Cancelled));
}

#[test]
fn a_result_committed_after_cancellation_is_discarded() {
    // The cancel/succeed race: the worker may finish while the cancel is in flight. The user asked
    // for it to stop, so the transcript must not appear in their draft afterwards.
    let store = store();
    store.accept("op-1", LocalMediaOperationKind::Stt, 0);
    store.cancel("op-1", 10);
    store.succeed("op-1", playback(100), 11);

    assert_eq!(store.phase("op-1"), Some(LocalMediaPhase::Cancelled));
    assert_eq!(
        store.result("op-1", 12).map_err(|error| error.code()),
        Err(LocalMediaErrorCode::OperationCancelled)
    );
}

#[test]
fn phases_advance_until_a_terminal_state_locks_them() {
    let store = store();
    store.accept("op-1", LocalMediaOperationKind::Ocr, 0);
    store.set_phase("op-1", LocalMediaPhase::Queued);
    assert_eq!(store.phase("op-1"), Some(LocalMediaPhase::Queued));
    store.set_phase("op-1", LocalMediaPhase::Processing);
    assert_eq!(store.phase("op-1"), Some(LocalMediaPhase::Processing));

    store.succeed("op-1", playback(1), 5);
    store.set_phase("op-1", LocalMediaPhase::Processing);
    assert_eq!(
        store.phase("op-1"),
        Some(LocalMediaPhase::Succeeded),
        "a terminal phase must not be walked backwards"
    );
}

#[test]
fn the_store_evicts_the_oldest_entries_past_its_capacity() {
    // Bounded by construction: an unbounded map keyed on operation id is a leak with a user-facing
    // trigger, since every composer action creates one.
    let store = LocalMediaOperationStore::new(60_000, 2);
    for index in 0..5 {
        let id = format!("op-{index}");
        store.accept(&id, LocalMediaOperationKind::Tts, index as u64);
        store.succeed(&id, playback(index as u64), index as u64);
    }
    assert_eq!(store.len(), 2);
    assert_eq!(store.result("op-0", 5), Ok(None));
    assert!(store.result("op-4", 5).expect("newest retained").is_some());
}

#[test]
fn eviction_prefers_terminal_entries_over_running_ones() {
    let store = LocalMediaOperationStore::new(60_000, 2);
    store.accept("running", LocalMediaOperationKind::Stt, 0);
    store.accept("done-1", LocalMediaOperationKind::Tts, 1);
    store.succeed("done-1", playback(1), 1);
    store.accept("done-2", LocalMediaOperationKind::Tts, 2);
    store.succeed("done-2", playback(2), 2);

    // Dropping the in-flight operation would strand the composer polling an id that vanished.
    assert_eq!(store.phase("running"), Some(LocalMediaPhase::Accepted));
}
