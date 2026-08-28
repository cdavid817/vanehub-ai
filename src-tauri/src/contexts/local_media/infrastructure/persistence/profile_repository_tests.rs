use super::*;
use crate::contexts::local_media::domain::{LocalMediaErrorCode, TtsModelKind};
use crate::platform::database::NativeDatabase;
use tempfile::TempDir;

struct StubClock;

impl LocalMediaClock for StubClock {
    fn now_iso(&self) -> String {
        "2026-02-02T03:04:05Z".to_string()
    }

    fn now_ms(&self) -> u64 {
        1_770_000_000_000
    }
}

fn repository() -> (TempDir, SqliteLocalMediaProfileRepository) {
    let directory = TempDir::new().expect("temp dir");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let repository = SqliteLocalMediaProfileRepository::new(database, Arc::new(StubClock));
    (directory, repository)
}

#[test]
fn the_first_read_creates_and_returns_disabled_defaults() {
    let (_directory, repository) = repository();
    let profile = repository.load().expect("first load");

    assert_eq!(profile.profile_id, DEFAULT_PROFILE_ID);
    assert_eq!(profile.revision, 0);
    assert!(!profile.enabled);
    assert!(!profile.ocr.enabled);
    assert!(!profile.stt.enabled);
    assert!(!profile.tts.enabled);
    assert_eq!(profile.updated_at, "2026-02-02T03:04:05Z");
}

#[test]
fn the_defaults_are_persisted_so_a_second_read_is_identical() {
    let (_directory, repository) = repository();
    let first = repository.load().expect("first load");
    let second = repository.load().expect("second load");
    assert_eq!(first, second);
}

#[test]
fn a_save_round_trips_every_engine_field() {
    let (_directory, repository) = repository();
    let mut profile = repository.load().expect("load");
    profile.enabled = true;
    profile.ocr.enabled = true;
    profile.ocr.python_executable = "/usr/bin/python3".to_string();
    profile.ocr.text_detection_model_dir = Some("/models/det".to_string());
    profile.ocr.max_pdf_pages = 7;
    profile.stt.beam_size = 3;
    profile.stt.vad_filter = false;
    profile.stt.microphone_device_id = Some("input-2".to_string());
    profile.tts.model_kind = TtsModelKind::Kokoro;
    profile.tts.voices_path = Some("/models/voices.bin".to_string());
    profile.tts.rule_fsts = vec!["/models/a.fst".to_string(), "/models/b.fst".to_string()];
    profile.tts.speed = 1.25;

    let saved = repository.save(&profile, 0).expect("save");
    assert_eq!(saved.revision, 1);

    let reloaded = repository.load().expect("reload");
    assert_eq!(reloaded, saved);
    assert_eq!(reloaded.ocr.max_pdf_pages, 7);
    assert_eq!(reloaded.stt.beam_size, 3);
    assert!(!reloaded.stt.vad_filter);
    assert_eq!(reloaded.tts.model_kind, TtsModelKind::Kokoro);
    assert_eq!(reloaded.tts.rule_fsts.len(), 2);
    assert!((reloaded.tts.speed - 1.25).abs() < f32::EPSILON);
}

#[test]
fn each_save_advances_the_revision_by_one() {
    let (_directory, repository) = repository();
    let profile = repository.load().expect("load");
    let first = repository.save(&profile, 0).expect("first save");
    let second = repository.save(&first, 1).expect("second save");
    let third = repository.save(&second, 2).expect("third save");
    assert_eq!((first.revision, second.revision, third.revision), (1, 2, 3));
}

#[test]
fn a_stale_revision_conflicts_and_changes_nothing() {
    let (_directory, repository) = repository();
    let mut profile = repository.load().expect("load");
    profile.ocr.language = "en".to_string();
    repository.save(&profile, 0).expect("first save");

    let mut stale = profile.clone();
    stale.ocr.language = "ja".to_string();
    let error = repository
        .save(&stale, 0)
        .expect_err("stale save must conflict");
    assert_eq!(error.code(), LocalMediaErrorCode::ProfileRevisionConflict);

    let stored = repository.load().expect("reload");
    assert_eq!(stored.ocr.language, "en");
    assert_eq!(stored.revision, 1);
}

#[test]
fn a_future_revision_also_conflicts() {
    // Not just "older than stored": any mismatch is a conflict, so a client that invented a
    // revision cannot force a write.
    let (_directory, repository) = repository();
    let profile = repository.load().expect("load");
    let error = repository
        .save(&profile, 99)
        .expect_err("mismatched revision");
    assert_eq!(error.code(), LocalMediaErrorCode::ProfileRevisionConflict);
}

#[test]
fn the_save_writes_its_own_timestamp_and_profile_id() {
    let (_directory, repository) = repository();
    let mut profile = repository.load().expect("load");
    profile.updated_at = "1999-01-01T00:00:00Z".to_string();
    profile.profile_id = "default".to_string();
    let saved = repository.save(&profile, 0).expect("save");
    assert_eq!(saved.updated_at, "2026-02-02T03:04:05Z");
    assert_eq!(saved.profile_id, DEFAULT_PROFILE_ID);
}

#[test]
fn malformed_stored_json_falls_back_to_defaults_instead_of_failing_the_read() {
    // A row written by a build with a different shape must not make the settings page unopenable.
    // Falling back to a disabled default is recoverable; a hard error is not.
    let (_directory, repository) = repository();
    repository.load().expect("seed");
    {
        let connection = repository.database.connection().expect("connection");
        connection
            .execute(
                "UPDATE local_media_profiles SET stt_config_json = '{ not json ' WHERE profile_id = 'default'",
                [],
            )
            .expect("corrupt the row");
    }

    let profile = repository.load().expect("load must still succeed");
    assert!(!profile.stt.enabled);
    assert_eq!(profile.stt.beam_size, 5);
}

#[test]
fn unknown_future_fields_in_stored_json_are_tolerated() {
    let (_directory, repository) = repository();
    repository.load().expect("seed");
    {
        let connection = repository.database.connection().expect("connection");
        connection
            .execute(
                r#"UPDATE local_media_profiles
                   SET ocr_config_json = '{"enabled":true,"language":"en","futureField":42}'
                   WHERE profile_id = 'default'"#,
                [],
            )
            .expect("write forward-compatible row");
    }

    let profile = repository.load().expect("load");
    assert!(profile.ocr.enabled);
    assert_eq!(profile.ocr.language, "en");
    assert_eq!(
        profile.ocr.max_pdf_pages, 20,
        "absent fields still take their defaults"
    );
}

#[test]
fn the_table_refuses_a_second_profile_row() {
    let (_directory, repository) = repository();
    repository.load().expect("seed");
    let connection = repository.database.connection().expect("connection");
    let result = connection.execute(
        "INSERT INTO local_media_profiles VALUES ('secondary', 0, 0, '{}', '{}', '{}', 'now')",
        [],
    );
    assert!(
        result.is_err(),
        "the CHECK constraint must reject a second profile"
    );
}

#[test]
fn stored_json_never_contains_a_secret_classification_marker() {
    // The profile holds paths, not credentials. If a future field ever needs one it must go
    // through the credential store, and this test is what notices the day it does not.
    let (_directory, repository) = repository();
    let mut profile = repository.load().expect("load");
    profile.ocr.python_executable = "/usr/bin/python3".to_string();
    repository.save(&profile, 0).expect("save");

    let connection = repository.database.connection().expect("connection");
    let stored: String = connection
        .query_row(
            "SELECT ocr_config_json || stt_config_json || tts_config_json FROM local_media_profiles",
            [],
            |row| row.get(0),
        )
        .expect("read row");
    // Whole key names, not substrings: `tokensPath` is a sherpa-onnx model file and a naive
    // "token" match would flag it forever.
    for marker in [
        "\"password\"",
        "\"secret\"",
        "\"apikey\"",
        "\"credential\"",
        "\"accesstoken\"",
    ] {
        assert!(
            !stored.to_lowercase().contains(marker),
            "unexpected {marker} in stored profile"
        );
    }
}
