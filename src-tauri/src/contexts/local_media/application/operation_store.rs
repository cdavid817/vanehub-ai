//! Typed local-media results, keyed by operation id.
//!
//! The generic operations context already stores status, logs, and an untyped `result` JSON blob.
//! This store exists alongside it because the media results are typed, are read only by the
//! composer that started the work, and must expire on their own schedule -- putting recognized text
//! into the shared operation record would make it visible to every generic operation consumer.

use super::super::domain::{
    LocalMediaError, LocalMediaErrorCode, LocalMediaOperationKind, LocalMediaOperationResult,
    LocalMediaPhase,
};
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone)]
struct StoredOperation {
    kind: LocalMediaOperationKind,
    phase: LocalMediaPhase,
    accepted_at_ms: u64,
    completed_at_ms: Option<u64>,
    result: Option<LocalMediaOperationResult>,
    error: Option<LocalMediaError>,
}

impl StoredOperation {
    fn is_terminal(&self) -> bool {
        self.phase.is_terminal()
    }
}

pub(crate) struct LocalMediaOperationStore {
    entries: Mutex<HashMap<String, StoredOperation>>,
    retention_ms: u64,
    capacity: usize,
}

impl LocalMediaOperationStore {
    pub(crate) fn new(retention_ms: u64, capacity: usize) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            retention_ms,
            capacity: capacity.max(1),
        }
    }

    pub(crate) fn accept(&self, operation_id: &str, kind: LocalMediaOperationKind, now_ms: u64) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        entries.insert(
            operation_id.to_string(),
            StoredOperation {
                kind,
                phase: LocalMediaPhase::Accepted,
                accepted_at_ms: now_ms,
                completed_at_ms: None,
                result: None,
                error: None,
            },
        );
        Self::evict(&mut entries, self.capacity);
    }

    /// Advance the phase. A terminal phase is final: a late progress report from a worker thread
    /// that lost the cancellation race must not resurrect a settled operation.
    pub(crate) fn set_phase(&self, operation_id: &str, phase: LocalMediaPhase) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        let Some(entry) = entries.get_mut(operation_id) else {
            return;
        };
        if entry.is_terminal() {
            return;
        }
        entry.phase = phase;
    }

    pub(crate) fn succeed(
        &self,
        operation_id: &str,
        result: LocalMediaOperationResult,
        now_ms: u64,
    ) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        let Some(entry) = entries.get_mut(operation_id) else {
            return;
        };
        if entry.is_terminal() {
            // Already cancelled or failed. Dropping the result here is what stops a cancelled
            // transcription from reaching the draft.
            return;
        }
        entry.phase = LocalMediaPhase::Succeeded;
        entry.completed_at_ms = Some(now_ms);
        entry.result = Some(result);
    }

    pub(crate) fn fail(&self, operation_id: &str, error: LocalMediaError, now_ms: u64) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        let Some(entry) = entries.get_mut(operation_id) else {
            return;
        };
        if entry.is_terminal() {
            return;
        }
        entry.phase = LocalMediaPhase::Failed;
        entry.completed_at_ms = Some(now_ms);
        entry.error = Some(error);
    }

    pub(crate) fn cancel(&self, operation_id: &str, now_ms: u64) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        let Some(entry) = entries.get_mut(operation_id) else {
            return;
        };
        if entry.is_terminal() {
            return;
        }
        entry.phase = LocalMediaPhase::Cancelled;
        entry.completed_at_ms = Some(now_ms);
        entry.error = Some(LocalMediaError::new(
            LocalMediaErrorCode::OperationCancelled,
        ));
    }

    /// Exercised by this module's tests; production reads the state it derives, not this.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn phase(&self, operation_id: &str) -> Option<LocalMediaPhase> {
        let entries = self.entries.lock().ok()?;
        entries.get(operation_id).map(|entry| entry.phase)
    }

    pub(crate) fn kind(&self, operation_id: &str) -> Option<LocalMediaOperationKind> {
        let entries = self.entries.lock().ok()?;
        entries.get(operation_id).map(|entry| entry.kind)
    }

    /// Read a terminal result.
    ///
    /// `Ok(None)` means "not finished, or never existed"; the caller keeps polling. An expired
    /// result is an error rather than `None` so the composer can say the result was dropped instead
    /// of polling forever.
    pub(crate) fn result(
        &self,
        operation_id: &str,
        now_ms: u64,
    ) -> Result<Option<LocalMediaOperationResult>, LocalMediaError> {
        let Ok(entries) = self.entries.lock() else {
            return Err(LocalMediaError::new(LocalMediaErrorCode::EngineUnavailable));
        };
        let Some(entry) = entries.get(operation_id) else {
            return Ok(None);
        };
        let Some(completed_at) = entry.completed_at_ms else {
            return Ok(None);
        };
        if now_ms.saturating_sub(completed_at) >= self.retention_ms {
            return Err(LocalMediaError::new(
                LocalMediaErrorCode::OperationResultExpired,
            ));
        }
        if let Some(error) = &entry.error {
            return Err(error.clone());
        }
        Ok(entry.result.clone())
    }

    /// Exercised by this module's tests; production reads the state it derives, not this.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn len(&self) -> usize {
        self.entries
            .lock()
            .map(|entries| entries.len())
            .unwrap_or(0)
    }

    /// Drop the oldest settled entries first.
    ///
    /// In-flight operations are never evicted: the composer is polling them, and losing one would
    /// leave a spinner with nothing behind it. If everything is in flight the map is allowed to
    /// exceed capacity briefly rather than break a live operation.
    fn evict(entries: &mut HashMap<String, StoredOperation>, capacity: usize) {
        while entries.len() > capacity {
            let oldest = entries
                .iter()
                .filter(|(_, entry)| entry.is_terminal())
                .min_by_key(|(_, entry)| entry.completed_at_ms.unwrap_or(entry.accepted_at_ms))
                .map(|(id, _)| id.clone());
            let Some(oldest) = oldest else {
                return;
            };
            entries.remove(&oldest);
        }
    }
}

#[cfg(test)]
#[path = "operation_store_tests.rs"]
mod tests;
