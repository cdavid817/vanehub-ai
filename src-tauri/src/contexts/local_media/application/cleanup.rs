//! Drop-based cleanup for operation-owned media.
//!
//! Explicit cleanup at each return site is the version of this that eventually misses one: the
//! interesting paths are the early returns, and those are exactly the ones a reviewer skims. A
//! guard makes success, failure, cancellation, and an unexpected unwind all take the same route.

use super::ports::MediaTempStore;
use std::sync::Arc;

pub(super) struct OperationMediaGuard {
    temp: Arc<dyn MediaTempStore>,
    operation_id: String,
    disarmed: bool,
}

impl OperationMediaGuard {
    pub(super) fn new(temp: Arc<dyn MediaTempStore>, operation_id: impl Into<String>) -> Self {
        Self {
            temp,
            operation_id: operation_id.into(),
            disarmed: false,
        }
    }

    /// Hand ownership to a later stage. Used when playback outlives the synthesis call and will
    /// delete the file itself.
    pub(super) fn disarm(&mut self) {
        self.disarmed = true;
    }
}

impl Drop for OperationMediaGuard {
    fn drop(&mut self) {
        if self.disarmed {
            return;
        }
        self.temp.cleanup_operation(&self.operation_id);
    }
}

#[cfg(test)]
#[path = "cleanup_tests.rs"]
mod tests;
