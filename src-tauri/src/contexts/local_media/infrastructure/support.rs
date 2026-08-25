//! Small adapters: clock, identifiers, the operations bridge, diagnostics, and resource lookup.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::contexts::local_media::application::ports::{
    LocalMediaClock, LocalMediaDiagnostics, OpaqueIdFactory, OperationBridge,
};
use crate::contexts::local_media::domain::{LocalMediaError, LocalMediaErrorCode};
use crate::contexts::operations::api::{
    DiagnosticLog, DiagnosticLogPort, OperationKind, OperationsApi,
};
use crate::contexts::operations::application::LogSeverity;

pub(crate) struct SystemLocalMediaClock;

impl LocalMediaClock for SystemLocalMediaClock {
    fn now_iso(&self) -> String {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }

    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as u64)
            .unwrap_or(0)
    }
}

pub(crate) struct RandomIdFactory;

impl OpaqueIdFactory for RandomIdFactory {
    /// 32 hex digits from a v4 UUID.
    ///
    /// The shape is what the domain's `parse` accepts, and it matters beyond tidiness: these values
    /// become directory names, so anything that could carry a separator would have to be rejected
    /// somewhere further down where the check is easier to forget.
    fn next(&self, prefix: &str) -> String {
        format!("{prefix}{}", uuid::Uuid::new_v4().simple())
    }
}

/// Bridges the context onto the generic operations runtime.
///
/// Everything media-specific -- phases, typed results, retention -- stays in `local_media`. What
/// crosses here is only what every operation has: an id, a coarse kind, a phase line, and a
/// terminal state.
pub(crate) struct OperationsApiBridge {
    operations: OperationsApi,
}

impl OperationsApiBridge {
    pub(crate) fn new(operations: OperationsApi) -> Self {
        Self { operations }
    }
}

impl OperationBridge for OperationsApiBridge {
    fn start(&self, kind: &str, message_key: &str) -> Result<String, LocalMediaError> {
        self.operations
            .start(
                OperationKind::LocalMedia,
                Some(kind.to_string()),
                Some(message_key.to_string()),
            )
            .map(|task| task.id)
            .map_err(|_| LocalMediaError::new(LocalMediaErrorCode::EngineUnavailable))
    }

    fn phase(&self, operation_id: &str, phase: &str) {
        // The phase name is the whole log line. Nothing about the media -- not the file, not the
        // text, not the duration in a form that could identify content -- is appended.
        let _ = self.operations.append_log(operation_id, phase.to_string());
    }

    fn succeed(&self, operation_id: &str) {
        let _ = self.operations.complete(operation_id, None);
    }

    fn fail(&self, operation_id: &str, code: &str) {
        let _ = self.operations.fail(operation_id, code.to_string());
    }

    fn cancel(&self, operation_id: &str) {
        let _ = self.operations.cancel(operation_id);
    }

    fn cancellation_flag(&self, operation_id: &str) -> Arc<AtomicBool> {
        self.operations
            .cancellation_flag(operation_id)
            .unwrap_or_else(|_| Arc::new(AtomicBool::new(false)))
    }

    fn is_cancelled(&self, operation_id: &str) -> bool {
        self.operations
            .cancellation_flag(operation_id)
            .map(|flag| flag.load(std::sync::atomic::Ordering::SeqCst))
            .unwrap_or(false)
    }
}

pub(super) fn render_diagnostic_context(fields: &[(&str, String)]) -> BTreeMap<String, String> {
    fields
        .iter()
        .map(|(key, value)| ((*key).to_string(), value.clone()))
        .collect()
}

pub(crate) struct UnifiedLocalMediaDiagnostics {
    port: Arc<dyn DiagnosticLogPort>,
}

impl UnifiedLocalMediaDiagnostics {
    pub(crate) fn new(port: Arc<dyn DiagnosticLogPort>) -> Self {
        Self { port }
    }
}

impl LocalMediaDiagnostics for UnifiedLocalMediaDiagnostics {
    /// The event name is the message and the fields are the context.
    ///
    /// There is no free-text parameter by construction. A `message: &str` here would be the one
    /// place a transcript or a path could enter the unified log without anyone noticing, and the
    /// redaction layer downstream is a safety net, not a design.
    fn record(&self, event: &str, fields: &[(&str, String)]) {
        let _ = self.port.write_diagnostic(DiagnosticLog {
            severity: LogSeverity::Info,
            category: "local_media.operation".to_string(),
            message: event.to_string(),
            context: render_diagnostic_context(fields),
        });
    }
}

/// Find the packaged Python bridge.
///
/// A directory only counts when it actually contains the worker package's entry point. An empty
/// `local-media-worker/` in a bundle would otherwise be accepted and turn every launch into an
/// import error instead of an honest "the bridge is not packaged".
pub(crate) fn resolve_worker_bridge_root(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|candidate| has_worker_package(candidate))
        .cloned()
}

fn has_worker_package(root: &Path) -> bool {
    root.join("vane_local_media_worker")
        .join("__main__.py")
        .is_file()
}

#[cfg(test)]
#[path = "support_tests.rs"]
mod tests;
