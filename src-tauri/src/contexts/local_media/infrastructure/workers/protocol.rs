//! Wire encoding for the JSON Lines worker protocol.
//!
//! Nothing here talks to a process. Keeping the encoding pure means the framing rules -- version,
//! id correlation, size bounds, reply shape -- are testable without spawning Python, and the
//! process module below has only lifecycle left to get wrong.

use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::contexts::local_media::application::worker_contract::{
    OcrReply, ProbeReply, SynthesizeReply, TranscribeReply, WorkerCall, WorkerLine, WorkerPage,
    WorkerReply,
};
use crate::contexts::local_media::domain::{
    LocalMediaEngine, LocalMediaError, LocalMediaErrorCode, LocalMediaProfileSnapshot,
};
use std::path::PathBuf;

pub(super) const PROTOCOL_VERSION: u8 = 1;
/// The identifier the compatibility fixture and the extension catalog both pin.
///
/// The wire field is the integer above; this string names the *contract* -- frame shapes,
/// method set, error vocabulary -- so a reviewed protocol change is visible in one place
/// rather than only as an incremented digit.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const LOCAL_MEDIA_WORKER_PROTOCOL: &str = "vanehub.local-media.worker.v1";
/// Requests carry paths and scalars only; a megabyte is already far past what any of them need.
pub(super) const MAX_REQUEST_FRAME_BYTES: usize = 1024 * 1024;
/// Responses carry recognized text. Eight megabytes is well above the product's own output bound
/// and exists to stop a runaway worker from filling memory one line at a time.
pub(super) const MAX_RESPONSE_FRAME_BYTES: usize = 8 * 1024 * 1024;

fn protocol_error() -> LocalMediaError {
    LocalMediaError::new(LocalMediaErrorCode::WorkerProtocolError)
}

/// The worker's opening frame.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct HelloFrame {
    pub(super) v: u8,
    #[serde(rename = "type")]
    pub(super) frame_type: String,
    pub(super) engine: String,
    /// Parsed so a malformed value fails the handshake, but not retained: engine status reports
    /// the probe's version, which is the one a user recognizes.
    #[cfg_attr(not(test), allow(dead_code))]
    #[serde(default)]
    pub(super) package_version: Option<String>,
    #[serde(default)]
    pub(super) capabilities: Vec<String>,
}

/// A reply to one request.
#[derive(Debug, Deserialize)]
pub(super) struct ResponseFrame {
    pub(super) v: u8,
    #[serde(rename = "type")]
    pub(super) frame_type: String,
    pub(super) id: Option<String>,
    pub(super) ok: bool,
    #[serde(default)]
    pub(super) result: Option<Value>,
    #[serde(default)]
    pub(super) error: Option<Value>,
}

pub(super) fn encode_request(request_id: &str, method: &str, params: Value) -> Vec<u8> {
    let frame = json!({
        "v": PROTOCOL_VERSION,
        "type": "request",
        "id": request_id,
        "method": method,
        "params": params,
    });
    serde_json::to_vec(&frame).unwrap_or_default()
}

pub(super) fn encode_control(frame_type: &str, request_id: &str) -> Vec<u8> {
    let frame = json!({ "v": PROTOCOL_VERSION, "type": frame_type, "id": request_id });
    serde_json::to_vec(&frame).unwrap_or_default()
}

/// Validate the handshake.
///
/// The engine identity is checked against the slot that launched the process: a mismatch means the
/// supervisor is holding a worker for a different engine, and letting that through would route an
/// OCR request into a speech model.
pub(super) fn validate_hello(
    line: &[u8],
    expected: LocalMediaEngine,
) -> Result<HelloFrame, LocalMediaError> {
    let hello: HelloFrame = serde_json::from_slice(line).map_err(|_| protocol_error())?;
    if hello.v != PROTOCOL_VERSION || hello.frame_type != "hello" {
        return Err(protocol_error());
    }
    if hello.engine != expected.worker_id() {
        return Err(protocol_error());
    }
    let required = ["probe", expected.inference_method(), "cancel", "shutdown"];
    if !required
        .iter()
        .all(|method| hello.capabilities.iter().any(|value| value == method))
    {
        return Err(protocol_error());
    }
    Ok(hello)
}

/// Parse one response and check that it answers the request that was sent.
pub(super) fn parse_response(
    line: &[u8],
    expected_id: &str,
    call: &WorkerCall,
) -> Result<WorkerReply, LocalMediaError> {
    let frame: ResponseFrame = serde_json::from_slice(line).map_err(|_| protocol_error())?;
    if frame.v != PROTOCOL_VERSION || frame.frame_type != "response" {
        return Err(protocol_error());
    }
    // An id that does not match means the stream is out of step with the request queue. There is
    // no safe way to resynchronize, so the caller terminates the worker.
    if frame.id.as_deref() != Some(expected_id) {
        return Err(protocol_error());
    }
    if !frame.ok {
        return Err(parse_error(frame.error.as_ref()));
    }
    let Some(result) = frame.result else {
        return Err(protocol_error());
    };
    parse_result(call, &result)
}

/// Map a worker-reported error onto the stable vocabulary.
///
/// An unrecognized code is a protocol error rather than a passthrough: the frontend localizes by
/// code, and inventing one at runtime would render as a missing translation.
pub(super) fn parse_error(error: Option<&Value>) -> LocalMediaError {
    let Some(error) = error else {
        return protocol_error();
    };
    let Some(code) = error.get("code").and_then(Value::as_str) else {
        return protocol_error();
    };
    let Some(code) = LocalMediaErrorCode::parse(code) else {
        return protocol_error();
    };
    let mut mapped = LocalMediaError::new(code);
    if let Some(details) = error.get("safeDetails").and_then(Value::as_object) {
        for (key, value) in details {
            mapped = match value {
                Value::String(text) => mapped.with_text(key, text.clone()),
                Value::Bool(flag) => mapped.with_flag(key, *flag),
                Value::Number(number) => match number.as_i64() {
                    Some(integer) => mapped.with_number(key, integer),
                    None => mapped,
                },
                _ => mapped,
            };
        }
    }
    mapped
}

fn parse_result(call: &WorkerCall, result: &Value) -> Result<WorkerReply, LocalMediaError> {
    match call {
        WorkerCall::Probe => Ok(WorkerReply::Probe(ProbeReply {
            package_version: text(result, "packageVersion"),
            device: text(result, "device"),
            model_identity: text(result, "modelIdentity"),
        })),
        WorkerCall::Ocr(_) => {
            let raw_pages = result
                .get("pages")
                .and_then(Value::as_array)
                .ok_or_else(protocol_error)?;
            let mut pages = Vec::with_capacity(raw_pages.len());
            for page in raw_pages {
                pages.push(WorkerPage {
                    page_number: number(page, "pageNumber").unwrap_or(0) as u32,
                    text: page
                        .get("text")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    line_count: number(page, "lineCount").unwrap_or(0) as u32,
                    lines: parse_lines(page),
                });
            }
            Ok(WorkerReply::Ocr(OcrReply {
                character_count: number(result, "characterCount").unwrap_or(0) as u32,
                truncated: flag(result, "truncated"),
                no_text_detected: flag(result, "noTextDetected"),
                engine_version: text(result, "engineVersion"),
                model_identity: text(result, "modelIdentity"),
                pages,
            }))
        }
        WorkerCall::Transcribe(_) => Ok(WorkerReply::Transcribe(TranscribeReply {
            text: result
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            detected_language: text(result, "detectedLanguage"),
            language_probability: result
                .get("languageProbability")
                .and_then(Value::as_f64)
                .map(|value| value as f32),
            duration_ms: number(result, "durationMs").map(|value| value as u64),
            no_speech_detected: flag(result, "noSpeechDetected"),
            engine_version: text(result, "engineVersion"),
            device: text(result, "device"),
        })),
        WorkerCall::Synthesize(_) => {
            let Some(path) = result.get("audioPath").and_then(Value::as_str) else {
                return Err(protocol_error());
            };
            Ok(WorkerReply::Synthesize(SynthesizeReply {
                audio_path: PathBuf::from(path),
                sample_rate: number(result, "sampleRate").unwrap_or(0) as u32,
                sample_count: number(result, "sampleCount").unwrap_or(0) as u64,
                duration_ms: number(result, "durationMs").unwrap_or(0) as u64,
                engine_version: text(result, "engineVersion"),
            }))
        }
    }
}

/// Per-line detail, when the worker reported it.
///
/// A page whose `lines` array is absent or malformed degrades to text-only rather than failing the
/// whole result: the composer never reads this, and OnePiece's contract tolerates a block list
/// without geometry far better than it tolerates no OCR at all.
fn parse_lines(page: &Value) -> Vec<WorkerLine> {
    let Some(raw) = page.get("lines").and_then(Value::as_array) else {
        return Vec::new();
    };
    raw.iter()
        .filter_map(|line| {
            let text = line.get("text").and_then(Value::as_str)?.to_string();
            Some(WorkerLine {
                text,
                confidence: line
                    .get("confidence")
                    .and_then(Value::as_f64)
                    .map(|value| value as f32),
                polygon: parse_polygon(line.get("polygon")),
            })
        })
        .collect()
}

fn parse_polygon(value: Option<&Value>) -> Option<Vec<(f32, f32)>> {
    let points = value?.as_array()?;
    // A polygon with fewer than three points is not one; dropping it beats emitting a degenerate
    // shape an agent might reason about.
    let parsed: Vec<(f32, f32)> = points
        .iter()
        .filter_map(|point| {
            let pair = point.as_array()?;
            let x = pair.first()?.as_f64()? as f32;
            let y = pair.get(1)?.as_f64()? as f32;
            Some((x, y))
        })
        .collect();
    (parsed.len() >= 3).then_some(parsed)
}

fn text(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

fn number(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn insert_optional(map: &mut Map<String, Value>, key: &str, value: &Option<String>) {
    if let Some(text) = value.as_deref().filter(|text| !text.trim().is_empty()) {
        map.insert(key.to_string(), Value::String(text.to_string()));
    }
}

/// Build the params object for one call from the operation's immutable snapshot.
///
/// Reading from the snapshot rather than the live profile is what makes a settings save during an
/// operation invisible to that operation.
pub(super) fn request_params(snapshot: &LocalMediaProfileSnapshot, call: &WorkerCall) -> Value {
    let mut params = Map::new();
    match snapshot.engine() {
        LocalMediaEngine::Ocr => {
            let ocr = snapshot.ocr();
            insert_optional(&mut params, "paddleXConfigPath", &ocr.paddle_x_config_path);
            insert_optional(
                &mut params,
                "textDetectionModelDir",
                &ocr.text_detection_model_dir,
            );
            insert_optional(
                &mut params,
                "textRecognitionModelDir",
                &ocr.text_recognition_model_dir,
            );
            insert_optional(
                &mut params,
                "textLineOrientationModelDir",
                &ocr.text_line_orientation_model_dir,
            );
            params.insert("language".into(), Value::String(ocr.language.clone()));
            params.insert(
                "device".into(),
                Value::String(ocr.device.as_str().to_string()),
            );
        }
        LocalMediaEngine::Stt => {
            let stt = snapshot.stt();
            params.insert(
                "modelDirectory".into(),
                Value::String(stt.model_directory.clone()),
            );
            params.insert(
                "device".into(),
                Value::String(stt.device.as_str().to_string()),
            );
            params.insert(
                "computeType".into(),
                Value::String(stt.compute_type.as_str().to_string()),
            );
            params.insert("language".into(), Value::String(stt.language.clone()));
            params.insert("vadFilter".into(), Value::Bool(stt.vad_filter));
            params.insert("beamSize".into(), Value::from(stt.beam_size));
        }
        LocalMediaEngine::Tts => {
            let tts = snapshot.tts();
            params.insert(
                "modelKind".into(),
                Value::String(tts.model_kind.as_str().to_string()),
            );
            params.insert("modelPath".into(), Value::String(tts.model_path.clone()));
            params.insert("tokensPath".into(), Value::String(tts.tokens_path.clone()));
            insert_optional(&mut params, "lexiconPath", &tts.lexicon_path);
            insert_optional(&mut params, "dataDir", &tts.data_dir);
            insert_optional(&mut params, "dictDir", &tts.dict_dir);
            insert_optional(&mut params, "voicesPath", &tts.voices_path);
            insert_optional(&mut params, "vocoderPath", &tts.vocoder_path);
            params.insert(
                "ruleFsts".into(),
                Value::Array(
                    tts.rule_fsts
                        .iter()
                        .map(|entry| Value::String(entry.clone()))
                        .collect(),
                ),
            );
            params.insert("speakerId".into(), Value::from(tts.speaker_id));
            params.insert("speed".into(), Value::from(tts.speed));
            params.insert("numThreads".into(), Value::from(tts.num_threads));
            params.insert(
                "device".into(),
                Value::String(tts.device.as_str().to_string()),
            );
        }
    }

    match call {
        WorkerCall::Probe => {}
        WorkerCall::Ocr(request) => {
            params.insert(
                "sourcePath".into(),
                Value::String(request.source_path.to_string_lossy().to_string()),
            );
            params.insert(
                "mediaType".into(),
                Value::String(request.media_type.as_str().to_string()),
            );
            params.insert("maxPdfPages".into(), Value::from(request.max_pdf_pages));
            params.insert(
                "maxOutputCharacters".into(),
                Value::from(request.max_output_characters),
            );
        }
        WorkerCall::Transcribe(request) => {
            params.insert(
                "audioPath".into(),
                Value::String(request.audio_path.to_string_lossy().to_string()),
            );
        }
        WorkerCall::Synthesize(request) => {
            params.insert("text".into(), Value::String(request.text.clone()));
            params.insert(
                "outputPath".into(),
                Value::String(request.output_path.to_string_lossy().to_string()),
            );
        }
    }

    Value::Object(params)
}

#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
