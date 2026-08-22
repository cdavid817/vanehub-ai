//! Publication ordering, and what every failure between the two writes leaves behind.

use super::{
    PublishedSnapshot, SnapshotContentStore, SnapshotPointerRepository, SnapshotPublicationService,
};
use crate::contexts::tooling::extension_platform::domain::{
    ContentPublication, ExtensionId, InstallationId, ManifestDigest, PackageHash, SnapshotId,
    SnapshotPointer, SnapshotPublicationError, SnapshotRecord, StagedRecovery,
};
use semver::Version;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

const PACKAGE_DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const MANIFEST_DIGEST: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn record(snapshot: &str) -> SnapshotRecord {
    SnapshotRecord {
        snapshot: SnapshotId::parse(snapshot).expect("snapshot id"),
        extension: ExtensionId::parse("acme.git-guardian").expect("extension"),
        version: Version::parse("1.2.0").expect("version"),
        package_hash: PackageHash::parse(PACKAGE_DIGEST).expect("hash"),
        manifest_digest: ManifestDigest::parse(MANIFEST_DIGEST).expect("digest"),
        created_at: "2026-08-22T00:00:00Z".to_string(),
    }
}

#[derive(Default)]
struct MemoryContent {
    published: Mutex<Vec<PathBuf>>,
    discarded: Mutex<Vec<PathBuf>>,
    already_present: bool,
    publish_failure: Option<String>,
    discard_failure: bool,
}

impl SnapshotContentStore for MemoryContent {
    fn publish(&self, staged: &Path, _hash: &PackageHash) -> Result<ContentPublication, String> {
        if let Some(failure) = &self.publish_failure {
            return Err(failure.clone());
        }
        if let Ok(mut published) = self.published.lock() {
            published.push(staged.to_path_buf());
        }
        Ok(if self.already_present {
            ContentPublication::AlreadyPresent
        } else {
            ContentPublication::Published
        })
    }

    fn discard_staged(&self, staged: &Path) -> Result<(), String> {
        if self.discard_failure {
            return Err("staged content is locked".to_string());
        }
        if let Ok(mut discarded) = self.discarded.lock() {
            discarded.push(staged.to_path_buf());
        }
        Ok(())
    }
}

struct MemoryPointers {
    current: Mutex<Option<SnapshotPointer>>,
    write_failure: Option<String>,
    read_failure: Option<String>,
}

impl MemoryPointers {
    fn empty() -> Self {
        Self {
            current: Mutex::new(None),
            write_failure: None,
            read_failure: None,
        }
    }

    fn holding(active: &str, revision: i64) -> Self {
        Self {
            current: Mutex::new(Some(SnapshotPointer {
                installation: InstallationId::parse("install-1").expect("installation"),
                extension: ExtensionId::parse("acme.git-guardian").expect("extension"),
                active: SnapshotId::parse(active).expect("snapshot"),
                previous: None,
                revision,
                updated_at: "2026-08-01T00:00:00Z".to_string(),
            })),
            write_failure: None,
            read_failure: None,
        }
    }
}

impl SnapshotPointerRepository for MemoryPointers {
    fn active(&self, _extension: &ExtensionId) -> Result<Option<SnapshotPointer>, String> {
        if let Some(failure) = &self.read_failure {
            return Err(failure.clone());
        }
        Ok(self.current.lock().ok().and_then(|guard| guard.clone()))
    }

    fn point_at(
        &self,
        record: &SnapshotRecord,
        expected_revision: i64,
    ) -> Result<SnapshotPointer, SnapshotPublicationError> {
        if let Some(failure) = &self.write_failure {
            return Err(SnapshotPublicationError::Pointer {
                reason: failure.clone(),
                recovery: StagedRecovery::Clean,
            });
        }
        let mut guard = self
            .current
            .lock()
            .map_err(|_| SnapshotPublicationError::Pointer {
                reason: "poisoned".to_string(),
                recovery: StagedRecovery::Clean,
            })?;
        let current_revision = guard.as_ref().map_or(0, |pointer| pointer.revision);
        if current_revision != expected_revision {
            return Err(SnapshotPublicationError::StaleRevision {
                expected: expected_revision,
                actual: current_revision,
            });
        }
        let pointer = SnapshotPointer {
            installation: InstallationId::parse("install-1").expect("installation"),
            extension: record.extension.clone(),
            active: record.snapshot.clone(),
            previous: guard.as_ref().map(|current| current.active.clone()),
            revision: current_revision + 1,
            updated_at: record.created_at.clone(),
        };
        *guard = Some(pointer.clone());
        Ok(pointer)
    }
}

fn service(
    content: Arc<MemoryContent>,
    pointers: Arc<MemoryPointers>,
) -> SnapshotPublicationService {
    SnapshotPublicationService::new(content, pointers)
}

#[test]
fn a_first_publication_points_at_new_content_and_has_nothing_to_roll_back_to() {
    let content = Arc::new(MemoryContent::default());
    let pointers = Arc::new(MemoryPointers::empty());
    let service = service(content.clone(), pointers);

    let published: PublishedSnapshot = service
        .publish(Path::new("/staged"), &record("snapshot-1"), 0)
        .expect("publish");

    assert_eq!(published.content, ContentPublication::Published);
    assert_eq!(published.pointer.active.as_str(), "snapshot-1");
    assert_eq!(published.pointer.previous, None);
    assert_eq!(published.pointer.revision, 1);
    assert_eq!(content.published.lock().expect("published").len(), 1);
}

#[test]
fn an_update_keeps_the_snapshot_it_replaced_as_the_rollback_target() {
    let content = Arc::new(MemoryContent::default());
    let pointers = Arc::new(MemoryPointers::holding("snapshot-1", 3));
    let service = service(content, pointers);

    let published = service
        .publish(Path::new("/staged"), &record("snapshot-2"), 3)
        .expect("publish");

    assert_eq!(published.pointer.active.as_str(), "snapshot-2");
    assert_eq!(
        published.pointer.previous.as_ref().map(SnapshotId::as_str),
        Some("snapshot-1")
    );
    assert_eq!(published.pointer.revision, 4);
}

#[test]
fn content_that_is_already_there_is_a_success_rather_than_a_conflict() {
    // Content is addressed by its own digest, so a destination that exists holds exactly the bytes
    // being published -- including when a concurrent install of the same package put them there.
    let content = Arc::new(MemoryContent {
        already_present: true,
        ..MemoryContent::default()
    });
    let pointers = Arc::new(MemoryPointers::empty());
    let service = service(content, pointers);

    let published = service
        .publish(Path::new("/staged"), &record("snapshot-1"), 0)
        .expect("publish");

    assert_eq!(published.content, ContentPublication::AlreadyPresent);
    assert_eq!(published.pointer.active.as_str(), "snapshot-1");
}

#[test]
fn a_caller_holding_a_stale_revision_is_refused_before_any_content_is_moved() {
    let content = Arc::new(MemoryContent::default());
    let pointers = Arc::new(MemoryPointers::holding("snapshot-1", 3));
    let service = service(content.clone(), pointers.clone());

    assert_eq!(
        service.publish(Path::new("/staged"), &record("snapshot-2"), 1),
        Err(SnapshotPublicationError::StaleRevision {
            expected: 1,
            actual: 3
        })
    );
    assert!(
        content.published.lock().expect("published").is_empty(),
        "a stale caller must not leave bytes behind for reconciliation to collect"
    );
    assert_eq!(
        pointers
            .active(&ExtensionId::parse("acme.git-guardian").expect("extension"))
            .expect("active")
            .map(|pointer| pointer.revision),
        Some(3),
        "and the pointer is exactly where it was"
    );
}

#[test]
fn content_that_cannot_be_written_leaves_the_pointer_untouched() {
    let content = Arc::new(MemoryContent {
        publish_failure: Some("disk is full".to_string()),
        ..MemoryContent::default()
    });
    let pointers = Arc::new(MemoryPointers::holding("snapshot-1", 3));
    let service = service(content, pointers.clone());

    assert_eq!(
        service.publish(Path::new("/staged"), &record("snapshot-2"), 3),
        Err(SnapshotPublicationError::Content(
            "disk is full".to_string()
        ))
    );
    assert_eq!(
        pointers
            .active(&ExtensionId::parse("acme.git-guardian").expect("extension"))
            .expect("active")
            .map(|pointer| pointer.active.as_str().to_string()),
        Some("snapshot-1".to_string())
    );
}

#[test]
fn a_pointer_write_that_fails_discards_the_staged_content_and_says_so() {
    let content = Arc::new(MemoryContent::default());
    let pointers = Arc::new(MemoryPointers {
        write_failure: Some("database is locked".to_string()),
        ..MemoryPointers::holding("snapshot-1", 3)
    });
    let service = service(content.clone(), pointers.clone());

    assert_eq!(
        service.publish(Path::new("/staged"), &record("snapshot-2"), 3),
        Err(SnapshotPublicationError::Pointer {
            reason: "database is locked".to_string(),
            recovery: StagedRecovery::Clean,
        })
    );
    assert_eq!(
        content.discarded.lock().expect("discarded").len(),
        1,
        "the staged copy goes; the published content is unreferenced and reconciliation's problem"
    );
    assert_eq!(
        pointers
            .active(&ExtensionId::parse("acme.git-guardian").expect("extension"))
            .expect("active")
            .map(|pointer| pointer.active.as_str().to_string()),
        Some("snapshot-1".to_string()),
        "the previous snapshot is retained on every failure path"
    );
}

#[test]
fn staged_content_that_cannot_be_removed_is_reported_rather_than_hidden() {
    let content = Arc::new(MemoryContent {
        discard_failure: true,
        ..MemoryContent::default()
    });
    let pointers = Arc::new(MemoryPointers {
        write_failure: Some("database is locked".to_string()),
        ..MemoryPointers::empty()
    });
    let service = service(content, pointers);

    assert_eq!(
        service.publish(Path::new("/staged"), &record("snapshot-1"), 0),
        Err(SnapshotPublicationError::Pointer {
            reason: "database is locked".to_string(),
            recovery: StagedRecovery::Abandoned,
        })
    );
}

#[test]
fn losing_the_race_at_the_pointer_write_leaves_the_content_for_reconciliation() {
    // The content is immutable, content-addressed, and unreferenced. Deleting it here would be
    // deleting bytes the install that won the race may be about to point at.
    let content = Arc::new(MemoryContent::default());
    let pointers = Arc::new(MemoryPointers::empty());

    // The pre-check reads revision 0, and the guarded write finds 1 because someone else got in.
    pointers
        .point_at(&record("snapshot-other"), 0)
        .expect("the other install wins");
    let stale = SnapshotPublicationService::new(content.clone(), pointers.clone());
    let outcome = stale.publish(Path::new("/staged"), &record("snapshot-1"), 0);

    assert_eq!(
        outcome,
        Err(SnapshotPublicationError::StaleRevision {
            expected: 0,
            actual: 1
        })
    );
    assert!(content.discarded.lock().expect("discarded").is_empty());
}

#[test]
fn a_pointer_that_cannot_be_read_is_reported_before_anything_is_moved() {
    let content = Arc::new(MemoryContent::default());
    let pointers = Arc::new(MemoryPointers {
        read_failure: Some("database is locked".to_string()),
        ..MemoryPointers::empty()
    });
    let service = service(content.clone(), pointers);

    assert_eq!(
        service.publish(Path::new("/staged"), &record("snapshot-1"), 0),
        Err(SnapshotPublicationError::Content(
            "database is locked".to_string()
        ))
    );
    assert!(content.published.lock().expect("published").is_empty());
}
