//! That a diagnostic reaches the unified log, and that it takes nothing with it.

use super::{rendered_measure_keys, LoggingRuntimeDiagnosticSink, RUNTIME_DIAGNOSTIC_CATEGORY};
use crate::contexts::tooling::extension_platform::application::RuntimeDiagnosticSink;
use crate::contexts::tooling::extension_platform::domain::{
    DiagnosticMeasure, ExtensionId, RuntimeDiagnosticCode, SafeExtensionRuntimeDiagnostic,
    SnapshotId,
};
use crate::platform::logging::LOG_FILE_NAME;
use crate::test_support::TempDirectory;

fn extension() -> ExtensionId {
    ExtensionId::parse("acme.git-guardian").expect("extension")
}

fn written(directory: &TempDirectory) -> String {
    std::fs::read_to_string(directory.path().join(LOG_FILE_NAME)).expect("log file")
}

#[test]
fn a_diagnostic_is_written_under_its_own_category_with_the_code_as_the_message() {
    let directory = TempDirectory::new("runtime-diagnostic-emit");
    let sink = LoggingRuntimeDiagnosticSink::new(directory.path().to_path_buf());
    let diagnostic = SafeExtensionRuntimeDiagnostic::admit(
        RuntimeDiagnosticCode::TimedOut,
        extension(),
        Some(SnapshotId::parse("snap-a").expect("snapshot")),
        &[
            (DiagnosticMeasure::DurationMs, 5_000),
            (DiagnosticMeasure::BudgetMs, 3_000),
        ],
    )
    .expect("admit");

    sink.emit(&diagnostic).expect("emit");

    let line = written(&directory);
    assert!(line.contains(RUNTIME_DIAGNOSTIC_CATEGORY), "{line}");
    assert!(line.contains("runtime_timed_out"), "{line}");
    assert!(line.contains("acme.git-guardian"), "{line}");
    assert!(line.contains("snap-a"), "{line}");
    assert!(line.contains("5000") && line.contains("3000"), "{line}");
}

#[test]
fn nothing_an_extension_chose_can_appear_in_the_line() {
    // The guarantee is structural rather than filtered: the diagnostic type has no field that
    // could carry a path, a payload, an environment, or captured output, so there is nothing for a
    // redactor to miss. This asserts the rendered line contains only what the host put there.
    let directory = TempDirectory::new("runtime-diagnostic-contents");
    let sink = LoggingRuntimeDiagnosticSink::new(directory.path().to_path_buf());
    let diagnostic = SafeExtensionRuntimeDiagnostic::admit(
        RuntimeDiagnosticCode::Trapped,
        extension(),
        None,
        &[(DiagnosticMeasure::FuelUsed, 42)],
    )
    .expect("admit");

    sink.emit(&diagnostic).expect("emit");

    let line = written(&directory);
    for host_owned in ["runtime_trapped", "acme.git-guardian", "fuel_used", "42"] {
        assert!(line.contains(host_owned), "missing {host_owned}: {line}");
    }
    // Nothing resembling a path, a stream capture, or an environment can be present, because no
    // constructor accepts one.
    for absent in ["C:\\", "/home/", "stdout", "stderr", "PATH="] {
        assert!(!line.contains(absent), "unexpected {absent}: {line}");
    }
}

#[test]
fn a_success_is_written_at_a_quieter_level_than_a_failure() {
    // An operator watching a log at default level should see the times an extension was stopped,
    // not the times one started.
    let directory = TempDirectory::new("runtime-diagnostic-levels");
    let sink = LoggingRuntimeDiagnosticSink::new(directory.path().to_path_buf());

    for code in [
        RuntimeDiagnosticCode::Started,
        RuntimeDiagnosticCode::MemoryExhausted,
    ] {
        sink.emit(
            &SafeExtensionRuntimeDiagnostic::admit(code, extension(), None, &[]).expect("admit"),
        )
        .expect("emit");
    }

    let log = written(&directory);
    let started = log
        .lines()
        .find(|line| line.contains("runtime_started"))
        .expect("started line");
    let exhausted = log
        .lines()
        .find(|line| line.contains("runtime_memory_exhausted"))
        .expect("exhausted line");

    assert!(started.to_lowercase().contains("debug"), "{started}");
    assert!(exhausted.to_lowercase().contains("warn"), "{exhausted}");
}

#[test]
fn the_rendered_keys_are_exactly_the_measures_the_domain_defines() {
    // So a reader of a log line can be told which keys are possible without going hunting, and so
    // adding a measure without deciding how it renders is caught here.
    assert_eq!(
        rendered_measure_keys(),
        vec![
            "budget_ms",
            "duration_ms",
            "fuel_used",
            "host_call_count",
            "peak_memory_bytes",
        ]
    );
}
