//! Native microphone capture.
//!
//! The cpal stream is `!Send` on several platforms, so it is built, played, and dropped entirely on
//! one owned thread. That thread is also where the max-duration stop lives, which keeps the timer
//! off the caller's path: releasing the button and hitting the ceiling take the same route.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, StreamTrait};

use super::devices::resolve_input_device;
use super::pcm::{downmix_to_mono, sample_from_f32, sample_from_u16, CaptureWriter, CommittedWav};
use crate::contexts::local_media::application::ports::{AudioCapturePort, StartCaptureRequest};
use crate::contexts::local_media::domain::{
    CommittedRecording, ComposerScopeId, LocalMediaError, LocalMediaErrorCode, RecordingId,
    RecordingSummary,
};

/// Chunks, not samples. Roughly a quarter second of slack at typical callback sizes: long enough to
/// ride out a scheduler hiccup, short enough that a stalled disk is detected while the user is
/// still speaking rather than after they let go.
const QUEUE_DEPTH: usize = 24;
const STOP_POLL: Duration = Duration::from_millis(10);

struct ActiveCapture {
    recording_id: RecordingId,
    summary: RecordingSummary,
    stop: Arc<AtomicBool>,
    thread: JoinHandle<Result<CommittedWav, LocalMediaError>>,
}

#[derive(Default)]
pub(crate) struct CpalAudioCapture {
    active: Mutex<Option<ActiveCapture>>,
}

impl CpalAudioCapture {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn take_active(&self, recording_id: &RecordingId) -> Option<ActiveCapture> {
        let mut active = self.active.lock().ok()?;
        if active.as_ref().map(|capture| &capture.recording_id) != Some(recording_id) {
            return None;
        }
        active.take()
    }
}

/// Everything the stream thread needs, moved across the boundary in one value.
struct CaptureThreadRequest {
    device_id: Option<String>,
    destination: PathBuf,
    max_duration_ms: u64,
    stop: Arc<AtomicBool>,
    ready: Sender<Result<u32, LocalMediaError>>,
}

impl AudioCapturePort for CpalAudioCapture {
    fn start(&self, request: StartCaptureRequest) -> Result<u32, LocalMediaError> {
        let Ok(mut active) = self.active.lock() else {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::AudioCaptureStartFailed,
            ));
        };
        if active.is_some() {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::RecordingAlreadyActive,
            ));
        }

        let stop = Arc::new(AtomicBool::new(false));
        let (ready_sender, ready_receiver) = channel();
        let thread_request = CaptureThreadRequest {
            device_id: request.device_id.clone(),
            destination: request.destination.clone(),
            max_duration_ms: request.max_duration_ms,
            stop: stop.clone(),
            ready: ready_sender,
        };

        let thread = std::thread::Builder::new()
            .name("local-media-capture".to_string())
            .spawn(move || run_capture(thread_request))
            .map_err(|_| LocalMediaError::new(LocalMediaErrorCode::AudioCaptureStartFailed))?;

        // Block until the stream is actually playing, so a denied microphone surfaces here rather
        // than as an empty WAV discovered on release.
        let sample_rate = match ready_receiver.recv_timeout(Duration::from_secs(10)) {
            Ok(Ok(sample_rate)) => sample_rate,
            Ok(Err(error)) => {
                let _ = thread.join();
                return Err(error);
            }
            Err(_) => {
                stop.store(true, Ordering::SeqCst);
                let _ = thread.join();
                return Err(LocalMediaError::new(
                    LocalMediaErrorCode::AudioCaptureStartFailed,
                ));
            }
        };

        *active = Some(ActiveCapture {
            recording_id: request.recording_id.clone(),
            summary: RecordingSummary {
                recording_id: request.recording_id,
                // Scope ownership is enforced by the application layer; capture only needs a
                // placeholder so the summary shape is complete.
                composer_scope: ComposerScopeId::new("native"),
                started_at_ms: 0,
                max_duration_ms: request.max_duration_ms,
            },
            stop,
            thread,
        });
        Ok(sample_rate)
    }

    fn finish(&self, recording_id: &RecordingId) -> Result<CommittedRecording, LocalMediaError> {
        let Some(capture) = self.take_active(recording_id) else {
            return Err(LocalMediaError::new(LocalMediaErrorCode::RecordingNotFound));
        };
        capture.stop.store(true, Ordering::SeqCst);
        let committed = capture
            .thread
            .join()
            .map_err(|_| LocalMediaError::new(LocalMediaErrorCode::AudioCaptureStartFailed))??;

        if committed.overrun {
            return Err(
                LocalMediaError::new(LocalMediaErrorCode::AudioCaptureOverrun)
                    .with_number("sampleCount", committed.sample_count as i64),
            );
        }
        Ok(CommittedRecording {
            recording_id: recording_id.clone(),
            duration_ms: committed.duration_ms,
            sample_rate: committed.sample_rate,
            sample_count: committed.sample_count,
            // One poll interval of tolerance: the stop check cannot fire exactly on the ceiling, so
            // a recording that ran to the limit lands a few milliseconds short of it.
            limit_reached: committed.duration_ms
                >= capture
                    .summary
                    .max_duration_ms
                    .saturating_sub(STOP_POLL.as_millis() as u64),
        })
    }

    fn cancel(&self, recording_id: &RecordingId) {
        let Some(capture) = self.take_active(recording_id) else {
            return;
        };
        capture.stop.store(true, Ordering::SeqCst);
        // The partial WAV is finalized and then deleted by the caller's cleanup. Joining rather
        // than detaching means the file handle is closed before that delete runs, which matters on
        // Windows where an open handle makes the removal fail.
        let _ = capture.thread.join();
    }

    fn active(&self) -> Option<RecordingSummary> {
        let active = self.active.lock().ok()?;
        active.as_ref().map(|capture| capture.summary.clone())
    }
}

fn run_capture(request: CaptureThreadRequest) -> Result<CommittedWav, LocalMediaError> {
    let Some(device) = resolve_input_device(request.device_id.as_deref()) else {
        let unavailable = LocalMediaError::new(LocalMediaErrorCode::MicDeviceUnavailable);
        let _ = request.ready.send(Err(unavailable.clone()));
        return Err(unavailable);
    };
    let supported = match device.default_input_config() {
        Ok(config) => config,
        Err(error) => {
            let mapped = map_config_error(&error);
            let _ = request.ready.send(Err(mapped.clone()));
            return Err(mapped);
        }
    };

    let sample_rate = supported.sample_rate();
    let channels = supported.channels();
    let writer = match CaptureWriter::start(&request.destination, sample_rate, QUEUE_DEPTH) {
        Ok(writer) => Arc::new(writer),
        Err(error) => {
            let _ = request.ready.send(Err(error.clone()));
            return Err(error);
        }
    };

    let overrun = writer.overrun_flag();
    let stream_config: cpal::StreamConfig = supported.config();
    let stream = match build_stream(
        &device,
        &stream_config,
        supported.sample_format(),
        channels,
        writer.clone(),
    ) {
        Ok(stream) => stream,
        Err(error) => {
            let _ = request.ready.send(Err(error.clone()));
            return Err(error);
        }
    };

    if let Err(error) = stream.play() {
        let mapped = map_play_error(&error);
        let _ = request.ready.send(Err(mapped.clone()));
        return Err(mapped);
    }
    let _ = request.ready.send(Ok(sample_rate));

    let started = Instant::now();
    let ceiling = Duration::from_millis(request.max_duration_ms);
    while !request.stop.load(Ordering::SeqCst) {
        if overrun.load(Ordering::SeqCst) {
            break;
        }
        if started.elapsed() >= ceiling {
            // Auto-stop keeps what was captured. The utterance is complete up to the ceiling and
            // the caller reports the limit as a warning on a successful transcription.
            break;
        }
        std::thread::sleep(STOP_POLL);
    }

    drop(stream);
    let Ok(writer) = Arc::try_unwrap(writer) else {
        return Err(LocalMediaError::new(
            LocalMediaErrorCode::AudioCaptureStartFailed,
        ));
    };
    writer.finish()
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    format: cpal::SampleFormat,
    channels: u16,
    writer: Arc<CaptureWriter>,
) -> Result<cpal::Stream, LocalMediaError> {
    // The callback is real-time: it converts, downmixes into reused buffers, and hands the chunk to
    // a bounded queue. No file I/O, no logging, and no allocation beyond the two `Vec` growths that
    // settle after the first few callbacks.
    macro_rules! stream_for {
        ($sample:ty, $convert:expr) => {{
            let mut scratch: Vec<i16> = Vec::new();
            let mut mono: Vec<i16> = Vec::new();
            device.build_input_stream(
                config,
                move |data: &[$sample], _: &cpal::InputCallbackInfo| {
                    scratch.clear();
                    scratch.extend(data.iter().copied().map($convert));
                    mono.clear();
                    downmix_to_mono(&scratch, channels, &mut mono);
                    if !mono.is_empty() {
                        let _ = writer.submit(mono.clone());
                    }
                },
                |_| {},
                None,
            )
        }};
    }

    let built = match format {
        cpal::SampleFormat::F32 => stream_for!(f32, sample_from_f32),
        cpal::SampleFormat::I16 => stream_for!(i16, |value| value),
        cpal::SampleFormat::U16 => stream_for!(u16, sample_from_u16),
        _ => {
            return Err(
                LocalMediaError::new(LocalMediaErrorCode::AudioCaptureStartFailed)
                    .with_text("field", "sampleFormat"),
            )
        }
    };
    built.map_err(|error| map_build_error(&error))
}

/// Map cpal's device errors onto the stable vocabulary.
///
/// cpal 0.17 has no typed permission variant -- WASAPI and CoreAudio both surface a denial as a
/// backend string -- so the permission case is recognized by wording. The match is deliberately
/// broad: over-reporting "check your microphone permission" is a better failure than telling a user
/// their device is missing when the OS simply refused. cpal 0.18 fixes this with
/// `ErrorKind::PermissionDenied`; the workspace manifest records why this crate is not on it.
fn map_build_error(error: &cpal::BuildStreamError) -> LocalMediaError {
    match error {
        cpal::BuildStreamError::DeviceNotAvailable => {
            LocalMediaError::new(LocalMediaErrorCode::MicDeviceUnavailable)
        }
        cpal::BuildStreamError::BackendSpecific { err }
            if mentions_permission(&err.description) =>
        {
            LocalMediaError::new(LocalMediaErrorCode::MicPermissionDenied)
        }
        _ => LocalMediaError::new(LocalMediaErrorCode::AudioCaptureStartFailed),
    }
}

fn map_play_error(error: &cpal::PlayStreamError) -> LocalMediaError {
    match error {
        cpal::PlayStreamError::DeviceNotAvailable => {
            LocalMediaError::new(LocalMediaErrorCode::MicDeviceUnavailable)
        }
        _ => LocalMediaError::new(LocalMediaErrorCode::AudioCaptureStartFailed),
    }
}

fn map_config_error(error: &cpal::DefaultStreamConfigError) -> LocalMediaError {
    match error {
        cpal::DefaultStreamConfigError::DeviceNotAvailable => {
            LocalMediaError::new(LocalMediaErrorCode::MicDeviceUnavailable)
        }
        cpal::DefaultStreamConfigError::BackendSpecific { err }
            if mentions_permission(&err.description) =>
        {
            LocalMediaError::new(LocalMediaErrorCode::MicPermissionDenied)
        }
        _ => LocalMediaError::new(LocalMediaErrorCode::AudioCaptureStartFailed),
    }
}

fn mentions_permission(description: &str) -> bool {
    let lowered = description.to_lowercase();
    [
        "access",
        "denied",
        "permission",
        "not authorized",
        "unauthorized",
        "privacy",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

#[cfg(test)]
#[path = "cpal_capture_tests.rs"]
mod tests;
