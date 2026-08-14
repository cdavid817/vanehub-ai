#![allow(dead_code)]

use super::{BrowserAction, BrowserOwnership};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const MAX_HANDOFF_DURATION: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserHandoffPhase {
    HandedOff,
    InspectionRequired,
    Active,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserHandoff {
    pub(crate) handoff_id: String,
    pub(crate) ownership: BrowserOwnership,
    pub(crate) page_id: String,
    pub(crate) phase: BrowserHandoffPhase,
    pub(crate) revision: u64,
    pub(crate) expires_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserHandoffError {
    InvalidRequest,
    AlreadyHandedOff,
    AutomationPaused,
    FreshInspectionRequired,
    StaleHandoff,
    Expired,
    InternalFailure,
}

pub(crate) struct BrowserHandoffManager {
    next_id: AtomicU64,
    states: Mutex<BTreeMap<BrowserOwnership, BrowserHandoff>>,
}

impl Default for BrowserHandoffManager {
    fn default() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            states: Mutex::new(BTreeMap::new()),
        }
    }
}

impl BrowserHandoffManager {
    pub(crate) fn begin(
        &self,
        ownership: BrowserOwnership,
        page_id: String,
        now: Instant,
        duration: Duration,
    ) -> Result<BrowserHandoff, BrowserHandoffError> {
        if page_id.is_empty()
            || page_id.len() > 128
            || duration.is_zero()
            || duration > MAX_HANDOFF_DURATION
        {
            return Err(BrowserHandoffError::InvalidRequest);
        }
        let mut states = self.states()?;
        if states
            .get(&ownership)
            .is_some_and(|state| state.phase == BrowserHandoffPhase::HandedOff)
        {
            return Err(BrowserHandoffError::AlreadyHandedOff);
        }
        let handoff = BrowserHandoff {
            handoff_id: format!(
                "browser-handoff-{}",
                self.next_id.fetch_add(1, Ordering::Relaxed)
            ),
            ownership: ownership.clone(),
            page_id,
            phase: BrowserHandoffPhase::HandedOff,
            revision: states.get(&ownership).map_or(0, |state| state.revision),
            expires_at: now + duration,
        };
        states.insert(ownership, handoff.clone());
        Ok(handoff)
    }

    pub(crate) fn resume(
        &self,
        ownership: &BrowserOwnership,
        handoff_id: &str,
        now: Instant,
        explicit_user_action: bool,
    ) -> Result<u64, BrowserHandoffError> {
        if !explicit_user_action {
            return Err(BrowserHandoffError::StaleHandoff);
        }
        let mut states = self.states()?;
        let state = states
            .get_mut(ownership)
            .ok_or(BrowserHandoffError::StaleHandoff)?;
        ensure_current(state, handoff_id, now)?;
        state.phase = BrowserHandoffPhase::InspectionRequired;
        state.revision = state.revision.saturating_add(1);
        Ok(state.revision)
    }

    pub(crate) fn ensure_automation_allowed(
        &self,
        ownership: &BrowserOwnership,
        action: BrowserAction,
        now: Instant,
    ) -> Result<(), BrowserHandoffError> {
        let states = self.states()?;
        let Some(state) = states.get(ownership) else {
            return Ok(());
        };
        if now >= state.expires_at {
            return Err(BrowserHandoffError::Expired);
        }
        match state.phase {
            BrowserHandoffPhase::HandedOff => Err(BrowserHandoffError::AutomationPaused),
            BrowserHandoffPhase::InspectionRequired if action != BrowserAction::Inspect => {
                Err(BrowserHandoffError::FreshInspectionRequired)
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn record_completed(
        &self,
        ownership: &BrowserOwnership,
        action: BrowserAction,
    ) -> Result<(), BrowserHandoffError> {
        let mut states = self.states()?;
        if let Some(state) = states.get_mut(ownership) {
            if state.phase == BrowserHandoffPhase::InspectionRequired
                && action == BrowserAction::Inspect
            {
                state.phase = BrowserHandoffPhase::Active;
            }
        }
        Ok(())
    }

    pub(crate) fn close(&self, ownership: &BrowserOwnership) -> Result<(), BrowserHandoffError> {
        self.states()?.remove(ownership);
        Ok(())
    }

    fn states(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, BTreeMap<BrowserOwnership, BrowserHandoff>>,
        BrowserHandoffError,
    > {
        self.states
            .lock()
            .map_err(|_| BrowserHandoffError::InternalFailure)
    }
}

fn ensure_current(
    state: &BrowserHandoff,
    handoff_id: &str,
    now: Instant,
) -> Result<(), BrowserHandoffError> {
    if now >= state.expires_at {
        return Err(BrowserHandoffError::Expired);
    }
    if state.phase != BrowserHandoffPhase::HandedOff || state.handoff_id != handoff_id {
        return Err(BrowserHandoffError::StaleHandoff);
    }
    Ok(())
}

#[cfg(test)]
#[path = "handoff_tests.rs"]
mod tests;
