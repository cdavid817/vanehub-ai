// The settings surface that calls this lands with task 12; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Reading and changing the Developer Mode switch.
//!
//! Two properties, both fail-closed:
//!
//! * **A read that cannot be answered is `Off`.** No row, a storage failure, a value nobody can
//!   parse — all of them mean unsigned content is refused. The alternative is a database problem
//!   quietly admitting content with no provenance, which is the one direction this must never
//!   fail in.
//! * **A change nobody can see did not happen.** The audit write is part of the change, not a
//!   courtesy afterwards, so a switch that flipped without a record is reported as a failure.

use super::ports::{DeveloperModeAuditEntry, DeveloperModeAuditSink, DeveloperModeRepository};
use crate::contexts::tooling::extension_platform::domain::{DeveloperMode, DeveloperModeError};
use std::sync::Arc;

/// The switch as it currently stands, with the revision a caller must quote to change it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeveloperModeView {
    pub(crate) mode: DeveloperMode,
    pub(crate) revision: i64,
    pub(crate) updated_at: Option<String>,
    pub(crate) updated_by: Option<String>,
    pub(crate) reason: Option<String>,
}

impl DeveloperModeView {
    /// What a build with nothing stored reports, and what a build that cannot read its own storage
    /// reports too.
    fn closed() -> Self {
        Self {
            mode: DeveloperMode::Off,
            revision: 0,
            updated_at: None,
            updated_by: None,
            reason: None,
        }
    }
}

pub(crate) trait DeveloperModeClock: Send + Sync {
    fn now_rfc3339(&self) -> String;
}

pub(crate) struct DeveloperModeService {
    repository: Arc<dyn DeveloperModeRepository>,
    audit: Arc<dyn DeveloperModeAuditSink>,
    clock: Arc<dyn DeveloperModeClock>,
}

impl DeveloperModeService {
    pub(crate) fn new(
        repository: Arc<dyn DeveloperModeRepository>,
        audit: Arc<dyn DeveloperModeAuditSink>,
        clock: Arc<dyn DeveloperModeClock>,
    ) -> Self {
        Self {
            repository,
            audit,
            clock,
        }
    }

    /// The current switch. Infallible on purpose: there is no useful thing a caller could do with
    /// a read error other than refuse unsigned content, which is what `Off` already means.
    pub(crate) fn current(&self) -> DeveloperModeView {
        self.repository.load().unwrap_or_else(|_| {
            // A storage failure is not reported here. It is reported by whatever the caller does
            // next, which under `Off` is to refuse — the safe direction.
            DeveloperModeView::closed()
        })
    }

    /// Changes the switch, guarded by the revision the caller last observed.
    pub(crate) fn set(
        &self,
        mode: DeveloperMode,
        expected_revision: i64,
        actor: &str,
        reason: Option<&str>,
    ) -> Result<DeveloperModeView, DeveloperModeError> {
        let current = self.repository.load()?;
        if current.revision != expected_revision {
            return Err(DeveloperModeError::StaleRevision {
                expected: expected_revision,
                actual: current.revision,
            });
        }

        let now = self.clock.now_rfc3339();
        let updated = self
            .repository
            .store(mode, current.revision + 1, &now, actor, reason)?;
        // Recorded even when the switch did not move. "Someone confirmed Developer Mode is on" is
        // exactly as much a fact about an installation as a change is.
        self.audit.record(&DeveloperModeAuditEntry {
            previous_enabled: current.mode.is_on(),
            new_enabled: mode.is_on(),
            revision: updated.revision,
            recorded_at: now,
            actor: actor.to_string(),
            reason: reason.map(str::to_string),
        })?;
        Ok(updated)
    }
}
