//! Enumeration, backup, removal, and quarantine of the pre-v2 directory.
//!
//! Every case here is about what the directory *contains*, so they run against a real directory
//! rather than a fake: the rules under test are the ones that decide whether a user's file is seen
//! at all, and a fake would only re-state the rule it was built from.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use tempfile::TempDir;

use super::legacy_memory_source::{FileLegacyMemorySource, LegacyOperation};
use super::markdown_memory_repository::{DERIVED_INDEX_FILE_NAME, QUARANTINE_DIRECTORY_NAME};
use super::memory_directory_lock::{MemoryDirectoryLock, MEMORY_LOCK_FILE_NAME};
use crate::contexts::personalization::application::{
    LegacyMemorySourcePort, PersonalizationApplicationError,
};
use crate::contexts::personalization::domain::{LegacySourceFingerprint, LegacySourceLocator};

const LEGACY: &str = "---\nname: user-role\ndescription: d\n---\n\nBody.\n";

struct Fixture {
    _directory: TempDir,
    root: PathBuf,
    lock: Arc<MemoryDirectoryLock>,
    source: FileLegacyMemorySource,
}

fn fixture(label: &str) -> Fixture {
    let directory =
        TempDir::with_prefix(format!("personalization-legacy-{label}-")).expect("temporary dir");
    let root = directory.path().join("memory");
    let lock = Arc::new(MemoryDirectoryLock::new(&root));
    let source = FileLegacyMemorySource::new(root.clone(), lock.clone()).expect("source");
    Fixture {
        _directory: directory,
        root,
        lock,
        source,
    }
}

impl Fixture {
    fn write(&self, file_name: &str, contents: &str) {
        fs::write(self.root.join(file_name), contents).expect("fixture file");
    }

    fn names(&self) -> Vec<String> {
        self.source
            .enumerate_sources()
            .expect("enumerate")
            .into_iter()
            .filter_map(|source| match source.locator {
                LegacySourceLocator::MarkdownFile {
                    normalized_relative_path,
                } => Some(normalized_relative_path.as_str().to_string()),
                LegacySourceLocator::SqliteRow { .. } => None,
            })
            .collect()
    }
}

fn locator(file_name: &str) -> LegacySourceLocator {
    LegacySourceLocator::markdown(file_name).expect("locator")
}

#[test]
fn an_empty_directory_holds_no_sources() {
    let fixture = fixture("empty");

    assert!(fixture
        .source
        .enumerate_sources()
        .expect("enumerate")
        .is_empty());
}

#[test]
fn a_missing_directory_is_an_empty_store_rather_than_a_failure() {
    let directory = TempDir::with_prefix("personalization-legacy-absent-").expect("temporary dir");
    let root = directory.path().join("memory");
    let source =
        FileLegacyMemorySource::new(root.clone(), Arc::new(MemoryDirectoryLock::new(&root)))
            .expect("source");
    fs::remove_dir_all(&root).expect("remove the directory out from under it");

    assert!(source.enumerate_sources().expect("enumerate").is_empty());
}

#[test]
fn enumeration_excludes_everything_that_is_not_a_legacy_memory() {
    let fixture = fixture("exclusions");
    fixture.write("kept.md", LEGACY);
    // Derived, never a memory.
    fixture.write(DERIVED_INDEX_FILE_NAME, "# Memory index\n");
    // The lock file: deleting or migrating it would let two holders each believe they own the
    // directory.
    fixture.write(MEMORY_LOCK_FILE_NAME, "");
    // Half of someone else's atomic write.
    fixture.write("half-written.md.tmp", LEGACY);
    fixture.write("other.lock", "");
    // Not a memory file at all.
    fixture.write("notes.txt", LEGACY);
    // Already governed: it declares the current schema version.
    fixture.write(
        "already-v2.md",
        "---\nschema_version: 2\nname: \"x\"\n---\n\nBody.\n",
    );
    // Copies of sources live in directories, which are skipped outright.
    fs::create_dir_all(fixture.root.join("legacy-backup")).expect("backup dir");
    fs::write(fixture.root.join("legacy-backup").join("copy.md"), LEGACY).expect("backup copy");
    fs::create_dir_all(fixture.root.join(QUARANTINE_DIRECTORY_NAME)).expect("quarantine dir");
    fs::write(
        fixture.root.join(QUARANTINE_DIRECTORY_NAME).join("bad.md"),
        LEGACY,
    )
    .expect("quarantined copy");

    assert_eq!(fixture.names(), vec!["kept.md".to_string()]);
}

#[test]
fn a_v2_file_is_recognized_by_its_declared_version_not_by_its_name() {
    // The trap this closes: a v1 memory's file name was its display name, and a name like
    // `use-pnpm` is also a perfectly valid v2 memory id. Neither shape can decide the format.
    let fixture = fixture("version-not-name");
    fixture.write(
        "use-pnpm.md",
        "---\nschema_version: 2\nname: \"use-pnpm\"\n---\n\nBody.\n",
    );
    fixture.write("01K2MEM0000000000000000001.md", LEGACY);

    assert_eq!(
        fixture.names(),
        vec!["01K2MEM0000000000000000001.md".to_string()]
    );
}

#[test]
fn a_source_carries_the_raw_fingerprint_of_its_bytes() {
    let fixture = fixture("fingerprint");
    // CRLF on purpose: the raw fingerprint covers the bytes on disk, not a normalized body, which
    // is what lets a pre-delete recheck notice an edit that only changed line endings.
    let raw = LEGACY.replace('\n', "\r\n");
    fixture.write("crlf.md", &raw);

    let sources = fixture.source.enumerate_sources().expect("enumerate");

    assert_eq!(sources.len(), 1);
    assert_eq!(
        sources[0].fingerprint.as_ref().expect("fingerprint"),
        &LegacySourceFingerprint::of(raw.as_bytes())
    );
    assert_eq!(
        sources[0]
            .fingerprint
            .as_ref()
            .expect("fingerprint")
            .byte_length,
        raw.len() as u64
    );
}

#[test]
fn a_malformed_source_is_enumerated_with_no_fields_rather_than_skipped() {
    // v1's own scan skipped what it could not parse, so a malformed file was invisible: a reset
    // could report success while leaving it behind. It must be visible here to be quarantined.
    let fixture = fixture("malformed");
    fixture.write("unterminated.md", "---\nname: x\ndescription: d\n");
    fixture.write("no-name.md", "---\ndescription: d\n---\n\nBody.\n");
    fixture.write("valid.md", LEGACY);

    let sources = fixture.source.enumerate_sources().expect("enumerate");

    assert_eq!(sources.len(), 3);
    let unreadable = sources
        .iter()
        .filter(|source| source.fields.is_none())
        .count();
    assert_eq!(unreadable, 2);
    // Their bytes were read, so they can be fingerprinted and reported even though they cannot be
    // migrated.
    assert!(sources.iter().all(|source| source.fingerprint.is_some()));
}

#[test]
fn a_file_that_is_not_valid_utf8_is_enumerated_and_not_parsed() {
    let fixture = fixture("non-utf8");
    fs::write(fixture.root.join("binary.md"), [0xff, 0xfe, 0x00, 0x01]).expect("binary fixture");

    let sources = fixture.source.enumerate_sources().expect("enumerate");

    assert_eq!(sources.len(), 1);
    assert!(sources[0].fields.is_none());
    assert!(sources[0].fingerprint.is_some());
}

#[test]
fn enumeration_is_ordered_so_an_interrupted_run_resumes_the_way_it_started() {
    // Directory iteration order is unspecified. An unstable order would make a resumed run process
    // sources in a different sequence than the run it is resuming, which is exactly what a resume
    // must not do.
    let fixture = fixture("ordering");
    for name in ["charlie.md", "alpha.md", "bravo.md"] {
        fixture.write(name, LEGACY);
    }

    assert_eq!(
        fixture.names(),
        vec![
            "alpha.md".to_string(),
            "bravo.md".to_string(),
            "charlie.md".to_string()
        ]
    );
}

#[test]
fn reading_raw_returns_the_bytes_on_disk_unchanged() {
    let fixture = fixture("read-raw");
    let raw = LEGACY.replace('\n', "\r\n");
    fixture.write("crlf.md", &raw);

    assert_eq!(
        fixture.source.read_raw(&locator("crlf.md")).expect("read"),
        raw.as_bytes()
    );
}

#[test]
fn a_backup_reproduces_the_source_byte_for_byte() {
    // The backup exists so a source an older build could read can be restored. That means the
    // original bytes, not a re-serialization of a parsed body.
    let fixture = fixture("backup");
    let raw = LEGACY.replace('\n', "\r\n");
    fixture.write("crlf.md", &raw);
    let bytes = fixture.source.read_raw(&locator("crlf.md")).expect("read");

    let relative = fixture
        .source
        .write_backup(&locator("crlf.md"), &bytes)
        .expect("backup");

    assert_eq!(relative, "legacy-backup/crlf.md");
    assert_eq!(
        fixture.source.read_backup(&relative).expect("read back"),
        raw.as_bytes()
    );
    // And the original is untouched by the copy.
    assert_eq!(
        fs::read(fixture.root.join("crlf.md")).expect("original"),
        raw.as_bytes()
    );
}

#[test]
fn a_backup_is_rewritten_rather_than_refused_when_a_run_is_resumed() {
    let fixture = fixture("backup-resume");
    fixture.write("resumed.md", LEGACY);
    let bytes = fixture
        .source
        .read_raw(&locator("resumed.md"))
        .expect("read");
    fixture
        .source
        .write_backup(&locator("resumed.md"), &bytes)
        .expect("first backup");

    let relative = fixture
        .source
        .write_backup(&locator("resumed.md"), &bytes)
        .expect("a resumed run rewrites its own backup");

    assert_eq!(
        fixture.source.read_backup(&relative).expect("read back"),
        bytes
    );
}

#[test]
fn a_backup_path_outside_the_backup_directory_is_refused() {
    let fixture = fixture("backup-escape");
    fixture.write("target.md", LEGACY);

    for refused in [
        "target.md",
        "legacy-backup/../target.md",
        "legacy-backup/nested/target.md",
        "legacy-backup/",
        "quarantine/target.md",
    ] {
        assert!(
            fixture.source.read_backup(refused).is_err(),
            "expected {refused:?} to be refused"
        );
    }
}

#[test]
fn removing_a_source_twice_is_the_end_state_the_caller_asked_for() {
    // A run interrupted between the removal and its journal entry resumes into exactly this call.
    let fixture = fixture("remove");
    fixture.write("doomed.md", LEGACY);

    fixture
        .source
        .remove_source(&locator("doomed.md"))
        .expect("remove");
    fixture
        .source
        .remove_source(&locator("doomed.md"))
        .expect("removing an absent source is not an error");

    assert!(!fixture.root.join("doomed.md").exists());
}

#[test]
fn quarantine_moves_the_file_and_keeps_its_bytes() {
    let fixture = fixture("quarantine");
    fixture.write("bad.md", "---\nname: x\n");

    let relative = fixture
        .source
        .quarantine_source(&locator("bad.md"))
        .expect("quarantine");

    assert_eq!(relative, "quarantine/bad.md");
    assert!(!fixture.root.join("bad.md").exists());
    assert_eq!(
        fs::read_to_string(fixture.root.join("quarantine").join("bad.md")).expect("quarantined"),
        "---\nname: x\n"
    );
}

#[test]
fn a_second_quarantine_of_the_same_name_does_not_overwrite_the_first() {
    // Overwriting would destroy the earlier quarantined file, which is the single outcome
    // quarantine exists to prevent.
    let fixture = fixture("quarantine-clash");
    fixture.write("bad.md", "first");
    fixture
        .source
        .quarantine_source(&locator("bad.md"))
        .expect("first");
    fixture.write("bad.md", "second");

    let relative = fixture
        .source
        .quarantine_source(&locator("bad.md"))
        .expect("second");

    assert_eq!(relative, "quarantine/bad-1.md");
    let quarantine = fixture.root.join("quarantine");
    assert_eq!(
        fs::read_to_string(quarantine.join("bad.md")).expect("first survives"),
        "first"
    );
    assert_eq!(
        fs::read_to_string(quarantine.join("bad-1.md")).expect("second"),
        "second"
    );
}

#[test]
fn a_locator_that_is_not_a_plain_file_name_is_refused() {
    let fixture = fixture("locator-shape");
    // `NormalizedLegacyPath` already refuses traversal and absolute paths, so the cases that can
    // reach here are the nested ones it allows through.
    let nested = LegacySourceLocator::markdown("nested/inner.md").expect("locator");

    assert!(fixture.source.read_raw(&nested).is_err());
    assert!(fixture.source.remove_source(&nested).is_err());
    assert!(fixture.source.quarantine_source(&nested).is_err());
}

#[test]
fn a_held_directory_defers_a_mutation_instead_of_failing_it() {
    let fixture = fixture("busy");
    fixture.write("waiting.md", LEGACY);
    let bytes = fixture
        .source
        .read_raw(&locator("waiting.md"))
        .expect("read");
    let _held = fixture.lock.try_acquire().expect("hold the directory");

    for rejection in [
        fixture.source.write_backup(&locator("waiting.md"), &bytes),
        fixture
            .source
            .remove_source(&locator("waiting.md"))
            .map(|()| String::new()),
        fixture.source.quarantine_source(&locator("waiting.md")),
    ] {
        assert!(matches!(
            rejection,
            Err(PersonalizationApplicationError::MaintenanceBusy)
        ));
    }
    // Nothing was half-applied while the directory was held.
    assert!(fixture.root.join("waiting.md").exists());
}

#[test]
fn reading_never_takes_the_directory_lock() {
    // Enumeration runs over the whole directory. Serializing it against ordinary writes would make
    // a scan block every save for its duration, and it mutates nothing, so it does not need to.
    let fixture = fixture("read-while-held");
    fixture.write("readable.md", LEGACY);
    let _held = fixture.lock.try_acquire().expect("hold the directory");

    assert_eq!(fixture.names(), vec!["readable.md".to_string()]);
    assert!(fixture.source.read_raw(&locator("readable.md")).is_ok());
}

#[test]
fn an_injected_failure_reports_a_code_and_leaves_the_source_in_place() {
    // A seam rather than a real filesystem condition: a read-only file is deletable on Linux and
    // not on Windows, and a full disk cannot be produced on a developer's machine at all.
    let fixture = fixture("injected");
    fixture.write("fragile.md", LEGACY);
    let bytes = fixture
        .source
        .read_raw(&locator("fragile.md"))
        .expect("read");

    fixture
        .source
        .inject_failure(LegacyOperation::Backup, "fragile.md");
    assert!(fixture
        .source
        .write_backup(&locator("fragile.md"), &bytes)
        .is_err());
    fixture
        .source
        .clear_failure(LegacyOperation::Backup, "fragile.md");

    fixture
        .source
        .inject_failure(LegacyOperation::Remove, "fragile.md");
    assert!(fixture
        .source
        .remove_source(&locator("fragile.md"))
        .is_err());

    fixture
        .source
        .inject_failure(LegacyOperation::Quarantine, "fragile.md");
    assert!(fixture
        .source
        .quarantine_source(&locator("fragile.md"))
        .is_err());

    assert!(fixture.root.join("fragile.md").exists());
}

#[cfg(unix)]
#[test]
fn a_symlink_pointing_out_of_the_directory_is_enumerated_but_never_read() {
    // Windows needs elevation or developer mode to create symlinks, so this runs on unix only.
    // Quarantining such an entry renames the link, never its target.
    let fixture = fixture("symlink");
    let outside = TempDir::with_prefix("personalization-legacy-outside-").expect("outside dir");
    let target = outside.path().join("secret.md");
    fs::write(&target, LEGACY).expect("outside file");
    std::os::unix::fs::symlink(&target, fixture.root.join("link.md")).expect("symlink");

    let sources = fixture.source.enumerate_sources().expect("enumerate");

    assert_eq!(sources.len(), 1);
    assert!(sources[0].fields.is_none());
    // No fingerprint, because its bytes were never read: nothing outside the directory is copied
    // into a backup or hashed into durable state.
    assert!(sources[0].fingerprint.is_none());
    assert!(fixture.source.read_raw(&locator("link.md")).is_err());
    assert!(target.exists(), "the outside file must be untouched");
}
