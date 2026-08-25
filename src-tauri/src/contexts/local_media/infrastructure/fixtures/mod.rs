//! Deterministic stand-ins for the hardware and interaction boundaries CI cannot provide.
//!
//! Compiled only under `desktop-e2e`, and even then assembled only when the runtime variable is
//! set, so a build without the feature contains none of these types. Everything *between* the
//! composer and the engine stays production code: the commands, `LocalMediaApi`, the application
//! service, the operation store, the temp store, the supervisor, the launcher, the child-process
//! transport, the JSON Lines protocol, and every cancel, grace, crash and restart rule.
//!
//! What is replaced here is only what a headless runner genuinely cannot supply: a microphone, a
//! speaker, a device enumerator, and a human choosing a file.

mod scenario;

pub(crate) mod audio;

pub(crate) use audio::{FixtureAudioCapture, FixtureAudioDeviceCatalog, FixtureAudioPlayback};
pub(crate) use scenario::{fixture_activation, FixtureActivation};

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// The image or PDF the fixture picker hands back in place of a system file dialog.
///
/// Only the *choice* of file is replaced. Everything after it -- content sniffing, the size, page
/// and pixel ceilings, the opaque staged id, the copy into staging, the one-time claim, the
/// operation, the real worker, and cleanup -- is the production path.
static OCR_SOURCE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

fn ocr_source_cell() -> &'static Mutex<Option<PathBuf>> {
    OCR_SOURCE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn publish_ocr_source(path: PathBuf) {
    if let Ok(mut cell) = ocr_source_cell().lock() {
        *cell = Some(path);
    }
}

/// `None` when fixtures are not active, which the command turns into a failure rather than a
/// fallback to the real dialog: a headless runner has nobody to answer it.
pub(crate) fn fixture_ocr_source() -> Option<PathBuf> {
    ocr_source_cell().lock().ok()?.clone()
}
