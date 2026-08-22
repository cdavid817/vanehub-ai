use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::contexts::local_media::api::LocalMediaApi;
use crate::contexts::local_media::application::LocalMediaDependencies;
use crate::contexts::local_media::domain::AdmissionLimits;
use crate::contexts::local_media::infrastructure::{
    build_supervisor, resolve_worker_bridge_root, CpalAudioCapture, CpalDeviceCatalog,
    FilesystemMediaTempStore, OperationsApiBridge, RandomIdFactory, RodioPlayback,
    SqliteLocalMediaProfileRepository, SystemLocalMediaClock, UnifiedLocalMediaDiagnostics,
};
use crate::contexts::operations::api::{DiagnosticLogPort, OperationsApi};
use crate::platform::database::NativeDatabase;

/// Where ephemeral media lives, relative to the application data root.
const MEDIA_ROOT: &str = "local-media";
/// The worker's current directory: application-owned and always empty, so a relative import or a
/// stray write from a Python library cannot land somewhere meaningful.
const WORKER_CWD: &str = "local-media/worker-cwd";

pub(crate) fn assemble_local_media_api(
    database: NativeDatabase,
    operations_api: OperationsApi,
    diagnostics: Arc<dyn DiagnosticLogPort>,
    data_root: &Path,
    bridge_candidates: &[PathBuf],
) -> LocalMediaApi {
    let clock = Arc::new(SystemLocalMediaClock);
    let ids = Arc::new(RandomIdFactory);
    let media_root = data_root.join(MEDIA_ROOT);
    let worker_cwd = data_root.join(WORKER_CWD);

    // An absent bridge is not a bootstrap failure. The context assembles, reports every engine as
    // unavailable at its first probe, and the settings page explains why -- which is a better
    // outcome than refusing to start the application because a resource is missing.
    let bridge_root = resolve_worker_bridge_root(bridge_candidates)
        .unwrap_or_else(|| media_root.join("missing-worker-bridge"));

    let temp = Arc::new(FilesystemMediaTempStore::new(
        media_root.clone(),
        ids.clone(),
        clock.clone(),
        AdmissionLimits::HARD_CEILING,
    ));

    LocalMediaApi::new(LocalMediaDependencies {
        repository: Arc::new(SqliteLocalMediaProfileRepository::new(
            database,
            clock.clone(),
        )),
        clock,
        ids,
        temp,
        workers: Arc::new(build_supervisor(bridge_root, media_root, worker_cwd)),
        capture: Arc::new(CpalAudioCapture::new()),
        playback: Arc::new(RodioPlayback::new()),
        devices: Arc::new(CpalDeviceCatalog),
        operations: Arc::new(OperationsApiBridge::new(operations_api)),
        diagnostics: Arc::new(UnifiedLocalMediaDiagnostics::new(diagnostics)),
    })
}

/// Candidate locations for the packaged Python bridge, most specific first.
///
/// The development path comes first so a `cargo run` uses the working tree rather than a stale copy
/// in an installed bundle. The `_up_` variant exists because Tauri rewrites a `../`-relative
/// resource into that subdirectory when it packages one.
pub(crate) fn worker_bridge_candidates(resource_dir: Option<PathBuf>) -> Vec<PathBuf> {
    let mut candidates = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("local-media-worker")];
    if let Some(resources) = resource_dir {
        candidates.push(resources.join("resources").join("local-media-worker"));
        candidates.push(
            resources
                .join("_up_")
                .join("resources")
                .join("local-media-worker"),
        );
        candidates.push(resources.join("local-media-worker"));
    }
    candidates
}

#[cfg(test)]
#[path = "local_media_tests.rs"]
mod tests;
