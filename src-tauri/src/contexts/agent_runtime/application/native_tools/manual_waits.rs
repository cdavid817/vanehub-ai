use super::{NativeToolDispatchError, NativeToolErrorCode};
use crate::contexts::agent_runtime::application::ToolApprovalDecision;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Instant;

#[derive(Default)]
pub(super) struct ManualApprovalWaits {
    entries: Mutex<HashMap<String, ManualApprovalWait>>,
    changed: Condvar,
}

struct ManualApprovalWait {
    session_id: String,
    decision: Option<ToolApprovalDecision>,
    cancelled: Arc<AtomicBool>,
}

impl ManualApprovalWaits {
    pub(super) fn register(
        &self,
        call_id: &str,
        session_id: &str,
        cancelled: Arc<AtomicBool>,
    ) -> Result<(), NativeToolDispatchError> {
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| error("approval_wait_unavailable"))?;
        if entries.contains_key(call_id) {
            return Err(NativeToolDispatchError {
                code: NativeToolErrorCode::Conflict,
                safe_message: "duplicate_call_id".to_owned(),
            });
        }
        entries.insert(
            call_id.to_owned(),
            ManualApprovalWait {
                session_id: session_id.to_owned(),
                decision: None,
                cancelled,
            },
        );
        Ok(())
    }

    pub(super) fn wait(&self, call_id: &str, deadline: Instant) -> Option<ToolApprovalDecision> {
        let mut entries = self.entries.lock().ok()?;
        loop {
            let entry = entries.get(call_id)?;
            if let Some(decision) = entry.decision.as_ref() {
                return Some(decision.clone());
            }
            if entry.cancelled.load(Ordering::Acquire) || Instant::now() >= deadline {
                return None;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            entries = self.changed.wait_timeout(entries, remaining).ok()?.0;
        }
    }

    pub(super) fn resolve(
        &self,
        session_id: &str,
        call_id: &str,
        decision: ToolApprovalDecision,
    ) -> bool {
        let Ok(mut entries) = self.entries.lock() else {
            return false;
        };
        let Some(entry) = entries.get_mut(call_id) else {
            return false;
        };
        if entry.session_id != session_id || entry.decision.is_some() {
            return false;
        }
        entry.decision = Some(decision);
        self.changed.notify_all();
        true
    }

    pub(super) fn cancel(&self, call_id: &str) -> bool {
        let Ok(entries) = self.entries.lock() else {
            return false;
        };
        let Some(entry) = entries.get(call_id) else {
            return false;
        };
        entry.cancelled.store(true, Ordering::Release);
        self.changed.notify_all();
        true
    }

    pub(super) fn remove(&self, call_id: &str) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.remove(call_id);
        }
    }
}

fn error(message: &str) -> NativeToolDispatchError {
    NativeToolDispatchError {
        code: NativeToolErrorCode::InternalFailure,
        safe_message: message.to_owned(),
    }
}
