//! Profile validation.
//!
//! Shape only: absolute, non-empty, no URL scheme, inside the numeric bounds. Existence and file
//! type belong to infrastructure, because the domain layer may not touch the filesystem. The split
//! matters for a second reason -- a path that exists at save time may not exist at probe time, so
//! "valid" here means "worth trying", not "will work".

use super::engine::LocalMediaEngine;
use super::error::{LocalMediaError, LocalMediaErrorCode};
use super::profile::{
    LocalMediaProfile, TtsModelKind, DEFAULT_PROFILE_ID, MAX_PDF_PAGES_CEILING,
    MAX_RECORDING_SECONDS, MIN_RECORDING_SECONDS,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One rejected field. The field name is a stable identifier the settings page maps to an input,
/// not a sentence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProfileFieldIssue {
    pub(crate) engine: Option<LocalMediaEngine>,
    pub(crate) field: String,
    pub(crate) code: LocalMediaErrorCode,
    /// Carried alongside the code so the settings page renders a field error without maintaining
    /// its own second copy of the code-to-key mapping.
    pub(crate) message_key: String,
}

impl ProfileFieldIssue {
    fn new(engine: Option<LocalMediaEngine>, field: &str, code: LocalMediaErrorCode) -> Self {
        Self {
            engine,
            field: field.to_string(),
            code,
            message_key: code.message_key(),
        }
    }
}

/// Validate the whole profile. A disabled engine contributes no issues at all -- the user must be
/// able to save a half-configured profile so long as the half they have not finished is off.
pub(crate) fn validate_profile(profile: &LocalMediaProfile) -> Vec<ProfileFieldIssue> {
    let mut issues = Vec::new();

    if profile.profile_id != DEFAULT_PROFILE_ID {
        issues.push(ProfileFieldIssue::new(
            None,
            "profileId",
            LocalMediaErrorCode::EngineUnconfigured,
        ));
    }

    if profile.ocr.enabled {
        validate_ocr(profile, &mut issues);
    }
    if profile.stt.enabled {
        validate_stt(profile, &mut issues);
    }
    if profile.tts.enabled {
        validate_tts(profile, &mut issues);
    }

    issues
}

/// The single error a caller reports when validation fails. The first issue drives the code so the
/// composer can distinguish "you have not configured this" from "this file is not there".
pub(crate) fn first_error(issues: &[ProfileFieldIssue]) -> Option<LocalMediaError> {
    issues.first().map(|issue| {
        let mut error = LocalMediaError::new(issue.code).with_text("field", issue.field.clone());
        if let Some(engine) = issue.engine {
            error = error.with_text("engine", engine.as_str());
        }
        error
    })
}

fn validate_ocr(profile: &LocalMediaProfile, issues: &mut Vec<ProfileFieldIssue>) {
    let engine = Some(LocalMediaEngine::Ocr);
    let ocr = &profile.ocr;

    push_executable(engine, "pythonExecutable", &ocr.python_executable, issues);

    let has_paddle_x = is_configured(&ocr.paddle_x_config_path);
    let has_detection = is_configured(&ocr.text_detection_model_dir);
    let has_recognition = is_configured(&ocr.text_recognition_model_dir);

    // Either form is acceptable; neither is not. Reporting both directory fields when the user
    // chose the PaddleX form would send them to fix the wrong input.
    if !has_paddle_x && !(has_detection && has_recognition) {
        if has_detection || has_recognition {
            if !has_detection {
                issues.push(ProfileFieldIssue::new(
                    engine,
                    "textDetectionModelDir",
                    LocalMediaErrorCode::ModelNotConfigured,
                ));
            }
            if !has_recognition {
                issues.push(ProfileFieldIssue::new(
                    engine,
                    "textRecognitionModelDir",
                    LocalMediaErrorCode::ModelNotConfigured,
                ));
            }
        } else {
            issues.push(ProfileFieldIssue::new(
                engine,
                "paddleXConfigPath",
                LocalMediaErrorCode::ModelNotConfigured,
            ));
        }
    }

    push_optional_path(
        engine,
        "paddleXConfigPath",
        &ocr.paddle_x_config_path,
        issues,
    );
    push_optional_path(
        engine,
        "textDetectionModelDir",
        &ocr.text_detection_model_dir,
        issues,
    );
    push_optional_path(
        engine,
        "textRecognitionModelDir",
        &ocr.text_recognition_model_dir,
        issues,
    );
    push_optional_path(
        engine,
        "textLineOrientationModelDir",
        &ocr.text_line_orientation_model_dir,
        issues,
    );

    if ocr.language.trim().is_empty() || ocr.language.len() > 32 {
        issues.push(ProfileFieldIssue::new(
            engine,
            "language",
            LocalMediaErrorCode::DeviceConfigurationInvalid,
        ));
    }
    if ocr.max_pdf_pages == 0 || ocr.max_pdf_pages > MAX_PDF_PAGES_CEILING {
        issues.push(ProfileFieldIssue::new(
            engine,
            "maxPdfPages",
            LocalMediaErrorCode::DeviceConfigurationInvalid,
        ));
    }
}

fn validate_stt(profile: &LocalMediaProfile, issues: &mut Vec<ProfileFieldIssue>) {
    let engine = Some(LocalMediaEngine::Stt);
    let stt = &profile.stt;

    push_executable(engine, "pythonExecutable", &stt.python_executable, issues);
    push_required_path(engine, "modelDirectory", &stt.model_directory, issues);

    if !(1..=10).contains(&stt.beam_size) {
        issues.push(ProfileFieldIssue::new(
            engine,
            "beamSize",
            LocalMediaErrorCode::DeviceConfigurationInvalid,
        ));
    }
    if !(MIN_RECORDING_SECONDS..=MAX_RECORDING_SECONDS).contains(&stt.max_recording_seconds) {
        issues.push(ProfileFieldIssue::new(
            engine,
            "maxRecordingSeconds",
            LocalMediaErrorCode::DeviceConfigurationInvalid,
        ));
    }
    if stt.language.trim().is_empty() || stt.language.len() > 32 {
        issues.push(ProfileFieldIssue::new(
            engine,
            "language",
            LocalMediaErrorCode::DeviceConfigurationInvalid,
        ));
    }
}

fn validate_tts(profile: &LocalMediaProfile, issues: &mut Vec<ProfileFieldIssue>) {
    let engine = Some(LocalMediaEngine::Tts);
    let tts = &profile.tts;

    push_executable(engine, "pythonExecutable", &tts.python_executable, issues);
    push_required_path(engine, "modelPath", &tts.model_path, issues);
    push_required_path(engine, "tokensPath", &tts.tokens_path, issues);

    // Model-kind-specific requirements. A single permissive path list would accept a kokoro voices
    // file in place of a matcha vocoder and fail inside the native library instead of here.
    match tts.model_kind {
        TtsModelKind::Kokoro => {
            push_required_optional(engine, "voicesPath", &tts.voices_path, issues);
        }
        TtsModelKind::Matcha => {
            push_required_optional(engine, "vocoderPath", &tts.vocoder_path, issues);
        }
        TtsModelKind::Vits | TtsModelKind::Piper => {}
    }

    push_optional_path(engine, "lexiconPath", &tts.lexicon_path, issues);
    push_optional_path(engine, "dataDir", &tts.data_dir, issues);
    push_optional_path(engine, "dictDir", &tts.dict_dir, issues);
    push_optional_path(engine, "voicesPath", &tts.voices_path, issues);
    push_optional_path(engine, "vocoderPath", &tts.vocoder_path, issues);

    for entry in &tts.rule_fsts {
        if !is_valid_path(entry) {
            issues.push(ProfileFieldIssue::new(
                engine,
                "ruleFsts",
                LocalMediaErrorCode::ModelNotConfigured,
            ));
            break;
        }
    }

    if !(0.5..=2.0).contains(&tts.speed) {
        issues.push(ProfileFieldIssue::new(
            engine,
            "speed",
            LocalMediaErrorCode::DeviceConfigurationInvalid,
        ));
    }
    if !(1..=16).contains(&tts.num_threads) {
        issues.push(ProfileFieldIssue::new(
            engine,
            "numThreads",
            LocalMediaErrorCode::DeviceConfigurationInvalid,
        ));
    }
}

fn is_configured(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|path| !path.trim().is_empty())
}

/// Absolute, free of control characters and URL schemes, and not a bare relative name.
///
/// `Path::is_absolute` is platform-dependent by design: `C:\models` is not absolute on Linux and
/// `/models` is not absolute on Windows. That asymmetry is the point -- a profile carrying the
/// other platform's paths cannot be launched here, and saying so at validation time is better than
/// a `MODEL_NOT_FOUND` three layers down.
pub(crate) fn is_valid_path(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 4_096 {
        return false;
    }
    if trimmed
        .chars()
        .any(|character| character.is_control() || character == '\0')
    {
        return false;
    }
    if has_url_scheme(trimmed) {
        return false;
    }
    Path::new(trimmed).is_absolute()
}

/// Reject `file://`, `http://`, `smb://` and friends before the path parser sees them. Checking for
/// `://` alone would miss `file:relative`, which some resolvers still accept.
fn has_url_scheme(value: &str) -> bool {
    let Some(index) = value.find(':') else {
        return false;
    };
    // A Windows drive letter is the one legitimate single-character prefix before a colon.
    if index == 1 {
        return false;
    }
    value[..index]
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '+')
        && index > 1
}

fn push_executable(
    engine: Option<LocalMediaEngine>,
    field: &str,
    value: &str,
    issues: &mut Vec<ProfileFieldIssue>,
) {
    if value.trim().is_empty() {
        issues.push(ProfileFieldIssue::new(
            engine,
            field,
            LocalMediaErrorCode::EngineUnconfigured,
        ));
    } else if !is_valid_path(value) {
        issues.push(ProfileFieldIssue::new(
            engine,
            field,
            LocalMediaErrorCode::PythonNotFound,
        ));
    }
}

fn push_required_path(
    engine: Option<LocalMediaEngine>,
    field: &str,
    value: &str,
    issues: &mut Vec<ProfileFieldIssue>,
) {
    // Empty and malformed are the same answer here: the engine is enabled and this path is not
    // usable. The settings page distinguishes them by whether the input has any text in it.
    if value.trim().is_empty() || !is_valid_path(value) {
        issues.push(ProfileFieldIssue::new(
            engine,
            field,
            LocalMediaErrorCode::ModelNotConfigured,
        ));
    }
}

fn push_required_optional(
    engine: Option<LocalMediaEngine>,
    field: &str,
    value: &Option<String>,
    issues: &mut Vec<ProfileFieldIssue>,
) {
    match value.as_deref() {
        Some(path) if !path.trim().is_empty() => {}
        _ => issues.push(ProfileFieldIssue::new(
            engine,
            field,
            LocalMediaErrorCode::ModelNotConfigured,
        )),
    }
}

fn push_optional_path(
    engine: Option<LocalMediaEngine>,
    field: &str,
    value: &Option<String>,
    issues: &mut Vec<ProfileFieldIssue>,
) {
    if let Some(path) = value.as_deref() {
        if !path.trim().is_empty() && !is_valid_path(path) {
            issues.push(ProfileFieldIssue::new(
                engine,
                field,
                LocalMediaErrorCode::ModelNotConfigured,
            ));
        }
    }
}
