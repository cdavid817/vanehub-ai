use super::*;

fn recording_id() -> RecordingId {
    RecordingId::new("lmr-0123456789abcdef0123456789abcdef")
}

#[test]
fn a_hold_shorter_than_the_floor_is_rejected_without_transcribing() {
    assert_eq!(
        RecordingOutcome::evaluate(0, 120_000),
        RecordingOutcome::TooShort
    );
    assert_eq!(
        RecordingOutcome::evaluate(MIN_RECORDING_MILLIS - 1, 120_000),
        RecordingOutcome::TooShort
    );
}

#[test]
fn the_floor_itself_is_accepted() {
    // Exactly 300 ms is a valid hold; the boundary belongs to the user, not to the guard.
    assert_eq!(
        RecordingOutcome::evaluate(MIN_RECORDING_MILLIS, 120_000),
        RecordingOutcome::Committed {
            limit_reached: false
        }
    );
}

#[test]
fn reaching_the_maximum_still_commits_the_utterance() {
    // Auto-stop must not discard what was captured; the limit is reported as a warning alongside a
    // successful transcription, not as a failure.
    assert_eq!(
        RecordingOutcome::evaluate(120_000, 120_000),
        RecordingOutcome::Committed {
            limit_reached: true
        }
    );
    assert_eq!(
        RecordingOutcome::evaluate(120_500, 120_000),
        RecordingOutcome::Committed {
            limit_reached: true
        }
    );
}

#[test]
fn an_ordinary_hold_reports_no_limit_warning() {
    assert_eq!(
        RecordingOutcome::evaluate(6_400, 120_000),
        RecordingOutcome::Committed {
            limit_reached: false
        }
    );
}

#[test]
fn duration_is_derived_from_sample_count_and_rate() {
    assert_eq!(duration_ms_for(16_000, 16_000), 1_000);
    assert_eq!(duration_ms_for(8_000, 16_000), 500);
    assert_eq!(duration_ms_for(0, 16_000), 0);
    // A zero sample rate is a device bug, not a divide-by-zero.
    assert_eq!(duration_ms_for(1_000, 0), 0);
}

#[test]
fn a_handle_reports_its_own_ceiling() {
    let handle = RecordingHandle {
        recording_id: recording_id(),
        started_at: "2026-01-01T00:00:00Z".to_string(),
        max_duration_ms: 45_000,
    };
    let json = serde_json::to_value(&handle).expect("serialize handle");
    assert_eq!(json["maxDurationMs"], 45_000);
    assert_eq!(json["recordingId"], recording_id().as_str());
    assert!(json.get("path").is_none());
}

#[test]
fn a_committed_recording_exposes_counts_but_no_path() {
    let committed = CommittedRecording {
        recording_id: recording_id(),
        duration_ms: 6_400,
        sample_rate: 16_000,
        sample_count: 102_400,
        limit_reached: false,
    };
    let json = serde_json::to_value(&committed).expect("serialize");
    let keys: std::collections::BTreeSet<&str> = json
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        [
            "durationMs",
            "limitReached",
            "recordingId",
            "sampleCount",
            "sampleRate"
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn a_recording_is_owned_by_one_composer_scope() {
    let summary = RecordingSummary {
        recording_id: recording_id(),
        composer_scope: ComposerScopeId::new("session-1"),
        started_at_ms: 10,
        max_duration_ms: 120_000,
    };
    assert!(summary.owned_by(&recording_id(), &ComposerScopeId::new("session-1")));
    // Guessing the recording id is not enough; the caller must also own the scope that started it.
    assert!(!summary.owned_by(&recording_id(), &ComposerScopeId::new("session-2")));
    assert!(!summary.owned_by(
        &RecordingId::new("lmr-ffffffffffffffffffffffffffffffff"),
        &ComposerScopeId::new("session-1")
    ));
}
