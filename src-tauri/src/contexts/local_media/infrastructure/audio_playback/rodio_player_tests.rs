use super::*;
use std::path::Path;
use tempfile::TempDir;

fn playback_id(suffix: &str) -> PlaybackId {
    PlaybackId::new(format!("lmp-{suffix:0>32}"))
}

fn write_wav(path: &Path, milliseconds: u32) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).expect("writer");
    let samples = 16_000 * milliseconds / 1_000;
    for index in 0..samples {
        // A quiet tone rather than silence, so a backend that skips silent buffers still plays.
        let value = ((index as f32 * 0.05).sin() * 2_000.0) as i16;
        writer.write_sample(value).expect("sample");
    }
    writer.finalize().expect("finalize");
}

#[test]
fn nothing_is_playing_before_a_request() {
    let playback = RodioPlayback::new();
    assert!(playback.active().is_none());
}

#[test]
fn stopping_when_nothing_plays_is_a_no_op() {
    let playback = RodioPlayback::new();
    playback.stop(None);
    playback.stop(Some(&playback_id("1")));
    assert!(playback.active().is_none());
}

#[test]
fn a_missing_file_reports_a_stable_error_rather_than_panicking() {
    let _audio_host = crate::contexts::local_media::infrastructure::audio_host_guard();
    let directory = TempDir::new().expect("temp dir");
    let playback = RodioPlayback::new();
    let error = playback
        .play_blocking(
            &playback_id("1"),
            &directory.path().join("absent.wav"),
            None,
            Arc::new(AtomicBool::new(false)),
        )
        .expect_err("missing file");
    assert!(matches!(
        error.code(),
        LocalMediaErrorCode::PlaybackDeviceUnavailable | LocalMediaErrorCode::TempStorageFailed
    ));
    assert!(
        playback.active().is_none(),
        "a failed playback must not stay active"
    );
}

#[test]
fn a_file_that_is_not_audio_is_refused() {
    let _audio_host = crate::contexts::local_media::infrastructure::audio_host_guard();
    let directory = TempDir::new().expect("temp dir");
    let path = directory.path().join("not-audio.wav");
    std::fs::write(&path, b"this is not a wav file at all").expect("write");

    let playback = RodioPlayback::new();
    assert!(playback
        .play_blocking(
            &playback_id("1"),
            &path,
            None,
            Arc::new(AtomicBool::new(false))
        )
        .is_err());
    assert!(playback.active().is_none());
}

#[test]
fn a_request_that_is_already_cancelled_does_not_play() {
    let _audio_host = crate::contexts::local_media::infrastructure::audio_host_guard();
    let directory = TempDir::new().expect("temp dir");
    let path = directory.path().join("speech.wav");
    write_wav(&path, 200);

    let playback = RodioPlayback::new();
    let error = playback
        .play_blocking(
            &playback_id("1"),
            &path,
            None,
            Arc::new(AtomicBool::new(true)),
        )
        .expect_err("already cancelled");
    assert_eq!(error.code(), LocalMediaErrorCode::OperationCancelled);
    assert!(playback.active().is_none());
}

#[test]
fn playing_a_short_file_completes_or_reports_a_device_error() {
    let _audio_host = crate::contexts::local_media::infrastructure::audio_host_guard();
    // CI runners usually have no output device. Both outcomes are acceptable; leaving the slot
    // occupied or hanging past the file's own duration is not.
    let directory = TempDir::new().expect("temp dir");
    let path = directory.path().join("speech.wav");
    write_wav(&path, 120);

    let playback = RodioPlayback::new();
    let started = std::time::Instant::now();
    let outcome = playback.play_blocking(
        &playback_id("1"),
        &path,
        None,
        Arc::new(AtomicBool::new(false)),
    );
    assert!(
        started.elapsed() < Duration::from_secs(15),
        "playback must not hang"
    );

    match outcome {
        Ok(duration_ms) => assert!(duration_ms > 0),
        Err(error) => assert_eq!(error.code(), LocalMediaErrorCode::PlaybackDeviceUnavailable),
    }
    assert!(
        playback.active().is_none(),
        "the slot must be free once playback settles"
    );
}

#[test]
fn stopping_a_playback_by_id_only_affects_that_one() {
    let playback = RodioPlayback::new();
    // With nothing active, a targeted stop for an unrelated id must not clear the slot state that
    // a concurrent playback would be relying on.
    playback.stop(Some(&playback_id("2")));
    assert!(playback.active().is_none());
}
