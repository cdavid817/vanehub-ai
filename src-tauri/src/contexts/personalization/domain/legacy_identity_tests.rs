use super::legacy_identity::{
    LegacyAddressKey, LegacySourceFingerprint, LegacySourceId, LegacySourceLocator,
    LegacyTableKind, MigrationStage, NormalizedLegacyPath,
};
use super::PersonalizationDomainError;

fn file_source(relative: &str) -> LegacySourceLocator {
    LegacySourceLocator::markdown(relative).expect("locator")
}

fn source_id(relative: &str) -> String {
    file_source(relative).source_id().as_str().to_string()
}

// --- LegacyAddressKey: compatibility addressing --------------------------------------------------

#[test]
fn an_address_is_the_filename_a_display_name_used_to_produce() {
    assert_eq!(
        LegacyAddressKey::from_display_name("Use npm")
            .expect("address")
            .as_str(),
        "Use npm.md"
    );
    assert_eq!(
        LegacyAddressKey::from_display_name("  Use npm  ")
            .expect("address")
            .as_str(),
        "Use npm.md",
        "the v1 name was trimmed before it became a filename"
    );
}

#[test]
fn a_name_that_could_never_have_been_a_v1_filename_has_no_address() {
    for name in [
        "Ratio 1:2",
        "a/b",
        "a\\b",
        "quote\"d",
        "pipe|d",
        "wild*card",
        "",
        "   ",
    ] {
        assert!(
            matches!(
                LegacyAddressKey::from_display_name(name),
                Err(PersonalizationDomainError::InvalidLegacyAddressKey(_))
            ),
            "{name:?} must have no legacy address"
        );
    }
}

// --- LegacySourceId: migration identity ----------------------------------------------------------

#[test]
fn a_source_id_comes_from_where_the_source_was_found_not_from_its_name() {
    // The concrete case: a v1 file whose frontmatter `name` disagrees with its filename. Deriving
    // the migration identity from the name would journal it under a location that does not exist.
    let actual = source_id("renamed-on-disk.md");
    let from_name = LegacyAddressKey::from_display_name("Use npm")
        .expect("address")
        .as_str()
        .to_string();
    assert!(actual.contains("renamed-on-disk.md"));
    assert!(
        !actual.contains(&from_name),
        "the source id must not embed a name-derived address"
    );
}

#[test]
fn two_files_with_the_same_display_name_have_different_source_ids() {
    // v1 could not produce this, but a hand-placed or externally-synced directory can, and both
    // files are real sources that must migrate independently.
    assert_ne!(source_id("use-npm.md"), source_id("use-npm-copy.md"));
}

#[test]
fn two_files_with_identical_content_have_different_source_ids() {
    // Identity is location, not content. Deduplicating here would silently drop one of the user's
    // files; a duplicate is the user's to resolve, not the migration's.
    let left = file_source("a.md");
    let right = file_source("b.md");
    let bytes = b"---\nname: Same\n---\n\nIdentical body\n";
    let left_print = LegacySourceFingerprint::of(bytes);
    let right_print = LegacySourceFingerprint::of(bytes);

    assert_ne!(left.source_id(), right.source_id());
    assert_eq!(
        left_print, right_print,
        "identical bytes fingerprint identically, which is exactly why the fingerprint is not the id"
    );
}

#[test]
fn a_malformed_file_with_no_readable_name_still_has_a_source_id() {
    // A file whose frontmatter will not parse has no name to derive anything from. Journalling it
    // by location is what lets it be quarantined and reported rather than silently skipped.
    let locator = file_source("01K2BROKEN00000000000000000.md");
    assert!(locator.source_id().as_str().contains("01K2BROKEN"));
}

#[test]
fn a_markdown_file_and_a_sqlite_row_never_collide() {
    let file = file_source("agent_memories.md");
    let row = LegacySourceLocator::sqlite_row(LegacyTableKind::AgentMemories, "agent_memories.md")
        .expect("locator");
    assert_ne!(file.source_id(), row.source_id());
    assert!(file.source_id().as_str().contains(":file:"));
    assert!(row.source_id().as_str().contains(":row:"));
}

#[test]
fn a_source_id_carries_a_version_prefix() {
    // A stored id declares which derivation rule produced it, so a future rule change cannot be
    // mistaken for the current one.
    assert!(source_id("a.md").starts_with("v1:"));
}

#[test]
fn a_source_id_is_stable_while_its_content_changes() {
    let locator = file_source("a.md");
    let before = LegacySourceFingerprint::of(b"first body");
    let after = LegacySourceFingerprint::of(b"second body");

    assert_eq!(locator.source_id(), locator.source_id());
    assert_ne!(before, after, "the fingerprint is what notices the change");
    assert!(!before.matches(&after));
    assert!(before.matches(&LegacySourceFingerprint::of(b"first body")));
}

#[test]
fn a_fingerprint_compares_length_as_well_as_digest() {
    let real = LegacySourceFingerprint::of(b"body");
    let forged = LegacySourceFingerprint {
        raw_sha256: real.raw_sha256.clone(),
        byte_length: real.byte_length + 1,
    };
    assert!(!real.matches(&forged));
}

#[test]
fn a_source_path_rejects_traversal_and_absolute_forms() {
    for path in [
        "../escape.md",
        "./a.md",
        "/absolute.md",
        "C:/absolute.md",
        "",
        "   ",
        "a\u{7f}.md",
    ] {
        assert!(
            matches!(
                NormalizedLegacyPath::parse(path),
                Err(PersonalizationDomainError::InvalidLegacySourcePath(_))
            ),
            "{path:?} must be rejected"
        );
    }
    assert_eq!(
        NormalizedLegacyPath::parse("nested\\inner.md")
            .expect("normalized")
            .as_str(),
        "nested/inner.md",
        "separators normalize rather than being rejected, so one directory has one identity"
    );
}

#[test]
fn a_source_id_never_embeds_a_machine_absolute_path() {
    // The id is durable state. An absolute path in it would break when the application data
    // directory moved, and would leak the machine's layout into the database.
    let id = source_id("a.md");
    assert!(!id.contains(":\\"));
    assert!(!id.contains("/Users/"));
    assert!(!id.contains("/home/"));
    assert_eq!(id, "v1:file:a.md");
}

#[test]
fn a_persisted_source_id_round_trips() {
    let original = file_source("a.md").source_id();
    assert_eq!(
        LegacySourceId::parse(original.as_str()).expect("parse"),
        original
    );
    assert!(matches!(
        LegacySourceId::parse(""),
        Err(PersonalizationDomainError::InvalidLegacySourceId(_))
    ));
}

#[test]
fn the_two_identities_are_not_interchangeable() {
    // Structural rather than behavioural: there is no From impl in either direction, so a source id
    // cannot reach an alias lookup and an address cannot reach the journal. This test documents the
    // intent that the type system enforces at every call site.
    let address = LegacyAddressKey::from_display_name("Use npm").expect("address");
    let source = file_source("Use npm.md").source_id();
    assert_ne!(address.as_str(), source.as_str());
    assert_eq!(address.as_str(), "Use npm.md");
    assert_eq!(source.as_str(), "v1:file:Use npm.md");
}

// --- Journal stages ------------------------------------------------------------------------------

#[test]
fn migration_stages_round_trip_through_their_persisted_strings() {
    for stage in [
        MigrationStage::Discovered,
        MigrationStage::BackupWritten,
        MigrationStage::BackupVerified,
        MigrationStage::V2Written,
        MigrationStage::V2Verified,
        MigrationStage::ProjectionWritten,
        MigrationStage::LegacyRemoved,
        MigrationStage::DerivedPending,
        MigrationStage::Completed,
        MigrationStage::Failed,
        MigrationStage::SourceChanged,
    ] {
        assert_eq!(MigrationStage::parse(stage.as_str()), Ok(stage));
    }
    assert!(matches!(
        MigrationStage::parse("half-done"),
        Err(PersonalizationDomainError::UnknownMigrationStage(_))
    ));
}

#[test]
fn a_written_but_unverified_record_is_not_usable_yet_is_still_resumable() {
    // The file exists but has not been proven readable, so an ordinary reader must not see it.
    // A resume run reaches it by journal id, which is what stops it being permanently stranded.
    assert!(!MigrationStage::V2Written.has_usable_memory());
    assert!(MigrationStage::V2Written.is_resumable());

    for stage in [
        MigrationStage::V2Verified,
        MigrationStage::ProjectionWritten,
        MigrationStage::LegacyRemoved,
        MigrationStage::DerivedPending,
        MigrationStage::Completed,
    ] {
        assert!(stage.has_usable_memory(), "{stage:?} should be usable");
    }
    for stage in [
        MigrationStage::Discovered,
        MigrationStage::BackupWritten,
        MigrationStage::BackupVerified,
        MigrationStage::Failed,
        MigrationStage::SourceChanged,
    ] {
        assert!(!stage.has_usable_memory(), "{stage:?} must not be usable");
    }
}

#[test]
fn terminal_stages_are_not_resumed() {
    for stage in [
        MigrationStage::Completed,
        MigrationStage::Failed,
        MigrationStage::SourceChanged,
    ] {
        assert!(!stage.is_resumable(), "{stage:?} is terminal");
    }
}

#[test]
fn stages_are_ordered_so_a_resume_can_compare_progress() {
    assert!(MigrationStage::Discovered < MigrationStage::BackupWritten);
    assert!(MigrationStage::BackupWritten < MigrationStage::BackupVerified);
    assert!(MigrationStage::BackupVerified < MigrationStage::V2Written);
    assert!(MigrationStage::V2Written < MigrationStage::V2Verified);
    assert!(MigrationStage::V2Verified < MigrationStage::ProjectionWritten);
    assert!(MigrationStage::ProjectionWritten < MigrationStage::LegacyRemoved);
    assert!(MigrationStage::LegacyRemoved < MigrationStage::DerivedPending);
    assert!(MigrationStage::DerivedPending < MigrationStage::Completed);
}

#[test]
fn the_legacy_source_may_still_exist_before_it_is_removed() {
    assert!(MigrationStage::BackupVerified.legacy_source_may_exist());
    assert!(MigrationStage::V2Verified.legacy_source_may_exist());
    assert!(!MigrationStage::LegacyRemoved.legacy_source_may_exist());
    assert!(!MigrationStage::Completed.legacy_source_may_exist());
    // A failed or changed source was never removed, so it is still there to repair.
    assert!(MigrationStage::Failed.legacy_source_may_exist());
    assert!(MigrationStage::SourceChanged.legacy_source_may_exist());
}
