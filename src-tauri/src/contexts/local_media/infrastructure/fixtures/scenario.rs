use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Runtime switch for the fixture assembly.
///
/// Read only from this module, which exists only under `desktop-e2e`. A production build has no
/// code that looks at this name, so there is nothing for a URL, a storage key, a window global, a
/// settings toggle, or a Tauri command to turn on.
const ACTIVATION: &str = "VANEHUB_LOCAL_MEDIA_E2E_FIXTURES";

/// Where the scenario document lives. The launcher forwards it to the worker so the test-only
/// Python packages read the same script the Rust fixtures do.
const SCENARIO_FILE: &str = "VANEHUB_LOCAL_MEDIA_E2E_SCENARIO_FILE";

/// Directory holding the test-only `paddleocr`, `faster_whisper`, and `sherpa_onnx` packages.
const PYTHON_FIXTURE_ROOT: &str = "VANEHUB_LOCAL_MEDIA_E2E_PYTHON_ROOT";

/// Everything the bootstrap needs when fixtures are on.
pub(crate) struct FixtureActivation {
    /// Appended to the worker's `PYTHONPATH` after the production bridge root, so the real
    /// `vane_local_media_worker` still wins and only the third-party packages are shadowed.
    pub(crate) python_root: PathBuf,
    /// Forwarded into the worker environment; absent when no scenario is scripted.
    pub(crate) scenario_file: Option<PathBuf>,
}

impl FixtureActivation {
    /// Extra worker environment entries. Deliberately narrow: two names, both fixture-only.
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

/// Resolve the activation, or `None` when the runtime switch is absent.
///
/// Being off is the default even in a `desktop-e2e` build: the ordinary Desktop Smoke layer runs
/// the same artifact and must keep exercising the production assembly.
pub(crate) fn fixture_activation() -> Option<FixtureActivation> {
    if std::env::var(ACTIVATION).ok().as_deref() != Some("1") {
        return None;
    }
    let python_root = PathBuf::from(std::env::var(PYTHON_FIXTURE_ROOT).ok()?);
    if !python_root.is_dir() {
        return None;
    }
    let scenario_file = std::env::var(SCENARIO_FILE).ok().map(PathBuf::from);
    Some(FixtureActivation {
        python_root,
        scenario_file,
    })
}

/// The behaviours the Rust-side fixtures honour, read from the same document the Python side uses.
#[derive(Debug, Clone, Default)]
pub(crate) struct FixtureScenario {
    pub(crate) capture: String,
    pub(crate) playback: String,
    pub(crate) devices: String,
}

impl FixtureScenario {
    /// Re-read per call rather than cached: a spec changes the script between interactions with a
    /// process that stays alive, which is how permission-denied-then-recover is exercised.
    pub(crate) fn load(path: Option<&Path>) -> Self {
        let Some(path) = path else {
            return Self::default();
        };
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        let Ok(document) = serde_json::from_str::<serde_json::Value>(&raw) else {
            return Self::default();
        };
        let read = |section: &str| {
            document
                .get(section)
                .and_then(|entry| entry.get("behaviour"))
                .and_then(|value| value.as_str())
                .unwrap_or("success")
                .to_string()
        };
        Self {
            capture: read("capture"),
            playback: read("playback"),
            devices: read("devices"),
        }
    }
}
