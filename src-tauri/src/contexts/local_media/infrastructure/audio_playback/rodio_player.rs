//! Local playback of one generated utterance at a time.
//!
//! rodio's output stream owns a cpal stream and is not `Send`, so the sink lives entirely on one
//! owned thread. `play_blocking` waits on that thread, which is what lets the caller keep the
//! operation in `playing` until the audio actually finishes rather than until it was queued.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::contexts::local_media::application::ports::AudioPlaybackPort;
use crate::contexts::local_media::domain::{LocalMediaError, LocalMediaErrorCode, PlaybackId};

const POLL: Duration = Duration::from_millis(20);
/// A generated preview is seconds long. The ceiling exists so a decoder that never reports empty
/// cannot hold the operation open indefinitely.
const MAX_PLAYBACK: Duration = Duration::from_secs(600);

struct ActivePlayback {
    playback_id: PlaybackId,
    stop: Arc<AtomicBool>,
}

#[derive(Default)]
pub(crate) struct RodioPlayback {
    active: Mutex<Option<ActivePlayback>>,
}

impl RodioPlayback {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn claim(&self, playback_id: &PlaybackId, stop: Arc<AtomicBool>) {
        let Ok(mut active) = self.active.lock() else {
            return;
        };
        // Starting a new utterance ends the previous one. Two synthesized voices overlapping is
        // never what the user asked for.
        if let Some(previous) = active.take() {
            previous.stop.store(true, Ordering::SeqCst);
        }
        *active = Some(ActivePlayback {
            playback_id: playback_id.clone(),
            stop,
        });
    }

    /// The playback currently holding the slot. No production caller needs this -- an operation
    /// already knows its own id -- but the tests do, to assert the slot is released on every path.
    #[cfg(test)]
    pub(super) fn active(&self) -> Option<PlaybackId> {
        let active = self.active.lock().ok()?;
        active.as_ref().map(|entry| entry.playback_id.clone())
    }

    fn release(&self, playback_id: &PlaybackId) {
        let Ok(mut active) = self.active.lock() else {
            return;
        };
        if active.as_ref().map(|entry| &entry.playback_id) == Some(playback_id) {
            *active = None;
        }
    }
}

impl AudioPlaybackPort for RodioPlayback {
    fn play_blocking(
        &self,
        playback_id: &PlaybackId,
        path: &Path,
        device_id: Option<&str>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<u64, LocalMediaError> {
        if cancelled.load(Ordering::SeqCst) {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::OperationCancelled,
            ));
        }

        let stop = Arc::new(AtomicBool::new(false));
        self.claim(playback_id, stop.clone());

        let owned_path: PathBuf = path.to_path_buf();
        let owned_device = device_id.map(str::to_string);
        let thread_stop = stop.clone();
        let thread_cancelled = cancelled.clone();

        let handle = std::thread::Builder::new()
            .name("local-media-playback".to_string())
            .spawn(move || run_playback(&owned_path, owned_device, thread_stop, thread_cancelled))
            .map_err(|_| LocalMediaError::new(LocalMediaErrorCode::PlaybackDeviceUnavailable));

        let handle = match handle {
            Ok(handle) => handle,
            Err(error) => {
                self.release(playback_id);
                return Err(error);
            }
        };

        let outcome = handle.join().unwrap_or_else(|_| {
            Err(LocalMediaError::new(
                LocalMediaErrorCode::PlaybackDeviceUnavailable,
            ))
        });
        self.release(playback_id);
        outcome
    }

    fn stop(&self, playback_id: Option<&PlaybackId>) {
        let Ok(mut active) = self.active.lock() else {
            return;
        };
        let matches = match (playback_id, active.as_ref()) {
            (None, Some(_)) => true,
            (Some(requested), Some(entry)) => &entry.playback_id == requested,
            _ => false,
        };
        if !matches {
            return;
        }
        if let Some(entry) = active.take() {
            entry.stop.store(true, Ordering::SeqCst);
        }
    }
}

fn run_playback(
    path: &Path,
    device_id: Option<String>,
    stop: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
) -> Result<u64, LocalMediaError> {
    let unavailable = || LocalMediaError::new(LocalMediaErrorCode::PlaybackDeviceUnavailable);

    let file = std::fs::File::open(path)
        .map_err(|_| LocalMediaError::new(LocalMediaErrorCode::TempStorageFailed))?;
    // `new_wav` rather than `new`: the only file that ever reaches here is one this application
    // just generated, so format sniffing would add a guess where the answer is already known.
    let source = rodio::Decoder::new_wav(std::io::BufReader::new(file))
        .map_err(|_| LocalMediaError::new(LocalMediaErrorCode::PlaybackDeviceUnavailable))?;

    let Some(device) = super::super::audio_capture::resolve_output_device(device_id.as_deref())
    else {
        return Err(unavailable());
    };
    let device_sink = rodio::DeviceSinkBuilder::from_device(device)
        .and_then(|builder| builder.open_stream())
        .map_err(|_| unavailable())?;
    let player = rodio::Player::connect_new(device_sink.mixer());
    player.append(source);

    let started = Instant::now();
    loop {
        if stop.load(Ordering::SeqCst) || cancelled.load(Ordering::SeqCst) {
            // `stop` drops the queued source immediately; the sink is torn down when this scope
            // ends, so the device is released before the caller deletes the file.
            player.stop();
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::OperationCancelled,
            ));
        }
        if player.empty() {
            break;
        }
        if started.elapsed() >= MAX_PLAYBACK {
            player.stop();
            return Err(unavailable());
        }
        std::thread::sleep(POLL);
    }
    Ok(started.elapsed().as_millis() as u64)
}

#[cfg(test)]
#[path = "rodio_player_tests.rs"]
mod tests;
