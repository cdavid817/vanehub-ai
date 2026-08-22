use super::*;
use tempfile::TempDir;

fn request(directory: &TempDir, id: &str) -> StartCaptureRequest {
    StartCaptureRequest {
        recording_id: RecordingId::new(id),
        device_id: None,
        max_duration_ms: 120_000,
        destination: directory.path().join(format!("{id}.wav")),
    }
}

fn backend_error(description: &str) -> cpal::BuildStreamError {
    cpal::BuildStreamError::BackendSpecific {
        err: cpal::BackendSpecificError {
            description: description.to_string(),
        },
    }
}

#[test]
fn nothing_is_active_before_a_recording_starts() {
    let capture = CpalAudioCapture::new();
    assert!(capture.active().is_none());
}

#[test]
fn finishing_an_unknown_recording_reports_not_found() {
    let capture = CpalAudioCapture::new();
    let error = capture
        .finish(&RecordingId::new("lmr-0123456789abcdef0123456789abcdef"))
        .expect_err("no such recording");
    assert_eq!(error.code(), LocalMediaErrorCode::RecordingNotFound);
}

#[test]
fn cancelling_an_unknown_recording_is_a_no_op() {
    let capture = CpalAudioCapture::new();
    capture.cancel(&RecordingId::new("lmr-0123456789abcdef0123456789abcdef"));
    assert!(capture.active().is_none());
}

#[test]
fn a_start_on_a_machine_with_no_input_device_reports_a_stable_error() {
    let _audio_host = crate::contexts::local_media::infrastructure::audio_host_guard();
    // CI runners generally have no microphone. Either outcome is correct; what must not happen is
    // a panic, a hang, or a slot left occupied so the next attempt reports "already active".
    let directory = TempDir::new().expect("temp dir");
    let capture = CpalAudioCapture::new();
    let recording = RecordingId::new("lmr-0123456789abcdef0123456789abcdef");

    match capture.start(request(&directory, recording.as_str())) {
        Ok(sample_rate) => {
            assert!(sample_rate > 0);
            assert!(capture.active().is_some());
            capture.cancel(&recording);
            assert!(capture.active().is_none(), "cancel must free the slot");
        }
        Err(error) => {
            assert!(
                matches!(
                    error.code(),
                    LocalMediaErrorCode::MicDeviceUnavailable
                        | LocalMediaErrorCode::MicPermissionDenied
                        | LocalMediaErrorCode::AudioCaptureStartFailed
                ),
                "unexpected code {}",
                error.code()
            );
            assert!(
                capture.active().is_none(),
                "a failed start must not occupy the slot"
            );
        }
    }
}

#[test]
fn a_second_start_is_refused_while_one_is_active() {
    let _audio_host = crate::contexts::local_media::infrastructure::audio_host_guard();
    let directory = TempDir::new().expect("temp dir");
    let capture = CpalAudioCapture::new();
    let first = RecordingId::new("lmr-0123456789abcdef0123456789abcdef");
    if capture.start(request(&directory, first.as_str())).is_err() {
        // No capture device here; the singleton rule is also covered by an application-level test
        // that needs no hardware.
        return;
    }
    let error = capture
        .start(request(&directory, "lmr-ffffffffffffffffffffffffffffffff"))
        .expect_err("second start");
    assert_eq!(error.code(), LocalMediaErrorCode::RecordingAlreadyActive);
    capture.cancel(&first);
}

#[test]
fn permission_wording_from_any_backend_maps_to_the_permission_error() {
    // The case users hit most. A generic "capture failed" sends them hunting for a broken device
    // instead of a privacy setting.
    assert!(mentions_permission("Access is denied. (0x80070005)"));
    assert!(mentions_permission("The operation is not authorized"));
    assert!(mentions_permission(
        "Microphone privacy setting blocks capture"
    ));
    assert!(!mentions_permission("No such device"));
    assert!(!mentions_permission("Invalid sample rate"));

    assert_eq!(
        map_build_error(&backend_error("Access is denied")).code(),
        LocalMediaErrorCode::MicPermissionDenied
    );
    assert_eq!(
        map_config_error(&cpal::DefaultStreamConfigError::BackendSpecific {
            err: cpal::BackendSpecificError {
                description: "Permission denied".to_string(),
            },
        })
        .code(),
        LocalMediaErrorCode::MicPermissionDenied
    );
}

#[test]
fn a_missing_device_maps_to_the_device_error_not_a_generic_failure() {
    assert_eq!(
        map_build_error(&cpal::BuildStreamError::DeviceNotAvailable).code(),
        LocalMediaErrorCode::MicDeviceUnavailable
    );
    assert_eq!(
        map_play_error(&cpal::PlayStreamError::DeviceNotAvailable).code(),
        LocalMediaErrorCode::MicDeviceUnavailable
    );
    assert_eq!(
        map_config_error(&cpal::DefaultStreamConfigError::DeviceNotAvailable).code(),
        LocalMediaErrorCode::MicDeviceUnavailable
    );
}

#[test]
fn an_unsupported_stream_configuration_is_a_start_failure() {
    assert_eq!(
        map_config_error(&cpal::DefaultStreamConfigError::StreamTypeNotSupported).code(),
        LocalMediaErrorCode::AudioCaptureStartFailed
    );
    assert_eq!(
        map_build_error(&cpal::BuildStreamError::StreamConfigNotSupported).code(),
        LocalMediaErrorCode::AudioCaptureStartFailed
    );
}

#[test]
fn a_mapped_error_carries_no_backend_message() {
    // cpal descriptions routinely name a device or a driver path; only the stable code survives.
    let error = map_build_error(&backend_error("ALSA device hw:2,0 belonging to someone"));
    assert!(error.details().is_empty());
    assert_eq!(error.to_string(), "AUDIO_CAPTURE_START_FAILED");
}
