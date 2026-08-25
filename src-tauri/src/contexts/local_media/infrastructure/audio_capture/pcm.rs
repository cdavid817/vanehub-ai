//! Sample conversion, downmixing, and the non-real-time WAV writer.
//!
//! Everything the audio callback is allowed to do lives here, and everything it is *not* allowed to
//! do -- open a file, allocate without a bound, block -- lives behind the channel. The split is the
//! whole reason a slow disk shows up as `AUDIO_CAPTURE_OVERRUN` instead of as a stuttering device.

use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender, TrySendError};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::contexts::local_media::domain::{duration_ms_for, LocalMediaError, LocalMediaErrorCode};

/// Scale a normalized float sample. `i16::MAX` on both ends keeps the range symmetric; using
/// `i16::MIN` for -1.0 would make a full-scale negative peak one unit louder than its positive
/// twin, which is audible as asymmetric clipping.
pub(super) fn sample_from_f32(value: f32) -> i16 {
    if value.is_nan() {
        return 0;
    }
    let clamped = value.clamp(-1.0, 1.0);
    (clamped * f32::from(i16::MAX)) as i16
}

/// Unsigned PCM is centred on 32,768. Subtracting in `i32` avoids the wrap that `value - 32_768`
/// would produce in `u16`.
pub(super) fn sample_from_u16(value: u16) -> i16 {
    let centred = i32::from(value) - 32_768;
    centred.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16
}

/// Average interleaved channels into mono, appending to `output`.
///
/// The accumulator is `i32`: two full-scale `i16` channels sum to 65,534, which wraps to -2 in
/// `i16`. A trailing partial frame is dropped rather than padded, because padding puts a click at
/// the end of every recording whose buffer did not divide evenly.
pub(super) fn downmix_to_mono(interleaved: &[i16], channels: u16, output: &mut Vec<i16>) {
    if channels == 0 {
        return;
    }
    let channels = usize::from(channels);
    if channels == 1 {
        output.extend_from_slice(interleaved);
        return;
    }
    for frame in interleaved.chunks_exact(channels) {
        let sum: i32 = frame.iter().map(|sample| i32::from(*sample)).sum();
        let averaged = sum / channels as i32;
        output.push(averaged.clamp(i32::from(i16::MIN), i32::from(i16::MAX)) as i16);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CommittedWav {
    pub(super) sample_count: u64,
    pub(super) sample_rate: u32,
    pub(super) duration_ms: u64,
    pub(super) overrun: bool,
}

fn capture_error(code: LocalMediaErrorCode) -> LocalMediaError {
    LocalMediaError::new(code)
}

pub(super) struct PcmWavWriter {
    writer: hound::WavWriter<std::io::BufWriter<std::fs::File>>,
    sample_rate: u32,
    sample_count: u64,
}

impl PcmWavWriter {
    pub(super) fn create(path: &Path, sample_rate: u32) -> Result<Self, LocalMediaError> {
        if sample_rate == 0 {
            return Err(capture_error(LocalMediaErrorCode::AudioCaptureStartFailed));
        }
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let writer = hound::WavWriter::create(path, spec)
            .map_err(|_| capture_error(LocalMediaErrorCode::TempStorageFailed))?;
        Ok(Self {
            writer,
            sample_rate,
            sample_count: 0,
        })
    }

    pub(super) fn write(&mut self, samples: &[i16]) -> Result<(), LocalMediaError> {
        for sample in samples {
            self.writer
                .write_sample(*sample)
                .map_err(|_| capture_error(LocalMediaErrorCode::TempStorageFailed))?;
        }
        self.sample_count += samples.len() as u64;
        Ok(())
    }

    /// Flush and write the final RIFF header. Until this runs the file's declared length is wrong,
    /// which is why nothing reads the WAV before capture reports it committed.
    pub(super) fn finalize(self) -> Result<CommittedWav, LocalMediaError> {
        let sample_count = self.sample_count;
        let sample_rate = self.sample_rate;
        self.writer
            .finalize()
            .map_err(|_| capture_error(LocalMediaErrorCode::TempStorageFailed))?;
        Ok(CommittedWav {
            sample_count,
            sample_rate,
            duration_ms: duration_ms_for(sample_count, sample_rate),
            overrun: false,
        })
    }
}

/// The writer half of the capture pipeline: a bounded queue and a thread that drains it.
pub(super) struct CaptureWriter {
    sender: Option<SyncSender<Vec<i16>>>,
    handle: Option<JoinHandle<Result<CommittedWav, LocalMediaError>>>,
    overrun: Arc<AtomicBool>,
    submitted: Arc<AtomicU64>,
    #[cfg(test)]
    paused: Arc<AtomicBool>,
}

impl CaptureWriter {
    /// `queue_depth` chunks, not samples. Sized for a short window: long enough to absorb a
    /// scheduling hiccup, short enough that a genuinely stalled writer is detected in well under a
    /// second instead of after the utterance is over.
    pub(super) fn start(
        path: &Path,
        sample_rate: u32,
        queue_depth: usize,
    ) -> Result<Self, LocalMediaError> {
        let mut writer = PcmWavWriter::create(path, sample_rate)?;
        let (sender, receiver): (SyncSender<Vec<i16>>, Receiver<Vec<i16>>) =
            sync_channel(queue_depth.max(1));
        let overrun = Arc::new(AtomicBool::new(false));
        #[cfg(test)]
        let paused = Arc::new(AtomicBool::new(false));
        #[cfg(test)]
        let writer_paused = paused.clone();

        let handle = std::thread::Builder::new()
            .name("local-media-wav-writer".to_string())
            .spawn(move || {
                while let Ok(chunk) = receiver.recv() {
                    #[cfg(test)]
                    while writer_paused.load(Ordering::SeqCst) {
                        std::thread::sleep(std::time::Duration::from_millis(1));
                    }
                    writer.write(&chunk)?;
                }
                writer.finalize()
            })
            .map_err(|_| capture_error(LocalMediaErrorCode::AudioCaptureStartFailed))?;

        Ok(Self {
            sender: Some(sender),
            handle: Some(handle),
            overrun,
            submitted: Arc::new(AtomicU64::new(0)),
            #[cfg(test)]
            paused,
        })
    }

    /// Hand a chunk to the writer without blocking.
    ///
    /// Called from the real-time audio callback, so it must never wait. A full queue is an explicit
    /// failure rather than a dropped chunk: dropping would produce a transcript of an utterance the
    /// user did not speak, which is worse than reporting that capture could not keep up.
    pub(super) fn submit(&self, chunk: Vec<i16>) -> Result<(), ()> {
        let Some(sender) = self.sender.as_ref() else {
            return Err(());
        };
        let length = chunk.len() as u64;
        match sender.try_send(chunk) {
            Ok(()) => {
                self.submitted.fetch_add(length, Ordering::Relaxed);
                Ok(())
            }
            Err(TrySendError::Full(_)) => {
                self.overrun.store(true, Ordering::SeqCst);
                Err(())
            }
            Err(TrySendError::Disconnected(_)) => Err(()),
        }
    }

    pub(super) fn overrun_flag(&self) -> Arc<AtomicBool> {
        self.overrun.clone()
    }

    pub(super) fn finish(mut self) -> Result<CommittedWav, LocalMediaError> {
        // Dropping the sender is what ends the writer's `recv` loop; joining then guarantees the
        // header is written before anyone reads the file.
        self.sender = None;
        let Some(handle) = self.handle.take() else {
            return Err(capture_error(LocalMediaErrorCode::TempStorageFailed));
        };
        let mut committed = handle
            .join()
            .map_err(|_| capture_error(LocalMediaErrorCode::TempStorageFailed))??;
        committed.overrun = self.overrun.load(Ordering::SeqCst);
        Ok(committed)
    }

    #[cfg(test)]
    pub(super) fn pause_writer_for_test(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    #[cfg(test)]
    pub(super) fn resume_writer_for_test(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }
}

#[cfg(test)]
#[path = "pcm_tests.rs"]
mod tests;
