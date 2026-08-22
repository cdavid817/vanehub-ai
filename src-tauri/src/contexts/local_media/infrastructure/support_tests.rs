use super::*;
use crate::contexts::local_media::domain::{
    LocalMediaOperationId, PlaybackId, RecordingId, StagedInputId,
};
use tempfile::TempDir;

#[test]
fn generated_ids_match_the_shape_the_domain_will_parse() {
    let factory = RandomIdFactory;
    for prefix in [
        StagedInputId::PREFIX,
        RecordingId::PREFIX,
        PlaybackId::PREFIX,
        LocalMediaOperationId::PREFIX,
    ] {
        let id = factory.next(prefix);
        assert!(id.starts_with(prefix));
        assert_eq!(id.len(), prefix.len() + 32);
        assert!(id[prefix.len()..]
            .chars()
            .all(|character| character.is_ascii_hexdigit()));
    }
    assert!(StagedInputId::parse(&factory.next(StagedInputId::PREFIX)).is_some());
}

#[test]
fn generated_ids_do_not_repeat() {
    let factory = RandomIdFactory;
    let ids: std::collections::BTreeSet<String> = (0..256)
        .map(|_| factory.next(StagedInputId::PREFIX))
        .collect();
    assert_eq!(ids.len(), 256);
}

#[test]
fn the_clock_reports_an_iso_timestamp_and_monotonic_milliseconds() {
    let clock = SystemLocalMediaClock;
    let stamp = clock.now_iso();
    assert!(stamp.ends_with('Z'), "{stamp} is not a UTC timestamp");
    assert!(stamp.len() >= 20);
    let first = clock.now_ms();
    let second = clock.now_ms();
    assert!(second >= first);
    assert!(
        first > 1_700_000_000_000,
        "the clock must be a real wall clock"
    );
}

#[test]
fn the_bridge_resolver_takes_the_first_candidate_that_holds_the_package() {
    let directory = TempDir::new().expect("temp dir");
    let missing = directory.path().join("missing");
    let present = directory.path().join("present");
    std::fs::create_dir_all(present.join("vane_local_media_worker")).expect("mkdir");
    std::fs::write(
        present.join("vane_local_media_worker").join("__main__.py"),
        b"x",
    )
    .expect("entry point");

    assert_eq!(
        resolve_worker_bridge_root(&[missing.clone(), present.clone()]),
        Some(present)
    );
}

#[test]
fn a_directory_without_the_entry_point_is_not_accepted() {
    // An empty `local-media-worker` directory in a packaged bundle is worse than none: it would
    // make every worker launch fail with an import error instead of an honest "not packaged".
    let directory = TempDir::new().expect("temp dir");
    let shell = directory.path().join("shell");
    std::fs::create_dir_all(shell.join("vane_local_media_worker")).expect("mkdir");
    assert_eq!(resolve_worker_bridge_root(&[shell]), None);
}

#[test]
fn no_candidates_resolve_to_nothing() {
    assert_eq!(resolve_worker_bridge_root(&[]), None);
    assert_eq!(
        resolve_worker_bridge_root(&[std::path::PathBuf::from("/definitely/not/here")]),
        None
    );
}

#[test]
fn diagnostics_render_only_the_fields_they_were_given() {
    let rendered = render_diagnostic_context(&[
        ("engine", "paddleocr".to_string()),
        ("pageCount", "3".to_string()),
    ]);
    assert_eq!(
        rendered.get("engine").map(String::as_str),
        Some("paddleocr")
    );
    assert_eq!(rendered.get("pageCount").map(String::as_str), Some("3"));
    assert_eq!(rendered.len(), 2);
}
