//! What survives migration about *where a memory came from*.
//!
//! Split from the migration tests because the question is different: those ask whether the text
//! arrives intact and recoverably, these ask whether the record still says how it was produced and
//! which workspace it belonged to. Both run against the real store, because the property is that
//! the values survive a write and a re-read, not that they were assembled correctly in memory.

use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use tempfile::TempDir;

use super::{
    FileLegacyMemorySource, MarkdownMemoryRepository, SqliteLegacyAddressAlias,
    SqliteMemoryProjection, SqliteMigrationJournal, UuidMemoryIdGenerator,
};
use crate::contexts::personalization::application::{
    legacy_workspace_request, ClockPort, LegacyMemoryMigrationPorts, LegacyMemoryMigrationService,
    MemoryMaintenanceRepository, MemoryProjectionPort, MemoryRepository, MigrationRunOutcome,
    WorkspaceIdentityResolver,
};
use crate::contexts::personalization::domain::{
    LegacyMemorySaveSource, MemoryRecord, MemorySource, OwnedEntryClassification,
    MEMORY_LEGACY_FIELD_MAX_CHARS,
};
use crate::platform::database::NativeDatabase;

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 24, 9, 0, 0).unwrap()
}

struct FixedClock;

impl ClockPort for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        now()
    }
}

struct Fixture {
    _directory: TempDir,
    root: std::path::PathBuf,
    repository: Arc<MarkdownMemoryRepository>,
    projection: Arc<SqliteMemoryProjection>,
    service: LegacyMemoryMigrationService,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDir::with_prefix(format!("personalization-provenance-{label}-"))
        .expect("temporary dir");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let root = directory.path().join("memory");
    let repository = Arc::new(
        MarkdownMemoryRepository::new(root.clone(), Arc::new(UuidMemoryIdGenerator))
            .expect("repository"),
    );
    let sources = Arc::new(
        FileLegacyMemorySource::new(root.clone(), repository.lock()).expect("legacy source"),
    );
    let projection = Arc::new(SqliteMemoryProjection::new(database.clone()));
    let service = LegacyMemoryMigrationService::new(LegacyMemoryMigrationPorts {
        sources,
        repository: repository.clone(),
        projection: projection.clone(),
        journal: Arc::new(SqliteMigrationJournal::new(database.clone())),
        aliases: Arc::new(SqliteLegacyAddressAlias::new(database)),
        identity: Arc::new(WorkspaceIdentityResolver::for_this_platform()),
        ids: Arc::new(UuidMemoryIdGenerator),
        clock: Arc::new(FixedClock),
    });
    Fixture {
        _directory: directory,
        root,
        repository,
        projection,
        service,
    }
}

impl Fixture {
    /// Writes one v1 file with the frontmatter lines a caller chooses, and migrates it.
    fn migrate(&self, file_name: &str, extra_frontmatter: &str) -> MigrationRunOutcome {
        std::fs::write(
            self.root.join(file_name),
            format!("---\nname: subject\ndescription: d\n{extra_frontmatter}---\n\nBody.\n"),
        )
        .expect("legacy fixture");
        self.service.run().expect("migration run")
    }

    /// The single migrated record, read back off disk rather than returned from the write.
    fn record(&self) -> MemoryRecord {
        let mut found = Vec::new();
        for entry in self
            .repository
            .enumerate_owned_entries()
            .expect("enumerate v2")
        {
            if entry.classification == OwnedEntryClassification::ValidV2 {
                let id = entry.memory_id.expect("a valid v2 entry has an id");
                found.push(self.repository.get(&id).expect("read").expect("present"));
            }
        }
        assert_eq!(found.len(), 1, "expected exactly one migrated record");
        found.remove(0)
    }
}

#[test]
fn an_explicit_legacy_memory_keeps_its_original_save_source() {
    let fixture = fixture("explicit");

    fixture.migrate("explicit.md", "source: explicit\n");

    let record = fixture.record();
    // The entry route and the original production are different facts, and both survive.
    assert_eq!(record.source, MemorySource::LegacyMigration);
    assert_eq!(
        record.provenance.legacy_original_save_source,
        Some(LegacyMemorySaveSource::Explicit)
    );
}

#[test]
fn an_automatic_legacy_memory_keeps_its_original_save_source() {
    let fixture = fixture("automatic");

    fixture.migrate("automatic.md", "source: automatic\n");

    let record = fixture.record();
    assert_eq!(record.source, MemorySource::LegacyMigration);
    assert_eq!(
        record.provenance.legacy_original_save_source,
        Some(LegacyMemorySaveSource::Automatic)
    );
}

#[test]
fn an_absent_or_unrecognized_save_source_never_becomes_automatic() {
    // The failure this closes: defaulting relabels a fact the user stated as one an Agent inferred,
    // and once the source file is gone there is no way to tell them apart again.
    for (label, frontmatter) in [
        ("absent", ""),
        ("empty", "source: \n"),
        ("unknown", "source: something-this-build-never-wrote\n"),
    ] {
        let fixture = fixture(&format!("no-source-{label}"));

        fixture.migrate("subject.md", frontmatter);

        let record = fixture.record();
        assert_eq!(
            record.provenance.legacy_original_save_source, None,
            "{label}: an unknown origin must stay unknown"
        );
        assert_eq!(record.source, MemorySource::LegacyMigration, "{label}");
    }
}

#[test]
fn a_resolvable_folder_yields_both_the_raw_value_and_a_stable_key() {
    let fixture = fixture("resolvable");

    fixture.migrate("subject.md", "folder: D:/code/vanehub-ai\n");

    let record = fixture.record();
    assert_eq!(
        record.provenance.legacy_folder.as_deref(),
        Some("D:/code/vanehub-ai"),
        "the raw value the file recorded is kept verbatim"
    );
    let key = record
        .provenance
        .source_workspace_key
        .as_ref()
        .expect("an absolute local path resolves");
    assert!(key.as_str().starts_with("ws_"));
    // Scope and audience are untouched by any of this: nothing a user could read before migration
    // becomes invisible after it.
    assert_eq!(
        record.scope,
        crate::contexts::personalization::domain::MemoryScope::Global
    );
    assert_eq!(
        record.audience,
        crate::contexts::personalization::domain::MemoryAudience::AllAgents
    );
}

#[test]
fn the_same_folder_always_derives_the_same_key() {
    let first = fixture("stable-key-a");
    first.migrate("subject.md", "folder: /home/user/project\n");
    let second = fixture("stable-key-b");
    second.migrate("subject.md", "folder: /home/user/project\n");

    assert_eq!(
        first.record().provenance.source_workspace_key,
        second.record().provenance.source_workspace_key
    );
    assert!(first.record().provenance.source_workspace_key.is_some());
}

#[test]
fn a_folder_that_does_not_identify_one_root_yields_no_key_and_keeps_the_raw_value() {
    // Each of these names a different directory depending on something nobody recorded: a working
    // directory, a current drive, or which mapping of a share the machine happens to have. A key
    // derived from one would compare equal to workspaces it is not.
    for (label, folder) in [
        ("relative", "code/vanehub-ai"),
        ("dot", "."),
        ("home", "~/code/vanehub-ai"),
        ("drive-relative", "C:notes"),
        ("bare-name", "vanehub-ai"),
        ("unc-backslash", r"\\fileserver\share\project"),
        ("unc-slash", "//fileserver/share/project"),
        ("single-backslash", r"\projects\vanehub"),
    ] {
        let fixture = fixture(&format!("unresolvable-{label}"));

        fixture.migrate("subject.md", &format!("folder: {folder}\n"));

        let record = fixture.record();
        assert_eq!(
            record.provenance.source_workspace_key, None,
            "{label}: {folder:?} must not produce an invented key"
        );
        // Present without a key is the diagnostic: an origin was recorded and could not be resolved.
        assert_eq!(
            record.provenance.legacy_folder.as_deref(),
            Some(folder),
            "{label}: the raw value must survive anyway"
        );
    }
}

#[test]
fn a_unicode_folder_survives_verbatim_and_still_derives_a_key() {
    let fixture = fixture("unicode");
    let folder = "D:/代码/项目-ünïcode/研究";

    fixture.migrate("subject.md", &format!("folder: {folder}\n"));

    let record = fixture.record();
    assert_eq!(record.provenance.legacy_folder.as_deref(), Some(folder));
    assert!(record.provenance.source_workspace_key.is_some());
}

#[test]
fn a_windows_folder_survives_in_the_spelling_the_file_used() {
    // Normalization belongs to key derivation, not to the recorded value: the raw string is what a
    // user recognizes, and rewriting it would make the provenance disagree with their own notes.
    let fixture = fixture("windows-path");
    let folder = r"D:\cdavid\Documents\code\vanehub-ai";

    fixture.migrate("subject.md", &format!("folder: {folder}\n"));

    let record = fixture.record();
    assert_eq!(record.provenance.legacy_folder.as_deref(), Some(folder));
    assert!(record.provenance.source_workspace_key.is_some());
}

#[test]
fn a_remote_uri_folder_keeps_credentials_out_of_the_key_and_the_record() {
    // An identity derived from a secret changes when the secret rotates, and would put recoverable
    // material into a value that appears in diagnostics.
    let with_password = "ssh://alice:hunter2@build.example.test:2222/srv/project";
    let without_password = "ssh://alice@build.example.test:2222/srv/project";

    let credentialed = fixture("remote-credentials");
    credentialed.migrate("subject.md", &format!("folder: {with_password}\n"));
    let record = credentialed.record();

    let key = record
        .provenance
        .source_workspace_key
        .as_ref()
        .expect("a remote URI with a host resolves");
    assert!(!key.as_str().contains("hunter2"));

    // And the same connection without the password derives the same key, which is what proves the
    // password never entered the derivation rather than merely not being readable in the output.
    let plain = fixture("remote-no-credentials");
    plain.migrate("subject.md", &format!("folder: {without_password}\n"));
    assert_eq!(
        plain.record().provenance.source_workspace_key.as_ref(),
        Some(key)
    );
}

#[test]
fn a_file_with_no_folder_records_neither_a_folder_nor_a_key() {
    let fixture = fixture("no-folder");

    fixture.migrate("subject.md", "source: explicit\n");

    let record = fixture.record();
    assert_eq!(record.provenance.legacy_folder, None);
    assert_eq!(record.provenance.source_workspace_key, None);
}

#[test]
fn the_migrated_record_remembers_which_file_it_came_from() {
    let fixture = fixture("source-path");

    fixture.migrate("some-old-memory.md", "");

    assert_eq!(
        fixture
            .record()
            .provenance
            .legacy_source_relative_path
            .as_deref(),
        Some("some-old-memory.md")
    );
}

#[test]
fn a_folder_beyond_the_recorded_bound_fails_rather_than_being_truncated() {
    // A truncated path points somewhere else. Failing leaves the source in place for a human.
    let fixture = fixture("oversized-folder");
    let folder = "D:/".to_string() + &"x".repeat(MEMORY_LEGACY_FIELD_MAX_CHARS);

    let outcome = fixture.migrate("subject.md", &format!("folder: {folder}\n"));

    assert_eq!(outcome.failed, 1);
    assert_eq!(outcome.migrated, 0);
    assert!(fixture.root.join("subject.md").exists());
}

#[test]
fn provenance_survives_the_projection_round_trip() {
    let fixture = fixture("projection");

    fixture.migrate(
        "subject.md",
        "source: automatic\nfolder: D:/code/vanehub-ai\n",
    );

    let record = fixture.record();
    let projected = fixture.projection.projected_ids().expect("projected");
    assert_eq!(projected, vec![record.id.clone()]);

    // Read straight out of the table: the projection is what a diagnostic query reads, so the
    // columns have to hold the values rather than the record merely having them in memory.
    let directory = fixture.root.parent().expect("data root").to_path_buf();
    let database = NativeDatabase::new(directory).expect("database");
    let connection = database.connection().expect("connection");
    let (save_source, folder, source_path, key): (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
    ) = connection
        .query_row(
            "SELECT legacy_save_source, legacy_folder, legacy_source_path, source_workspace_key
             FROM personalization_memory_projection WHERE memory_id = ?1",
            rusqlite::params![record.id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .expect("projected row");

    assert_eq!(save_source.as_deref(), Some("automatic"));
    assert_eq!(folder.as_deref(), Some("D:/code/vanehub-ai"));
    assert_eq!(source_path.as_deref(), Some("subject.md"));
    assert!(key.is_some());
}

#[test]
fn provenance_survives_a_run_that_is_interrupted_and_resumed() {
    use crate::contexts::personalization::domain::MigrationStage;

    let fixture = fixture("interrupted");
    fixture
        .service
        .interrupt_after(MigrationStage::BackupVerified);
    let interrupted = fixture.migrate(
        "subject.md",
        "source: explicit\nfolder: D:/code/vanehub-ai\n",
    );
    assert_eq!(interrupted.migrated, 0);

    let resumed = fixture.service.run().expect("resume");

    assert_eq!(resumed.migrated, 1);
    let record = fixture.record();
    assert_eq!(
        record.provenance.legacy_original_save_source,
        Some(LegacyMemorySaveSource::Explicit)
    );
    assert_eq!(
        record.provenance.legacy_folder.as_deref(),
        Some("D:/code/vanehub-ai")
    );
    assert_eq!(
        record.provenance.legacy_source_relative_path.as_deref(),
        Some("subject.md")
    );
}

#[test]
fn the_folder_resolution_rule_is_one_rule_shared_by_every_caller() {
    // Exported and tested directly because the compatibility save path uses it too. Two rules for
    // one input is how the two surfaces would come to disagree about which workspace a memory is in.
    for resolvable in [
        "D:/code/vanehub-ai",
        r"D:\code\vanehub-ai",
        "/home/user/project",
        "ssh://host.example.test/srv/project",
    ] {
        assert!(
            legacy_workspace_request(resolvable).is_some(),
            "{resolvable:?} identifies one root"
        );
    }
    for refused in ["", "   ", "code/project", ".", "C:notes", r"\\host\share"] {
        assert!(
            legacy_workspace_request(refused).is_none(),
            "{refused:?} does not identify one root"
        );
    }
}
