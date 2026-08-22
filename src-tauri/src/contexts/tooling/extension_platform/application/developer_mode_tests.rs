//! Reading and changing the switch, including the ways it must fail.

use super::{
    DeveloperModeAuditEntry, DeveloperModeAuditSink, DeveloperModeClock, DeveloperModeRepository,
    DeveloperModeService, DeveloperModeView,
};
use crate::contexts::tooling::extension_platform::domain::{DeveloperMode, DeveloperModeError};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct MemorySwitch {
    stored: Mutex<Option<DeveloperModeView>>,
    read_failure: Option<String>,
    write_failure: Option<String>,
}

impl MemorySwitch {
    fn unreadable() -> Self {
        Self {
            read_failure: Some("database is locked".to_string()),
            ..Self::default()
        }
    }

    fn unwritable() -> Self {
        Self {
            write_failure: Some("disk is full".to_string()),
            ..Self::default()
        }
    }
}

impl DeveloperModeRepository for MemorySwitch {
    fn load(&self) -> Result<DeveloperModeView, DeveloperModeError> {
        if let Some(failure) = &self.read_failure {
            return Err(DeveloperModeError::Storage(failure.clone()));
        }
        Ok(self
            .stored
            .lock()
            .ok()
            .and_then(|guard| guard.clone())
            .unwrap_or(DeveloperModeView {
                mode: DeveloperMode::Off,
                revision: 0,
                updated_at: None,
                updated_by: None,
                reason: None,
            }))
    }

    fn store(
        &self,
        mode: DeveloperMode,
        revision: i64,
        updated_at: &str,
        updated_by: &str,
        reason: Option<&str>,
    ) -> Result<DeveloperModeView, DeveloperModeError> {
        if let Some(failure) = &self.write_failure {
            return Err(DeveloperModeError::Storage(failure.clone()));
        }
        let view = DeveloperModeView {
            mode,
            revision,
            updated_at: Some(updated_at.to_string()),
            updated_by: Some(updated_by.to_string()),
            reason: reason.map(str::to_string),
        };
        if let Ok(mut guard) = self.stored.lock() {
            *guard = Some(view.clone());
        }
        Ok(view)
    }
}

#[derive(Default)]
struct RecordingAudit {
    entries: Mutex<Vec<DeveloperModeAuditEntry>>,
    failure: Option<String>,
}

impl DeveloperModeAuditSink for RecordingAudit {
    fn record(&self, entry: &DeveloperModeAuditEntry) -> Result<(), DeveloperModeError> {
        if let Some(failure) = &self.failure {
            return Err(DeveloperModeError::Storage(failure.clone()));
        }
        if let Ok(mut guard) = self.entries.lock() {
            guard.push(entry.clone());
        }
        Ok(())
    }
}

struct FixedClock;

impl DeveloperModeClock for FixedClock {
    fn now_rfc3339(&self) -> String {
        "2026-08-22T00:00:00Z".to_string()
    }
}

fn service(switch: Arc<MemorySwitch>, audit: Arc<RecordingAudit>) -> DeveloperModeService {
    DeveloperModeService::new(switch, audit, Arc::new(FixedClock))
}

#[test]
fn a_fresh_installation_has_developer_mode_off() {
    let service = service(
        Arc::new(MemorySwitch::default()),
        Arc::new(RecordingAudit::default()),
    );

    let current = service.current();
    assert_eq!(current.mode, DeveloperMode::Off);
    assert_eq!(current.revision, 0);
    assert_eq!(current.updated_at, None);
}

#[test]
fn turning_it_on_records_who_did_it_and_why() {
    let audit = Arc::new(RecordingAudit::default());
    let service = service(Arc::new(MemorySwitch::default()), audit.clone());

    let updated = service
        .set(
            DeveloperMode::On,
            0,
            "operator",
            Some("local extension work"),
        )
        .expect("set");

    assert_eq!(updated.mode, DeveloperMode::On);
    assert_eq!(updated.revision, 1);
    assert_eq!(service.current().mode, DeveloperMode::On);

    let entries = audit.entries.lock().expect("entries").clone();
    assert_eq!(entries.len(), 1);
    assert!(!entries[0].previous_enabled);
    assert!(entries[0].new_enabled);
    assert_eq!(entries[0].revision, 1);
    assert_eq!(entries[0].actor, "operator");
    assert_eq!(entries[0].reason.as_deref(), Some("local extension work"));
    assert_eq!(entries[0].recorded_at, "2026-08-22T00:00:00Z");
}

#[test]
fn a_writer_holding_a_stale_revision_is_refused_without_changing_anything() {
    let switch = Arc::new(MemorySwitch::default());
    let audit = Arc::new(RecordingAudit::default());
    let service = service(switch, audit.clone());
    service
        .set(DeveloperMode::On, 0, "operator", None)
        .expect("first change");

    assert_eq!(
        service.set(DeveloperMode::Off, 0, "someone else", None),
        Err(DeveloperModeError::StaleRevision {
            expected: 0,
            actual: 1
        })
    );
    assert_eq!(service.current().mode, DeveloperMode::On);
    assert_eq!(audit.entries.lock().expect("entries").len(), 1);
}

#[test]
fn confirming_the_current_value_is_still_audited() {
    // "Someone confirmed Developer Mode is on" is as much a fact about an installation as a change
    // is, and an audit that only records movement cannot answer who last looked.
    let audit = Arc::new(RecordingAudit::default());
    let service = service(Arc::new(MemorySwitch::default()), audit.clone());

    service
        .set(DeveloperMode::Off, 0, "operator", None)
        .expect("no-op change");

    let entries = audit.entries.lock().expect("entries").clone();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].previous_enabled, entries[0].new_enabled);
}

#[test]
fn a_switch_that_cannot_be_read_reports_off() {
    // The one direction this must never fail in: a storage problem refuses unsigned content rather
    // than admitting it.
    let service = service(
        Arc::new(MemorySwitch::unreadable()),
        Arc::new(RecordingAudit::default()),
    );

    assert_eq!(service.current().mode, DeveloperMode::Off);
}

#[test]
fn a_change_that_cannot_be_written_is_reported_rather_than_assumed() {
    let audit = Arc::new(RecordingAudit::default());
    let service = service(Arc::new(MemorySwitch::unwritable()), audit.clone());

    assert_eq!(
        service.set(DeveloperMode::On, 0, "operator", None),
        Err(DeveloperModeError::Storage("disk is full".to_string()))
    );
    assert!(audit.entries.lock().expect("entries").is_empty());
}

#[test]
fn a_change_nobody_can_see_is_reported_as_a_failure() {
    let audit = Arc::new(RecordingAudit {
        entries: Mutex::new(Vec::new()),
        failure: Some("audit table is missing".to_string()),
    });
    let service = service(Arc::new(MemorySwitch::default()), audit);

    assert_eq!(
        service.set(DeveloperMode::On, 0, "operator", None),
        Err(DeveloperModeError::Storage(
            "audit table is missing".to_string()
        ))
    );
}

#[test]
fn every_error_has_a_stable_code() {
    assert_eq!(
        DeveloperModeError::StaleRevision {
            expected: 0,
            actual: 1
        }
        .code(),
        "developer_mode_stale_revision"
    );
    assert_eq!(
        DeveloperModeError::Storage(String::new()).code(),
        "developer_mode_storage_failure"
    );
}
