use super::*;

const HEX32: &str = "0123456789abcdef0123456789abcdef";

fn operation_id() -> LocalMediaOperationId {
    LocalMediaOperationId::new(format!("{}{HEX32}", LocalMediaOperationId::PREFIX))
}

fn profile() -> LocalMediaProfile {
    let mut profile = LocalMediaProfile::disabled_default("2026-01-01T00:00:00Z".to_string());
    profile.revision = 7;
    profile.enabled = true;
    profile.ocr.enabled = true;
    profile.ocr.python_executable = "python-a".to_string();
    profile.ocr.max_pdf_pages = 12;
    profile.stt.python_executable = "python-b".to_string();
    profile.tts.python_executable = "python-c".to_string();
    profile
}

fn snapshot(engine: LocalMediaEngine, source: &LocalMediaProfile) -> LocalMediaProfileSnapshot {
    LocalMediaProfileSnapshot::capture(
        operation_id(),
        LocalMediaOperationKind::Ocr,
        engine,
        source,
        Some(ComposerScopeId::new("session-1")),
        "2026-01-01T00:00:01Z".to_string(),
    )
}

#[test]
fn operation_kinds_use_the_context_prefix_and_round_trip() {
    assert_eq!(LocalMediaOperationKind::Probe.as_str(), "local-media.probe");
    assert_eq!(LocalMediaOperationKind::Ocr.as_str(), "local-media.ocr");
    assert_eq!(LocalMediaOperationKind::Stt.as_str(), "local-media.stt");
    assert_eq!(LocalMediaOperationKind::Tts.as_str(), "local-media.tts");
    for kind in LocalMediaOperationKind::ALL {
        assert_eq!(LocalMediaOperationKind::parse(kind.as_str()), Some(kind));
    }
    assert_eq!(LocalMediaOperationKind::parse("local-media.unknown"), None);
    assert_eq!(LocalMediaOperationKind::parse("ocr"), None);
}

#[test]
fn operation_labels_are_locale_keys_not_content() {
    for kind in LocalMediaOperationKind::ALL {
        assert!(kind.message_key().starts_with("localMedia.operations."));
    }
}

#[test]
fn only_the_three_settled_phases_are_terminal() {
    for phase in [
        LocalMediaPhase::Succeeded,
        LocalMediaPhase::Failed,
        LocalMediaPhase::Cancelled,
    ] {
        assert!(phase.is_terminal(), "{phase:?}");
    }
    for phase in [
        LocalMediaPhase::Accepted,
        LocalMediaPhase::FinalizingRecording,
        LocalMediaPhase::Queued,
        LocalMediaPhase::LoadingEngine,
        LocalMediaPhase::Processing,
        LocalMediaPhase::GeneratingAudio,
        // Playback is still cancellable, so it is not terminal.
        LocalMediaPhase::Playing,
    ] {
        assert!(!phase.is_terminal(), "{phase:?}");
    }
}

#[test]
fn phase_wire_values_are_kebab_case_and_unique() {
    let phases = [
        LocalMediaPhase::Accepted,
        LocalMediaPhase::FinalizingRecording,
        LocalMediaPhase::Queued,
        LocalMediaPhase::LoadingEngine,
        LocalMediaPhase::Processing,
        LocalMediaPhase::GeneratingAudio,
        LocalMediaPhase::Playing,
        LocalMediaPhase::Succeeded,
        LocalMediaPhase::Failed,
        LocalMediaPhase::Cancelled,
    ];
    let unique: std::collections::BTreeSet<&str> =
        phases.iter().map(|phase| phase.as_str()).collect();
    assert_eq!(unique.len(), phases.len());
    assert_eq!(
        LocalMediaPhase::FinalizingRecording.as_str(),
        "finalizing-recording"
    );
    let json = serde_json::to_value(LocalMediaPhase::GeneratingAudio).expect("serialize");
    assert_eq!(json, serde_json::json!("generating-audio"));
}

#[test]
fn a_profile_can_narrow_the_page_limit_but_not_raise_it() {
    let mut source = profile();
    source.ocr.max_pdf_pages = 12;
    assert_eq!(AdmissionLimits::for_ocr(&source.ocr).max_pdf_pages, 12);

    source.ocr.max_pdf_pages = 5_000;
    assert_eq!(
        AdmissionLimits::for_ocr(&source.ocr).max_pdf_pages,
        AdmissionLimits::HARD_CEILING.max_pdf_pages
    );

    source.ocr.max_pdf_pages = 0;
    assert_eq!(AdmissionLimits::for_ocr(&source.ocr).max_pdf_pages, 1);
}

#[test]
fn narrowing_pages_leaves_the_other_ceilings_untouched() {
    let mut source = profile();
    source.ocr.max_pdf_pages = 3;
    let limits = AdmissionLimits::for_ocr(&source.ocr);
    assert_eq!(
        limits.max_input_bytes,
        AdmissionLimits::HARD_CEILING.max_input_bytes
    );
    assert_eq!(
        limits.max_decoded_pixels,
        AdmissionLimits::HARD_CEILING.max_decoded_pixels
    );
    assert_eq!(
        limits.max_output_characters,
        AdmissionLimits::HARD_CEILING.max_output_characters
    );
}

#[test]
fn a_snapshot_does_not_observe_later_profile_edits() {
    let mut source = profile();
    let captured = snapshot(LocalMediaEngine::Ocr, &source);

    source.revision = 99;
    source.ocr.python_executable = "python-replaced".to_string();
    source.ocr.max_pdf_pages = 2;
    source.stt.model_directory = "elsewhere".to_string();

    assert_eq!(captured.profile_revision(), 7);
    assert_eq!(captured.python_executable(), "python-a");
    assert_eq!(captured.limits().max_pdf_pages, 12);
    assert!(captured.stt().model_directory.is_empty());
}

#[test]
fn the_snapshot_resolves_the_executable_of_its_own_engine() {
    let source = profile();
    assert_eq!(
        snapshot(LocalMediaEngine::Ocr, &source).python_executable(),
        "python-a"
    );
    assert_eq!(
        snapshot(LocalMediaEngine::Stt, &source).python_executable(),
        "python-b"
    );
    assert_eq!(
        snapshot(LocalMediaEngine::Tts, &source).python_executable(),
        "python-c"
    );
}

#[test]
fn the_snapshot_retains_its_composer_scope_and_identity() {
    let source = profile();
    let captured = snapshot(LocalMediaEngine::Stt, &source);
    assert_eq!(captured.operation_id(), &operation_id());
    assert_eq!(captured.kind(), LocalMediaOperationKind::Ocr);
    assert_eq!(captured.engine(), LocalMediaEngine::Stt);
    assert_eq!(
        captured.composer_scope().map(ComposerScopeId::as_str),
        Some("session-1")
    );
    assert_eq!(captured.created_at(), "2026-01-01T00:00:01Z");
}

#[test]
fn a_probe_snapshot_has_no_composer_scope() {
    let source = profile();
    let captured = LocalMediaProfileSnapshot::capture(
        operation_id(),
        LocalMediaOperationKind::Probe,
        LocalMediaEngine::Tts,
        &source,
        None,
        "2026-01-01T00:00:02Z".to_string(),
    );
    assert!(captured.composer_scope().is_none());
}
