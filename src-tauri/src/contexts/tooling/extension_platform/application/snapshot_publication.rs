// The install flow that calls this lands with Task Group 4; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Publishing one snapshot, across two kinds of storage that cannot share a transaction.
//!
//! The order is fixed and the reason is asymmetry. Content that is published and never pointed at
//! is garbage, and startup reconciliation collects it. A pointer that names content which is not
//! there is an installation that cannot run and cannot be repaired without reinstalling. So
//! content goes first, the pointer goes last in one guarded write, and every failure between them
//! leaves the previous snapshot exactly where it was.

use super::ports::{SnapshotContentStore, SnapshotPointerRepository};
use crate::contexts::tooling::extension_platform::domain::{
    ContentPublication, SnapshotPointer, SnapshotPublicationError, SnapshotRecord, StagedRecovery,
};
use std::path::Path;
use std::sync::Arc;

/// A publication that succeeded, and what it did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublishedSnapshot {
    pub(crate) pointer: SnapshotPointer,
    pub(crate) content: ContentPublication,
}

pub(crate) struct SnapshotPublicationService {
    content: Arc<dyn SnapshotContentStore>,
    pointers: Arc<dyn SnapshotPointerRepository>,
}

impl SnapshotPublicationService {
    pub(crate) fn new(
        content: Arc<dyn SnapshotContentStore>,
        pointers: Arc<dyn SnapshotPointerRepository>,
    ) -> Self {
        Self { content, pointers }
    }

    /// Publishes `staged` as `record`, then points `installation` at it.
    ///
    /// `expected_revision` is the pointer revision the caller last observed. A publication that
    /// loses that race is refused rather than applied, because the operator approved this change
    /// against a state that has since moved.
    pub(crate) fn publish(
        &self,
        staged: &Path,
        record: &SnapshotRecord,
        expected_revision: i64,
    ) -> Result<PublishedSnapshot, SnapshotPublicationError> {
        // Checked before the content is moved, so a caller holding a stale revision does not leave
        // bytes behind for reconciliation to collect. It is checked again inside the pointer write,
        // where it is authoritative; this one is a courtesy that avoids pointless work.
        let current = self
            .pointers
            .active(&record.extension)
            .map_err(SnapshotPublicationError::Content)?;
        let current_revision = current.as_ref().map_or(0, |pointer| pointer.revision);
        if current_revision != expected_revision {
            return Err(SnapshotPublicationError::StaleRevision {
                expected: expected_revision,
                actual: current_revision,
            });
        }

        let content = self
            .content
            .publish(staged, &record.package_hash)
            .map_err(SnapshotPublicationError::Content)?;

        match self.pointers.point_at(record, expected_revision) {
            Ok(pointer) => Ok(PublishedSnapshot { pointer, content }),
            Err(SnapshotPublicationError::StaleRevision { expected, actual }) => {
                // Content stays. It is immutable, content-addressed, and unreferenced, which makes
                // it reconciliation's problem rather than a reason to delete bytes another install
                // may be about to point at.
                Err(SnapshotPublicationError::StaleRevision { expected, actual })
            }
            Err(error) => Err(match error {
                SnapshotPublicationError::Pointer { reason, .. } => {
                    SnapshotPublicationError::Pointer {
                        reason,
                        recovery: self.discard(staged),
                    }
                }
                other => other,
            }),
        }
    }

    /// Removes staged content after a failure, reporting whether it went.
    fn discard(&self, staged: &Path) -> StagedRecovery {
        if self.content.discard_staged(staged).is_ok() {
            StagedRecovery::Clean
        } else {
            StagedRecovery::Abandoned
        }
    }
}
