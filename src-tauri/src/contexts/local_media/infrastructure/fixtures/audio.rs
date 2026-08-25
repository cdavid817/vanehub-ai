use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use super::scenario::FixtureScenario;
use crate::contexts::local_media::application::ports::{
    AudioCapturePort, AudioDeviceCatalogPort, AudioPlaybackPort, StartCaptureRequest,
};
use crate::contexts::local_media::domain::{
    AudioDevice, AudioDeviceCatalog, CommittedRecording, ComposerScopeId, LocalMediaError,
    LocalMediaErrorCode, PlaybackId, RecordingId, RecordingSummary,
};

const SAMPLE_RATE: u32 = 16_000;

struct ActiveCapture {
    summary: RecordingSummary,
    destination: PathBuf,
    frames: u64,
    limit_reached: bool,
}

/// Writes a real 16-bit mono PCM WAV into the operation's own directory.
///
/// Returning an in-memory transcript instead would skip everything that makes the STT path worth
/// testing: the file has to be written where the host authorised, survive the temp store's
/// ownership rules, and be read back by the real worker over the real protocol.
pub(crate) struct FixtureAudioCapture {
    scenario: Option<PathBuf>,
    active: Mutex<Option<ActiveCapture>>,
}

impl FixtureAudioCapture {
    pub(crate) fn new(scenario: Option<PathBuf>) -> Self {
        Self {
            scenario,
            active: Mutex::new(None),
        }
    }

    fn behaviour(&self) -> String {
        FixtureScenario::load(self.scenario.as_deref()).capture
    }
}

/// A one-second 440 Hz tone. Not speech: the transcript comes from the scripted Python fixture, and
/// shipping a recording of a real voice would put someone's speech in the repository for no signal.
fn tone(frames: u64) -> Vec<u8> {
    let mut samples = Vec::with_capacity(frames as usize * 2);
    for index in 0..frames {
        let phase = 2.0 * std::f64::consts::PI * 440.0 * (index as f64) / f64::from(SAMPLE_RATE);
        let value = (12_000.0 * phase.sin()) as i16;
        samples.extend_from_slice(&value.to_le_bytes());
    }
    samples
}

fn write_wav(path: &Path, pcm: &[u8]) -> Result<(), LocalMediaError> {
    let storage_failed = || LocalMediaError::new(LocalMediaErrorCode::TempStorageFailed);
    let mut bytes = Vec::with_capacity(pcm.len() + 44);
    let data_len = pcm.len() as u32;
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    bytes.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    bytes.extend_from_slice(pcm);
    std::fs::write(path, bytes).map_err(|_| storage_failed())
}

impl AudioCapturePort for FixtureAudioCapture {
    fn start(&self, request: StartCaptureRequest) -> Result<u32, LocalMediaError> {
        let behaviour = self.behaviour();
        let failure = match behaviour.as_str() {
            "permission_denied" => Some(LocalMediaErrorCode::MicPermissionDenied),
            "device_unavailable" => Some(LocalMediaErrorCode::MicDeviceUnavailable),
            "start_failure" => Some(LocalMediaErrorCode::AudioCaptureStartFailed),
            _ => None,
        };
        if let Some(code) = failure {
            return Err(LocalMediaError::new(code));
        }

        let mut active = self
            .active
            .lock()
            .map_err(|_| LocalMediaError::new(LocalMediaErrorCode::AudioCaptureStartFailed))?;
        if active.is_some() {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::RecordingAlreadyActive,
            ));
        }
        // "too_short" produces a real under-length file so the application layer's minimum, not the
        // fixture, decides the outcome.
        let frames = match behaviour.as_str() {
            "too_short" => u64::from(SAMPLE_RATE) / 100,
            _ => u64::from(SAMPLE_RATE),
        };
        *active = Some(ActiveCapture {
            summary: RecordingSummary {
                recording_id: request.recording_id,
                composer_scope: ComposerScopeId::new("fixture"),
                started_at_ms: 0,
                max_duration_ms: request.max_duration_ms,
            },
            destination: request.destination,
            frames,
            limit_reached: behaviour == "limit_reached",
        });
        Ok(SAMPLE_RATE)
    }

    fn finish(&self, recording_id: &RecordingId) -> Result<CommittedRecording, LocalMediaError> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| LocalMediaError::new(LocalMediaErrorCode::RecordingNotFound))?;
        let Some(capture) = active
            .take()
            .filter(|c| &c.summary.recording_id == recording_id)
        else {
            return Err(LocalMediaError::new(LocalMediaErrorCode::RecordingNotFound));
        };

        match self.behaviour().as_str() {
            "device_disconnected" => {
                return Err(LocalMediaError::new(
                    LocalMediaErrorCode::MicDeviceUnavailable,
                ))
            }
            "overrun" => {
                return Err(LocalMediaError::new(
                    LocalMediaErrorCode::AudioCaptureOverrun,
                ))
            }
            _ => {}
        }

        write_wav(&capture.destination, &tone(capture.frames))?;
        Ok(CommittedRecording {
            recording_id: capture.summary.recording_id,
            duration_ms: capture.frames * 1000 / u64::from(SAMPLE_RATE),
            sample_rate: SAMPLE_RATE,
            sample_count: capture.frames,
            limit_reached: capture.limit_reached,
        })
    }

    fn cancel(&self, recording_id: &RecordingId) {
        if let Ok(mut active) = self.active.lock() {
            if active
                .as_ref()
                .is_some_and(|c| &c.summary.recording_id == recording_id)
            {
                *active = None;
            }
        }
    }

    fn active(&self) -> Option<RecordingSummary> {
        self.active.lock().ok()?.as_ref().map(|c| c.summary.clone())
    }
}

/// Playback without a sound card, but with the real state machine.
pub(crate) struct FixtureAudioPlayback {
    scenario: Option<PathBuf>,
    active: Mutex<Option<(PlaybackId, Arc<AtomicBool>)>>,
}

impl FixtureAudioPlayback {
    pub(crate) fn new(scenario: Option<PathBuf>) -> Self {
        Self {
            scenario,
            active: Mutex::new(None),
        }
    }
}

impl AudioPlaybackPort for FixtureAudioPlayback {
    fn play_blocking(
        &self,
        playback_id: &PlaybackId,
        path: &Path,
        _device_id: Option<&str>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<u64, LocalMediaError> {
        if FixtureScenario::load(self.scenario.as_deref()).playback == "failure" {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::PlaybackDeviceUnavailable,
            ));
        }
        // The file the real worker wrote must exist and be readable; a fixture that skipped this
        // would pass even if synthesis never produced anything.
        let bytes = std::fs::metadata(path)
            .map(|meta| meta.len())
            .map_err(|_| LocalMediaError::new(LocalMediaErrorCode::PlaybackDeviceUnavailable))?;
        if bytes <= 44 {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::PlaybackDeviceUnavailable,
            ));
        }

        if let Ok(mut active) = self.active.lock() {
            // Starting a new utterance ends the previous one, exactly as the real sink does.
            if let Some((_, previous)) = active.take() {
                previous.store(true, Ordering::SeqCst);
            }
            *active = Some((playback_id.clone(), cancelled.clone()));
        }

        let frames = (bytes - 44) / 2;
        let duration_ms = frames * 1000 / u64::from(SAMPLE_RATE);
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(duration_ms);
        while std::time::Instant::now() < deadline {
            if cancelled.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        if let Ok(mut active) = self.active.lock() {
            if active.as_ref().is_some_and(|(id, _)| id == playback_id) {
                *active = None;
            }
        }
        Ok(duration_ms)
    }

    fn stop(&self, playback_id: Option<&PlaybackId>) {
        if let Ok(active) = self.active.lock() {
            if let Some((id, flag)) = active.as_ref() {
                if playback_id.is_none_or(|requested| requested == id) {
                    flag.store(true, Ordering::SeqCst);
                }
            }
        }
    }
}

/// Deterministic devices, plus the two states a headless runner otherwise never reaches.
pub(crate) struct FixtureAudioDeviceCatalog {
    scenario: Option<PathBuf>,
}

impl FixtureAudioDeviceCatalog {
    pub(crate) fn new(scenario: Option<PathBuf>) -> Self {
        Self { scenario }
    }
}

impl AudioDeviceCatalogPort for FixtureAudioDeviceCatalog {
    fn catalog(&self) -> Result<AudioDeviceCatalog, LocalMediaError> {
        match FixtureScenario::load(self.scenario.as_deref())
            .devices
            .as_str()
        {
            "enumeration_failure" => Err(LocalMediaError::new(
                LocalMediaErrorCode::MicDeviceUnavailable,
            )),
            "no_devices" => Ok(AudioDeviceCatalog {
                inputs: Vec::new(),
                outputs: Vec::new(),
            }),
            _ => Ok(AudioDeviceCatalog {
                inputs: vec![AudioDevice {
                    device_id: "fixture-input".to_string(),
                    label: "Fixture Microphone".to_string(),
                    is_default: true,
                }],
                outputs: vec![AudioDevice {
                    device_id: "fixture-output".to_string(),
                    label: "Fixture Speaker".to_string(),
                    is_default: true,
                }],
            }),
        }
    }
}
