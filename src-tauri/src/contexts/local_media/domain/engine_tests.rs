use super::*;

fn status(engine: LocalMediaEngine, readiness: EngineReadiness) -> EngineStatus {
    EngineStatus {
        engine,
        readiness,
        profile_revision: 3,
        worker_state: WorkerState::Idle,
        installed_version: None,
        model_identity: None,
        device_summary: None,
        last_checked_at: None,
    }
}

fn runtime(engines: Vec<EngineStatus>) -> LocalMediaRuntimeStatus {
    LocalMediaRuntimeStatus {
        native_available: true,
        platform_support: PlatformSupport::Supported,
        enabled: true,
        profile_revision: 3,
        engines,
        path_classifications: Vec::new(),
    }
}

#[test]
fn engine_ids_round_trip() {
    for engine in LocalMediaEngine::ALL {
        assert_eq!(LocalMediaEngine::parse(engine.as_str()), Some(engine));
    }
    assert_eq!(LocalMediaEngine::parse("paddleocr"), None);
    assert_eq!(LocalMediaEngine::parse(""), None);
}

#[test]
fn worker_ids_and_methods_are_distinct_per_engine() {
    let worker_ids: std::collections::BTreeSet<&str> = LocalMediaEngine::ALL
        .iter()
        .map(|engine| engine.worker_id())
        .collect();
    assert_eq!(worker_ids.len(), 3);
    assert_eq!(LocalMediaEngine::Ocr.worker_id(), "paddleocr");
    assert_eq!(LocalMediaEngine::Stt.worker_id(), "faster-whisper");
    assert_eq!(LocalMediaEngine::Tts.worker_id(), "sherpa-onnx");
    assert_eq!(LocalMediaEngine::Ocr.inference_method(), "ocr");
    assert_eq!(LocalMediaEngine::Stt.inference_method(), "transcribe");
    assert_eq!(LocalMediaEngine::Tts.inference_method(), "synthesize");
}

#[test]
fn only_ready_permits_an_operation() {
    assert!(EngineReadiness::Ready.permits_operation());
    for readiness in [
        EngineReadiness::Disabled,
        EngineReadiness::Unconfigured,
        EngineReadiness::Checking,
        EngineReadiness::RestartRequired,
        EngineReadiness::Unavailable {
            code: LocalMediaErrorCode::ModelNotFound,
            field: None,
        },
    ] {
        assert!(
            !readiness.permits_operation(),
            "{readiness:?} must not permit an operation"
        );
    }
}

#[test]
fn one_failed_engine_does_not_disable_the_others() {
    let runtime = runtime(vec![
        status(
            LocalMediaEngine::Ocr,
            EngineReadiness::Unavailable {
                code: LocalMediaErrorCode::EngineImportFailed,
                field: None,
            },
        ),
        status(LocalMediaEngine::Stt, EngineReadiness::Ready),
        status(LocalMediaEngine::Tts, EngineReadiness::Ready),
    ]);
    assert!(!runtime.permits(LocalMediaEngine::Ocr));
    assert!(runtime.permits(LocalMediaEngine::Stt));
    assert!(runtime.permits(LocalMediaEngine::Tts));
}

#[test]
fn web_mode_permits_nothing_even_with_ready_engines() {
    let mut web = runtime(vec![
        status(LocalMediaEngine::Ocr, EngineReadiness::Ready),
        status(LocalMediaEngine::Stt, EngineReadiness::Ready),
    ]);
    web.native_available = false;
    for engine in LocalMediaEngine::ALL {
        assert!(!web.permits(engine));
    }
}

#[test]
fn the_master_switch_gates_every_engine() {
    let mut disabled = runtime(vec![status(LocalMediaEngine::Tts, EngineReadiness::Ready)]);
    disabled.enabled = false;
    assert!(!disabled.permits(LocalMediaEngine::Tts));
}

#[test]
fn an_unsupported_platform_gates_every_engine() {
    let mut unsupported = runtime(vec![status(LocalMediaEngine::Stt, EngineReadiness::Ready)]);
    unsupported.platform_support = PlatformSupport::Unsupported;
    assert!(!unsupported.permits(LocalMediaEngine::Stt));
}

#[test]
fn an_absent_engine_status_permits_nothing() {
    let runtime = runtime(vec![status(LocalMediaEngine::Ocr, EngineReadiness::Ready)]);
    assert!(!runtime.permits(LocalMediaEngine::Tts));
    assert!(runtime.engine(LocalMediaEngine::Tts).is_none());
}

#[test]
fn platform_support_is_three_valued_not_boolean() {
    assert_eq!(
        PlatformSupport::for_target("windows", "x86_64"),
        PlatformSupport::Supported
    );
    assert_eq!(
        PlatformSupport::for_target("linux", "x86_64"),
        PlatformSupport::Supported
    );
    assert_eq!(
        PlatformSupport::for_target("macos", "aarch64"),
        PlatformSupport::Supported
    );
    assert_eq!(
        PlatformSupport::for_target("macos", "x86_64"),
        PlatformSupport::Experimental
    );
    assert_eq!(
        PlatformSupport::for_target("linux", "aarch64"),
        PlatformSupport::Experimental
    );
    assert_eq!(
        PlatformSupport::for_target("windows", "aarch64"),
        PlatformSupport::Unsupported
    );
    assert_eq!(
        PlatformSupport::for_target("android", "aarch64"),
        PlatformSupport::Unsupported
    );
    // Experimental still runs; only `Unsupported` is a refusal.
    assert!(PlatformSupport::Experimental.permits_operation());
    assert!(!PlatformSupport::Unsupported.permits_operation());
}

#[test]
fn the_current_platform_resolves_without_panicking() {
    let _ = PlatformSupport::current();
}

#[test]
fn worker_running_covers_only_idle_and_busy() {
    assert!(WorkerState::Idle.is_running());
    assert!(WorkerState::Busy.is_running());
    for state in [
        WorkerState::Stopped,
        WorkerState::Starting,
        WorkerState::Restarting,
        WorkerState::Quarantined,
    ] {
        assert!(!state.is_running());
    }
}

#[test]
fn readiness_serializes_as_a_tagged_state_with_a_stable_code() {
    let json = serde_json::to_value(EngineReadiness::Unavailable {
        code: LocalMediaErrorCode::PythonNotFound,
        field: None,
    })
    .expect("serialize readiness");
    assert_eq!(json["state"], "unavailable");
    assert_eq!(json["code"], "PYTHON_NOT_FOUND");

    let ready = serde_json::to_value(EngineReadiness::Ready).expect("serialize ready");
    assert_eq!(ready["state"], "ready");
}

#[test]
fn a_disabled_status_reports_no_metadata() {
    let disabled = EngineStatus::disabled(LocalMediaEngine::Ocr, 7);
    assert_eq!(disabled.readiness, EngineReadiness::Disabled);
    assert_eq!(disabled.worker_state, WorkerState::Stopped);
    assert_eq!(disabled.profile_revision, 7);
    assert!(disabled.installed_version.is_none());
    assert!(disabled.model_identity.is_none());
    assert!(disabled.device_summary.is_none());
}
