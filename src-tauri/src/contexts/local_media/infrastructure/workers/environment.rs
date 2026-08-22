//! The environment a worker process runs in.
//!
//! Built as an allowlist over the parent environment rather than a denylist. A denylist would need
//! updating every time the application gains a credential variable, and the failure mode of
//! forgetting is that a provider key is handed to a Python process whose whole purpose is to never
//! reach the network.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Names carried through from the parent. Everything here is needed to *start* an interpreter or to
/// let it find its own standard library and temporary directory; none of it is application state.
const INHERITED: &[&str] = &[
    // Locating the interpreter's own runtime.
    "PATH",
    "HOME",
    "USERPROFILE",
    "SYSTEMROOT",
    "SYSTEMDRIVE",
    "WINDIR",
    "COMSPEC",
    "PATHEXT",
    // Temporary directories, for libraries that insist on one.
    "TMP",
    "TEMP",
    "TMPDIR",
    // Locale, so a model that reports language names does not depend on the C locale.
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    // Dynamic-loader paths the user's own Python installation may depend on.
    "LD_LIBRARY_PATH",
    "DYLD_LIBRARY_PATH",
    // Display/session bits some audio and GPU stacks probe even in a headless worker.
    "XDG_RUNTIME_DIR",
    "CUDA_VISIBLE_DEVICES",
];

/// Build the complete environment for a worker.
///
/// The result is used with a spawn that clears the parent environment first, so anything absent
/// here is absent in the child. That is the point: proxy variables are not overwritten with empty
/// strings and hoped over, they simply do not exist.
pub(super) fn worker_environment(
    bridge_root: &Path,
    media_root: &Path,
    parent: &BTreeMap<String, String>,
    overlay: &WorkerEnvironmentOverlay,
) -> BTreeMap<String, String> {
    let mut environment = BTreeMap::new();
    for name in INHERITED {
        if let Some(value) = parent.get(*name) {
            environment.insert((*name).to_string(), value.clone());
        }
    }

    // Offline flags for the model ecosystems these engines sit on.
    environment.insert("HF_HUB_OFFLINE".to_string(), "1".to_string());
    environment.insert("TRANSFORMERS_OFFLINE".to_string(), "1".to_string());

    // The bridge comes first on the path. Extending the *parent's* `PYTHONPATH` would let an
    // unrelated entry shadow `vane_local_media_worker`; the overlay is appended after the bridge,
    // so a test build can substitute third-party packages without ever displacing the real worker.
    let mut python_path = bridge_root.to_string_lossy().to_string();
    for extra in &overlay.python_path_suffix {
        python_path.push(SEPARATOR);
        python_path.push_str(&extra.to_string_lossy());
    }
    environment.insert("PYTHONPATH".to_string(), python_path);
    environment.insert("PYTHONUNBUFFERED".to_string(), "1".to_string());
    environment.insert("PYTHONNOUSERSITE".to_string(), "1".to_string());
    environment.insert("PYTHONDONTWRITEBYTECODE".to_string(), "1".to_string());

    // The worker repeats the host's containment check against this root.
    environment.insert(
        "VANEHUB_LOCAL_MEDIA_ROOT".to_string(),
        media_root.to_string_lossy().to_string(),
    );

    for (name, value) in &overlay.variables {
        environment.insert(name.clone(), value.clone());
    }

    environment
}

#[cfg(windows)]
const SEPARATOR: char = ';';
#[cfg(not(windows))]
const SEPARATOR: char = ':';

/// Additions a test build may make to the worker environment.
///
/// Empty in every production assembly, and the allowlist above still decides what is inherited --
/// this only adds, it cannot widen inheritance.
#[derive(Debug, Default, Clone)]
pub(crate) struct WorkerEnvironmentOverlay {
    pub(crate) python_path_suffix: Vec<PathBuf>,
    pub(crate) variables: BTreeMap<String, String>,
}

/// Snapshot the current process environment in the shape `worker_environment` expects.
pub(super) fn current_environment() -> BTreeMap<String, String> {
    std::env::vars().collect()
}

#[cfg(test)]
#[path = "environment_tests.rs"]
mod tests;
