//! What an extension runtime may say about itself: a host code, ids, and numbers.

use super::{
    all_diagnostic_rejections, DiagnosticMeasure, DiagnosticRejection, ExtensionId,
    RuntimeDiagnosticCode, SafeExtensionRuntimeDiagnostic, SnapshotId, ALL_DIAGNOSTIC_MEASURES,
    ALL_RUNTIME_DIAGNOSTIC_CODES, MAX_DIAGNOSTIC_MEASUREMENTS,
};

fn extension() -> ExtensionId {
    ExtensionId::parse("acme.git-guardian").expect("extension")
}

fn snapshot() -> SnapshotId {
    SnapshotId::parse("snap-a").expect("snapshot")
}

#[test]
fn a_diagnostic_carries_a_code_ids_and_numbers_and_nothing_else() {
    // The structural guarantee. Exhaustive destructuring of the *inputs* is not possible here
    // because the type's fields are private, so this asserts the accessors are the whole surface:
    // a code, an extension, an optional snapshot, and integer measurements. There is no accessor
    // that returns text an extension chose, because there is no such field to return.
    let diagnostic = SafeExtensionRuntimeDiagnostic::admit(
        RuntimeDiagnosticCode::TimedOut,
        extension(),
        Some(snapshot()),
        &[
            (DiagnosticMeasure::DurationMs, 5_000),
            (DiagnosticMeasure::BudgetMs, 3_000),
        ],
    )
    .expect("admit");

    assert_eq!(diagnostic.code(), RuntimeDiagnosticCode::TimedOut);
    assert_eq!(diagnostic.extension(), &extension());
    assert_eq!(diagnostic.snapshot(), Some(&snapshot()));
    assert_eq!(
        diagnostic.measurements(),
        &[
            (DiagnosticMeasure::DurationMs, 5_000),
            (DiagnosticMeasure::BudgetMs, 3_000),
        ]
    );
}

#[test]
fn measurements_are_ordered_so_two_diagnostics_of_one_run_render_identically() {
    let forward = SafeExtensionRuntimeDiagnostic::admit(
        RuntimeDiagnosticCode::StoppedCleanly,
        extension(),
        None,
        &[
            (DiagnosticMeasure::FuelUsed, 10),
            (DiagnosticMeasure::DurationMs, 20),
        ],
    )
    .expect("admit");
    let reversed = SafeExtensionRuntimeDiagnostic::admit(
        RuntimeDiagnosticCode::StoppedCleanly,
        extension(),
        None,
        &[
            (DiagnosticMeasure::DurationMs, 20),
            (DiagnosticMeasure::FuelUsed, 10),
        ],
    )
    .expect("admit");

    assert_eq!(forward, reversed);
}

#[test]
fn the_same_measure_twice_is_refused_rather_than_resolved() {
    // Two values for one measure has no meaning, and picking either is a guess about which one the
    // runtime meant.
    let error = SafeExtensionRuntimeDiagnostic::admit(
        RuntimeDiagnosticCode::Started,
        extension(),
        None,
        &[
            (DiagnosticMeasure::DurationMs, 1),
            (DiagnosticMeasure::DurationMs, 2),
        ],
    )
    .expect_err("duplicate measure");

    assert_eq!(error.code(), "diagnostic_duplicate_measure");
}

#[test]
fn a_negative_measurement_is_refused() {
    // Every measure here is a count, a duration, or a size.
    let error = SafeExtensionRuntimeDiagnostic::admit(
        RuntimeDiagnosticCode::Started,
        extension(),
        None,
        &[(DiagnosticMeasure::PeakMemoryBytes, -1)],
    )
    .expect_err("negative");

    assert_eq!(error.code(), "diagnostic_negative_measurement");
}

#[test]
fn more_measurements_than_there_are_measures_is_refused() {
    let repeated: Vec<(DiagnosticMeasure, i64)> = (0..MAX_DIAGNOSTIC_MEASUREMENTS + 1)
        .map(|_| (DiagnosticMeasure::DurationMs, 1))
        .collect();

    let error = SafeExtensionRuntimeDiagnostic::admit(
        RuntimeDiagnosticCode::Started,
        extension(),
        None,
        &repeated,
    )
    .expect_err("too many");

    assert_eq!(error.code(), "diagnostic_too_many_measurements");
}

#[test]
fn every_code_round_trips_and_one_this_build_does_not_know_is_refused() {
    for code in ALL_RUNTIME_DIAGNOSTIC_CODES.iter().copied() {
        assert_eq!(RuntimeDiagnosticCode::parse(code.as_str()), Some(code));
    }
    // A runtime cannot invent a code: an unknown one does not parse, so it cannot be logged as a
    // diagnostic at all.
    assert_eq!(
        RuntimeDiagnosticCode::parse("runtime_says_whatever_it_likes"),
        None
    );
    assert_eq!(
        RuntimeDiagnosticCode::parse("failed to open C:\\Users\\alice\\secret.txt"),
        None
    );
}

#[test]
fn every_measure_round_trips_and_a_free_form_measure_name_is_refused() {
    for measure in ALL_DIAGNOSTIC_MEASURES.iter().copied() {
        assert_eq!(DiagnosticMeasure::parse(measure.as_str()), Some(measure));
    }
    // A free-form measure name would be a string field by another route.
    assert_eq!(DiagnosticMeasure::parse("stdout"), None);
    assert_eq!(DiagnosticMeasure::parse("last_error"), None);
}

#[test]
fn every_spelling_is_distinct_and_lower_snake_case() {
    let mut spellings: Vec<&str> = ALL_RUNTIME_DIAGNOSTIC_CODES
        .iter()
        .map(|code| code.as_str())
        .chain(
            ALL_DIAGNOSTIC_MEASURES
                .iter()
                .map(|measure| measure.as_str()),
        )
        .collect();
    let total = spellings.len();
    spellings.sort_unstable();
    spellings.dedup();
    assert_eq!(spellings.len(), total);

    for spelling in spellings {
        assert!(spelling
            .chars()
            .all(|character| character.is_ascii_lowercase() || character == '_'));
    }
}

#[test]
fn only_the_outcomes_an_operator_should_see_count_as_failures() {
    assert!(!RuntimeDiagnosticCode::Started.is_failure());
    assert!(!RuntimeDiagnosticCode::StoppedCleanly.is_failure());
    for failure in [
        RuntimeDiagnosticCode::TimedOut,
        RuntimeDiagnosticCode::MemoryExhausted,
        RuntimeDiagnosticCode::FuelExhausted,
        RuntimeDiagnosticCode::Trapped,
        RuntimeDiagnosticCode::CapabilityRefused,
        RuntimeDiagnosticCode::GateClosed,
        RuntimeDiagnosticCode::InstantiationFailed,
    ] {
        assert!(failure.is_failure(), "{failure:?}");
    }
}

#[test]
fn every_rejection_has_a_distinct_stable_code() {
    let rejections = all_diagnostic_rejections();
    let total = rejections.len();

    let mut codes: Vec<&str> = rejections.iter().map(DiagnosticRejection::code).collect();
    codes.sort_unstable();
    codes.dedup();

    assert_eq!(codes.len(), total);
}
