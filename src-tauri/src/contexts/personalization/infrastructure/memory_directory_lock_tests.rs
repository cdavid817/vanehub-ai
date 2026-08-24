use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{TimeZone, Utc};
use tempfile::TempDir;

use super::markdown_memory_repository::MarkdownMemoryRepository;
use super::memory_directory_lock::{
    is_lock_file, MemoryDirectoryLock, MemoryLockRejection, MEMORY_LOCK_FILE_NAME,
};
use crate::contexts::personalization::application::{
    CreateMemoryInput, MemoryIdGeneratorPort, MemoryMaintenanceRepository, MemoryRepository,
    PersonalizationApplicationError, UpdateMemoryPatch,
};
use crate::contexts::personalization::domain::{
    MaintenancePhase, MemoryAudience, MemoryId, MemoryProvenance, MemoryScope, MemoryScopeFilter,
    MemorySensitivity, MemorySource, MemoryStatus, MemoryType, ResetConfirmationToken,
    ResetMemoryRequest, RESET_CONFIRMATION_PHRASE,
};

#[derive(Default)]
struct SequentialIds {
    next: AtomicUsize,
}

impl MemoryIdGeneratorPort for SequentialIds {
    fn generate(&self) -> MemoryId {
        let index = self.next.fetch_add(1, Ordering::SeqCst);
        MemoryId::parse(&format!("01K2LCK{index:019}")).expect("memory id")
    }
}

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 24, 9, 0, 0).unwrap()
}

fn input(name: &str) -> CreateMemoryInput {
    CreateMemoryInput {
        name: name.to_string(),
        description: String::new(),
        memory_type: MemoryType::Project,
        content: "body".to_string(),
        scope: MemoryScope::Global,
        audience: MemoryAudience::AllAgents,
        status: MemoryStatus::Active,
        source: MemorySource::ExplicitUser,
        provenance: MemoryProvenance::default(),
        sensitivity: MemorySensitivity::Normal,
    }
}

fn reset_request() -> ResetMemoryRequest {
    ResetMemoryRequest {
        scope: MemoryScopeFilter::Any,
        statuses: Vec::new(),
        token: ResetConfirmationToken {
            value: "tok_01K2ABCDEF".to_string(),
            issued_at: now(),
            scope: MemoryScopeFilter::Any,
            statuses: Vec::new(),
        },
        typed_phrase: RESET_CONFIRMATION_PHRASE.to_string(),
    }
}

/// Two repositories over one directory, each with its own lock instance.
///
/// Separate instances mean separate in-process mutexes and separate OS file handles, so anything
/// that arbitrates between them is the operating system's lock and not shared process state. On
/// Unix `flock` is held per open file description and on Windows `LockFileEx` is held per handle,
/// so this exercises exactly the primitive a second process would contend on. A genuine subprocess
/// test is not reachable from here — every item in this context is `pub(crate)`, so an external
/// test binary cannot construct a repository — and is recorded as a gap rather than faked.
fn two_repositories(label: &str) -> (TempDir, MarkdownMemoryRepository, MarkdownMemoryRepository) {
    let directory = TempDir::with_prefix(format!("personalization-lock-{label}-"))
        .expect("temporary directory");
    let root = directory.path().join("memory");
    let first = MarkdownMemoryRepository::new(root.clone(), Arc::new(SequentialIds::default()))
        .expect("first repository");
    let second = MarkdownMemoryRepository::new(root, Arc::new(SequentialIds::default()))
        .expect("second repository");
    (directory, first, second)
}

#[test]
fn a_held_lock_makes_an_independent_holder_report_busy() {
    let directory = TempDir::with_prefix("personalization-lock-basic-").expect("directory");
    let root = directory.path().join("memory");
    fs::create_dir_all(&root).expect("root");

    let first = MemoryDirectoryLock::new(&root);
    let second = MemoryDirectoryLock::new(&root);

    let guard = first.try_acquire().expect("first acquisition");
    assert_eq!(
        second.try_acquire().err(),
        Some(MemoryLockRejection::Busy),
        "the OS lock, not a shared mutex, is what rejects the second holder"
    );

    drop(guard);
    assert!(
        second.try_acquire().is_ok(),
        "the lock is released when the guard drops"
    );
}

#[test]
fn the_guard_releases_on_an_early_return_or_a_panic() {
    let directory = TempDir::with_prefix("personalization-lock-panic-").expect("directory");
    let root = directory.path().join("memory");
    fs::create_dir_all(&root).expect("root");
    let lock = MemoryDirectoryLock::new(&root);
    let observer = MemoryDirectoryLock::new(&root);

    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = lock.try_acquire().expect("acquire");
        panic!("simulated failure while holding the directory");
    }));
    assert!(outcome.is_err());

    assert!(
        observer.try_acquire().is_ok(),
        "a panic while holding must not leave the directory locked forever"
    );
}

#[test]
fn a_reentrant_acquisition_reports_busy_instead_of_deadlocking() {
    // Non-reentrant by design. Reporting busy turns a caller bug into a diagnosable error rather
    // than a thread that never returns.
    let directory = TempDir::with_prefix("personalization-lock-reentrant-").expect("directory");
    let root = directory.path().join("memory");
    fs::create_dir_all(&root).expect("root");
    let lock = MemoryDirectoryLock::new(&root);

    let _guard = lock.try_acquire().expect("first");
    assert_eq!(
        lock.try_acquire().err(),
        Some(MemoryLockRejection::Busy),
        "the same instance must not grant the lock twice"
    );
}

#[test]
fn a_bounded_retry_gives_up_rather_than_waiting_forever() {
    let directory = TempDir::with_prefix("personalization-lock-retry-").expect("directory");
    let root = directory.path().join("memory");
    fs::create_dir_all(&root).expect("root");
    let holder = MemoryDirectoryLock::new(&root);
    let waiter = MemoryDirectoryLock::new(&root);

    let _guard = holder.try_acquire().expect("hold");
    let started = Instant::now();
    let result = waiter.acquire_with_budget(3, Duration::from_millis(5));
    let elapsed = started.elapsed();

    assert_eq!(result.err(), Some(MemoryLockRejection::Busy));
    assert!(
        elapsed < Duration::from_secs(2),
        "the retry budget must be bounded; waited {elapsed:?}"
    );
}

#[test]
fn every_mutating_operation_contends_on_the_same_lock() {
    // create, update, delete, reset, and reconcile all take the one directory lock, so a
    // maintenance run cannot interleave with an ordinary write.
    let (_directory, first, second) = two_repositories("mutual-exclusion");
    let created = first.create(input("Subject"), now()).expect("seed");

    let lock = second.lock();
    let held = lock
        .try_acquire()
        .expect("hold the directory from elsewhere");

    let create = first.create(input("Blocked"), now());
    let update = first.update(
        &created.id,
        1,
        UpdateMemoryPatch {
            content: Some("blocked".to_string()),
            ..UpdateMemoryPatch::default()
        },
        now(),
    );
    let delete = first.delete(&created.id, Some(1));
    let reset = first.reset(&reset_request(), now());
    let reconcile = first.reconcile(now());

    for outcome in [
        create.err(),
        update.err(),
        delete.err(),
        reset.err(),
        reconcile.err(),
    ] {
        assert_eq!(
            outcome,
            Some(PersonalizationApplicationError::MaintenanceBusy),
            "every mutating path must report a typed busy while the directory is held"
        );
    }

    drop(held);
    assert!(
        first.create(input("Allowed"), now()).is_ok(),
        "the same operations succeed once the holder releases"
    );
}

#[test]
fn a_failed_acquisition_leaves_the_authoritative_store_untouched() {
    let (_directory, first, second) = two_repositories("no-partial-write");
    let created = first.create(input("Original"), now()).expect("seed");
    let before = first.enumerate_owned_entries().expect("enumerate").len();

    let lock = second.lock();
    let held = lock.try_acquire().expect("hold");

    assert!(first.create(input("Rejected"), now()).is_err());
    assert!(first
        .update(
            &created.id,
            1,
            UpdateMemoryPatch {
                content: Some("rejected".to_string()),
                ..UpdateMemoryPatch::default()
            },
            now(),
        )
        .is_err());
    assert!(first.reset(&reset_request(), now()).is_err());

    drop(held);

    assert_eq!(
        first.enumerate_owned_entries().expect("enumerate").len(),
        before,
        "no file may be created or removed by a rejected operation"
    );
    let stored = first.get(&created.id).expect("get").expect("exists");
    assert_eq!(stored.content, "body");
    assert_eq!(
        stored.revision, 1,
        "a rejected update must not bump revision"
    );
}

#[test]
fn reads_are_not_blocked_by_a_held_lock() {
    // Only mutation is serialized. Blocking reads would make a maintenance run look like an
    // application outage.
    let (_directory, first, second) = two_repositories("reads");
    let created = first.create(input("Readable"), now()).expect("seed");

    let lock = second.lock();
    let _held = lock.try_acquire().expect("hold");

    assert!(first.get(&created.id).expect("get").is_some());
    assert_eq!(first.enumerate_owned_entries().expect("enumerate").len(), 1);
}

#[test]
fn the_lock_file_is_never_enumerated_counted_or_deleted() {
    let (_directory, repository, _second) = two_repositories("lock-file");
    repository.create(input("Subject"), now()).expect("seed");
    assert!(
        repository.lock().lock_path().is_file(),
        "the lock file exists after a mutation"
    );

    let entries = repository.enumerate_owned_entries().expect("enumerate");
    assert_eq!(entries.len(), 1, "only the memory is enumerated");
    assert!(!entries
        .iter()
        .any(|entry| is_lock_file(&entry.file_name) || entry.file_name == MEMORY_LOCK_FILE_NAME));

    let outcome = repository.reset(&reset_request(), now()).expect("reset");
    assert_eq!(outcome.deleted_files, 1);
    assert!(
        repository.lock().lock_path().is_file(),
        "a reset must not delete the lock file: the next two holders would each create their own"
    );
}

#[test]
fn a_stale_lock_file_alone_does_not_block_anyone() {
    // Presence is not possession. A lock file left behind by a crashed process must not lock the
    // directory forever, which is exactly what a file-exists check would do.
    let directory = TempDir::with_prefix("personalization-lock-stale-").expect("directory");
    let root = directory.path().join("memory");
    fs::create_dir_all(&root).expect("root");
    fs::write(root.join(MEMORY_LOCK_FILE_NAME), b"").expect("stale lock file");

    let lock = MemoryDirectoryLock::new(&root);
    assert!(
        lock.try_acquire().is_ok(),
        "an unlocked lock file grants the directory to the next caller"
    );
}

#[test]
fn an_injected_removal_failure_reports_repair_without_losing_the_other_records() {
    // Deterministic on every platform. A read-only file is deletable on Linux and not on Windows,
    // and an open file is deletable on POSIX and not on Windows, so the real conditions cannot
    // carry this assertion.
    let (_directory, repository, _second) = two_repositories("partial-failure");
    let kept = repository.create(input("Kept"), now()).expect("first");
    let removed = repository.create(input("Removed"), now()).expect("second");

    repository.inject_delete_failure(&kept.file_name());
    let outcome = repository.reset(&reset_request(), now()).expect("reset");

    assert!(outcome.requires_repair());
    assert_eq!(outcome.failures.len(), 1);
    assert_eq!(
        outcome.failures[0].phase,
        MaintenancePhase::AuthoritativeFile
    );
    assert_eq!(
        outcome.deleted_files, 1,
        "the record that could be removed still was"
    );

    let remaining = repository.enumerate_owned_entries().expect("enumerate");
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].memory_id.as_ref(), Some(&kept.id));
    assert!(repository.get(&removed.id).expect("get").is_none());
}

#[test]
fn an_injected_delete_failure_does_not_report_a_deleted_file() {
    let (_directory, repository, _second) = two_repositories("delete-failure");
    let record = repository.create(input("Subject"), now()).expect("seed");
    repository.inject_delete_failure(&record.file_name());

    assert!(
        repository.delete(&record.id, Some(1)).is_err(),
        "a removal that failed must not report success"
    );
    assert!(
        repository.get(&record.id).expect("get").is_some(),
        "the record survives a failed removal"
    );
}

#[test]
fn quarantine_failure_preserves_the_original_file() {
    // Portable injection: `create_dir_all` fails deterministically on every platform when the path
    // already exists as a file.
    let (_directory, repository, _second) = two_repositories("quarantine-failure");
    fs::write(
        repository.root().join("01K2BROKEN00000000000000000.md"),
        "not frontmatter",
    )
    .expect("malformed file");
    fs::write(repository.root().join("quarantine"), b"blocking file")
        .expect("block the quarantine directory");

    let outcome = repository.reconcile(now()).expect("reconcile");
    assert_eq!(outcome.quarantined_entries, 0);
    assert_eq!(outcome.failures.len(), 1);
    assert_eq!(outcome.failures[0].phase, MaintenancePhase::Quarantine);
    assert!(
        repository
            .root()
            .join("01K2BROKEN00000000000000000.md")
            .is_file(),
        "a failed quarantine must never silently delete the user's file"
    );
}
