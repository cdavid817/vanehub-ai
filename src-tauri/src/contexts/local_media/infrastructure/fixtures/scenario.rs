use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Runtime switch for the fixture assembly.
///
/// Read only from this module, which exists only under `desktop-e2e`. A production build has no
/// code that looks at this name, so there is nothing for a URL, a storage key, a window global, a
/// settings toggle, or a command to turn on.
const ACTIVATION: &str = "VANEHUB_LOCAL_MEDIA_E2E_FIXTURES";

/// Where the scenario document lives. The launcher forwards it to the worker so the test-only
/// Python packages read the same script the Rust fixtures do.
const SCENARIO_FILE: &str = "VANEHUB_LOCAL_MEDIA_E2E_SCENARIO_FILE";

/// Directory holding the test-only `paddleocr`, `faster_whisper`, and `sherpa_onnx` packages.
const PYTHON_FIXTURE_ROOT: &str = "VANEHUB_LOCAL_MEDIA_E2E_PYTHON_ROOT";

/// Image or PDF the fixture picker returns in place of a system file dialog.
const OCR_SOURCE: &str = "VANEHUB_LOCAL_MEDIA_E2E_OCR_SOURCE";

/// Behaviours the Rust-side fixtures accept. Anything else is a configuration error, because a
/// silently-ignored typo in a scenario would turn a failure test into a passing success test.
const CAPTURE_BEHAVIOURS: [&str; 8] = [
    "success",
    "permission_denied",
    "device_unavailable",
    "device_disconnected",
    "start_failure",
    "too_short",
    "overrun",
    // Produces a valid recording that also reports the ceiling was hit, which is a warning on a
    // successful transcription rather than a failure.
    "limit_reached",
];
const PLAYBACK_BEHAVIOURS: [&str; 2] = ["success", "failure"];
const DEVICE_BEHAVIOURS: [&str; 3] = ["success", "no_devices", "enumeration_failure"];

/// Everything the bootstrap needs when fixtures are on.
pub(crate) struct FixtureActivation {
    /// Appended to the worker's `PYTHONPATH` after the production bridge root, so the real
    /// `vane_local_media_worker` still wins and only the third-party packages are shadowed.
    pub(crate) python_root: PathBuf,
    pub(crate) scenario_file: Option<PathBuf>,
    pub(crate) ocr_source: PathBuf,
}

impl FixtureActivation {
    /// Extra worker environment entries. Deliberately narrow: one name, fixture-only.
    pub(crate) fn worker_environment(&self) -> BTreeMap<String, String> {
        let mut extra = BTreeMap::new();
        if let Some(scenario) = &self.scenario_file {
            extra.insert(
                SCENARIO_FILE.to_string(),
                scenario.to_string_lossy().to_string(),
            );
        }
        extra
    }

    pub(crate) fn python_path_suffix(&self) -> Option<&Path> {
        Some(self.python_root.as_path())
    }
}

/// Abort with a stable, greppable message.
///
/// A panic rather than a fallback: once the activation variable says "fixtures", quietly running
/// the production assembly would let an E2E suite pass while testing something else entirely, and
/// that is a worse outcome than a loud failure in a build that only exists for testing.
fn misconfigured(detail: &str) -> ! {
    panic!("local-media fixture configuration error: {detail}");
}

/// Resolve the activation, or `None` when the runtime switch is absent.
///
/// Absent is the default even in a `desktop-e2e` build: the ordinary Desktop Smoke layer runs the
/// same artifact and must keep exercising the production assembly.
pub(crate) fn fixture_activation() -> Option<FixtureActivation> {
    if std::env::var(ACTIVATION).ok().as_deref() != Some("1") {
        return None;
    }

    let Ok(python_root) = std::env::var(PYTHON_FIXTURE_ROOT) else {
        misconfigured(&format!(
            "{ACTIVATION}=1 but {PYTHON_FIXTURE_ROOT} is not set"
        ));
    };
    let python_root = PathBuf::from(python_root);
    if !python_root.is_dir() {
        misconfigured(&format!("{PYTHON_FIXTURE_ROOT} is not a directory"));
    }
    for package in ["paddleocr", "faster_whisper", "sherpa_onnx"] {
        if !python_root.join(package).is_dir() {
            misconfigured(&format!("{PYTHON_FIXTURE_ROOT} is missing `{package}`"));
        }
    }

    let Ok(ocr_source) = std::env::var(OCR_SOURCE) else {
        misconfigured(&format!("{ACTIVATION}=1 but {OCR_SOURCE} is not set"));
    };
    let ocr_source = PathBuf::from(ocr_source);
    if !ocr_source.is_file() {
        misconfigured(&format!("{OCR_SOURCE} is not a file"));
    }

    let scenario_file = match std::env::var(SCENARIO_FILE) {
        Ok(value) => {
            let path = PathBuf::from(value);
            let Some(parent) = path.parent() else {
                misconfigured(&format!("{SCENARIO_FILE} has no parent directory"));
            };
            if !parent.is_dir() {
                misconfigured(&format!("{SCENARIO_FILE}'s directory does not exist"));
            }
            // Validated eagerly so a malformed script fails at startup rather than mid-scenario.
            FixtureScenario::load(Some(path.as_path()));
            Some(path)
        }
        Err(_) => None,
    };

    Some(FixtureActivation {
        python_root,
        scenario_file,
        ocr_source,
    })
}

/// The behaviours the Rust-side fixtures honour, read from the same document the Python side uses.
#[derive(Debug, Clone)]
pub(crate) struct FixtureScenario {
    pub(crate) capture: String,
    pub(crate) playback: String,
    pub(crate) devices: String,
}

impl Default for FixtureScenario {
    fn default() -> Self {
        Self {
            capture: "success".to_string(),
            playback: "success".to_string(),
            devices: "success".to_string(),
        }
    }
}

impl FixtureScenario {
    /// Re-read per call rather than cached: a spec changes the script between interactions with a
    /// process that stays alive, which is how permission-denied-then-recover is exercised.
    ///
    /// Only a completely absent scenario path means "default success". Once a path is configured,
    /// a missing file, malformed JSON, a wrongly-typed section, or an unknown behaviour is a
    /// configuration error, because each of those would otherwise read as a passing test.
    pub(crate) fn load(path: Option<&Path>) -> Self {
        let Some(path) = path else {
            return Self::default();
        };
        let Ok(raw) = std::fs::read_to_string(path) else {
            misconfigured("the scenario file could not be read");
        };
        let Ok(document) = serde_json::from_str::<serde_json::Value>(&raw) else {
            misconfigured("the scenario file is not valid JSON");
        };
        if !document.is_object() {
            misconfigured("the scenario file is not a JSON object");
        }

        Self {
            capture: read_behaviour(&document, "capture", &CAPTURE_BEHAVIOURS),
            playback: read_behaviour(&document, "playback", &PLAYBACK_BEHAVIOURS),
            devices: read_behaviour(&document, "devices", &DEVICE_BEHAVIOURS),
        }
    }
}

fn read_behaviour(document: &serde_json::Value, section: &str, allowed: &[&str]) -> String {
    let Some(entry) = document.get(section) else {
        return "success".to_string();
    };
    if !entry.is_object() {
        misconfigured(&format!("scenario section `{section}` is not an object"));
    }
    let Some(behaviour) = entry.get("behaviour") else {
        return "success".to_string();
    };
    let Some(behaviour) = behaviour.as_str() else {
        misconfigured(&format!("scenario `{section}.behaviour` is not a string"));
    };
    if !allowed.contains(&behaviour) {
        misconfigured(&format!(
            "scenario `{section}.behaviour` is `{behaviour}`, expected one of {allowed:?}"
        ));
    }
    behaviour.to_string()
}

#[cfg(test)]
#[path = "scenario_tests.rs"]
mod tests;
