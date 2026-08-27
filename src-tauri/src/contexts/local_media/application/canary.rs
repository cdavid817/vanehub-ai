//! Readiness canaries: the minimal real inference that turns "it loaded" into "it works".
//!
//! A probe that stops at construction reports `Ready` for the one failure this project has actually
//! measured -- a model paddlepaddle's accelerated executor accepts on load and cannot run. The user
//! then meets it on their first real operation, with an operation-shaped error instead of a
//! settings-shaped one.
//!
//! There is no second inference implementation here. Each canary builds an input in the probe
//! operation's own directory and issues the same `WorkerCall` the composer issues, so what is
//! proven is the production path and not a parallel one. Nothing the canary produces is kept:
//! recognized text, transcripts and synthesized audio are discarded before this returns, and the
//! operation guard deletes the directory on every exit.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use super::service::LocalMediaApplicationService;
use super::worker_contract::{
    OcrWorkerRequest, SttWorkerRequest, TtsWorkerRequest, WorkerCall, WorkerReply,
};
use crate::contexts::local_media::domain::{
    LocalMediaEngine, LocalMediaError, LocalMediaErrorCode, LocalMediaProfileSnapshot, OcrMediaType,
};

/// A 156-byte greyscale PNG of five blocky glyph shapes on a baseline.
///
/// Not a blank image, deliberately: a detector finds nothing in blankness, recognition never runs,
/// and the stage that fails on an incompatible graph is exactly the one skipped. Not letters of any
/// alphabet either -- the canary has to be recognized *at*, not recognized *as* something, and a
/// real word would invite an assertion about the text that came back.
const CANARY_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0xa0, 0x00, 0x00, 0x00, 0x30, 0x08, 0x00, 0x00, 0x00, 0x00, 0xb3, 0xc2, 0x2e,
    0xb2, 0x00, 0x00, 0x00, 0x63, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0xed, 0xd8, 0x31, 0x0e, 0xc0,
    0x20, 0x08, 0x05, 0x50, 0xee, 0x7f, 0xe9, 0xea, 0xdc, 0x04, 0xc5, 0xa5, 0x36, 0xfa, 0xfe, 0x88,
    0x4a, 0xde, 0x22, 0x31, 0xc6, 0xf3, 0xf3, 0x04, 0x20, 0x20, 0x20, 0x20, 0x20, 0xe0, 0x25, 0xc0,
    0xe8, 0x49, 0x37, 0xbd, 0xb2, 0xb2, 0x96, 0xf5, 0x1f, 0x9d, 0x03, 0x04, 0xcc, 0x99, 0xd5, 0xea,
    0x1c, 0x58, 0xee, 0x01, 0x08, 0x08, 0xb8, 0x19, 0x58, 0x1a, 0x25, 0x80, 0x80, 0x6e, 0x31, 0x20,
    0xe0, 0x21, 0xc0, 0xf1, 0x48, 0xc8, 0xab, 0x9f, 0x3d, 0x58, 0x01, 0x8f, 0x06, 0xfa, 0x59, 0x00,
    0x04, 0x04, 0x04, 0x04, 0x04, 0xdc, 0x92, 0x06, 0xe5, 0xd8, 0x67, 0x07, 0xb3, 0xa9, 0x9b, 0xdf,
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
];

const CANARY_SAMPLE_RATE: u32 = 16_000;
/// A tenth of a second. faster-whisper pads its input internally, so the cost of the canary is the
/// decoder pass rather than the audio length.
const CANARY_FRAMES: u32 = CANARY_SAMPLE_RATE / 10;

/// One token, because a canary that says a sentence is a canary that assumes a language.
///
/// Recorded limitation: this is still Latin text, and a voice whose lexicon has no entry for it may
/// fail a canary it would pass with model-appropriate input. See design.md section 35.
const CANARY_TTS_TEXT: &str = "a";

const CANARY_IMAGE_FILE: &str = "canary.png";
const CANARY_AUDIO_FILE: &str = "canary.wav";

/// 16-bit mono PCM silence with a RIFF header.
///
/// Silence is a legitimate canary: with the VAD shortcut disabled the decoder runs over it and an
/// empty transcript is a real result. Built here rather than embedded because a header plus zeroes
/// is smaller as code than as a byte array.
fn canary_wav() -> Vec<u8> {
    let data_bytes = CANARY_FRAMES * 2;
    let mut wav = Vec::with_capacity(44 + data_bytes as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&CANARY_SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(CANARY_SAMPLE_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    wav.resize(44 + data_bytes as usize, 0);
    wav
}

impl LocalMediaApplicationService {
    /// Run the engine's own inference once over host-authored input.
    ///
    /// Returns `Ok(())` only when the engine produced a well-formed result. Everything it produced
    /// is dropped here; the caller records readiness and nothing else.
    pub(super) fn run_readiness_canary(
        &self,
        operation_id: &str,
        snapshot: &LocalMediaProfileSnapshot,
        flag: Arc<AtomicBool>,
    ) -> Result<(), LocalMediaError> {
        let engine = snapshot.engine();
        let call = self.canary_call(operation_id, engine)?;
        let reply = self.inner.workers.call(snapshot, call, flag)?;

        match (engine, reply) {
            (LocalMediaEngine::Ocr, WorkerReply::Ocr(_)) => Ok(()),
            // An empty transcript is a pass. The canary is silence, and what it proves is that the
            // decoder ran -- asserting on words would make the canary depend on the model's output.
            (LocalMediaEngine::Stt, WorkerReply::Transcribe(_)) => Ok(()),
            (LocalMediaEngine::Tts, WorkerReply::Synthesize(reply)) => {
                // The one canary whose output has to be checked rather than discarded unread: a
                // synthesis that reports success and wrote nothing playable is not a working engine.
                let bytes = self
                    .inner
                    .temp
                    .verify_output_wav(operation_id, &reply.audio_path)?;
                if bytes == 0 || reply.sample_rate == 0 || reply.duration_ms == 0 {
                    return Err(LocalMediaError::new(LocalMediaErrorCode::EngineUnavailable)
                        .with_text("engine", engine.as_str()));
                }
                Ok(())
            }
            // A reply of the wrong shape is a protocol failure, not a partial success.
            _ => Err(LocalMediaError::new(
                LocalMediaErrorCode::WorkerProtocolError,
            )),
        }
    }

    fn canary_call(
        &self,
        operation_id: &str,
        engine: LocalMediaEngine,
    ) -> Result<WorkerCall, LocalMediaError> {
        Ok(match engine {
            LocalMediaEngine::Ocr => WorkerCall::Ocr(OcrWorkerRequest {
                source_path: self.canary_input(operation_id, CANARY_IMAGE_FILE, CANARY_PNG)?,
                media_type: OcrMediaType::Image,
                max_pdf_pages: 1,
                max_output_characters: 4_096,
            }),
            LocalMediaEngine::Stt => WorkerCall::Transcribe(SttWorkerRequest {
                audio_path: self.canary_input(operation_id, CANARY_AUDIO_FILE, &canary_wav())?,
                // The profile's VAD setting is overridden for this one call. Left on, the filter
                // finds no speech in silence and returns without ever reaching the decoder -- the
                // canary would pass on a model that cannot decode at all.
                bypass_voice_activity_filter: true,
            }),
            LocalMediaEngine::Tts => WorkerCall::Synthesize(TtsWorkerRequest {
                text: CANARY_TTS_TEXT.to_string(),
                output_path: self.inner.temp.authorize_output_wav(operation_id)?,
            }),
        })
    }

    fn canary_input(
        &self,
        operation_id: &str,
        file_name: &str,
        bytes: &[u8],
    ) -> Result<PathBuf, LocalMediaError> {
        self.inner
            .temp
            .authorize_canary_input(operation_id, file_name, bytes)
    }
}
