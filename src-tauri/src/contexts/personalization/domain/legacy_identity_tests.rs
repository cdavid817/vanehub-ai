use super::legacy_identity::{LegacySourceId, MigrationStage};
use super::PersonalizationDomainError;

#[test]
fn a_legacy_identity_is_the_filename_a_display_name_used_to_produce() {
    // Reproduces the v1 rule rather than inventing one: the point is to recognize records a v1
    // caller would have considered the same memory.
    assert_eq!(
        LegacySourceId::from_display_name("Use npm")
            .expect("identity")
            .as_str(),
        "Use npm.md"
    );
    assert_eq!(
        LegacySourceId::from_display_name("  Use npm  ")
            .expect("identity")
            .as_str(),
        "Use npm.md",
        "the v1 name was trimmed before it became a filename"
    );
}

#[test]
fn a_name_that_could_never_have_been_a_v1_filename_has_no_legacy_identity() {
    // Correct rather than restrictive: nothing under v1 could have created such a memory, so there
    // is no legacy record for it to alias to.
    for name in [
        "Ratio 1:2",
        "a/b",
        "a\\b",
        "quote\"d",
        "pipe|d",
        "wild*card",
        "who?",
        "less<than",
        "greater>than",
        "",
        "   ",
    ] {
        assert!(
            matches!(
                LegacySourceId::from_display_name(name),
                Err(PersonalizationDomainError::InvalidLegacySourceId(_))
            ),
            "{name:?} must have no legacy identity"
        );
    }
}

#[test]
fn a_legacy_identity_cannot_carry_a_separator_or_a_control_character() {
    for value in [
        "",
        " leading",
        "trailing ",
        "a/b.md",
        "a\\b.md",
        "a\u{7f}.md",
    ] {
        assert!(
            matches!(
                LegacySourceId::parse(value),
                Err(PersonalizationDomainError::InvalidLegacySourceId(_))
            ),
            "{value:?} must be rejected"
        );
    }
    assert!(LegacySourceId::parse("Use npm.md").is_ok());
}

#[test]
fn migration_stages_round_trip_through_their_persisted_strings() {
    for stage in [
        MigrationStage::Discovered,
        MigrationStage::ManifestWritten,
        MigrationStage::V2Written,
        MigrationStage::V2Verified,
        MigrationStage::ProjectionWritten,
        MigrationStage::LegacyRemoved,
        MigrationStage::DerivedRebuilt,
        MigrationStage::Completed,
        MigrationStage::Failed,
    ] {
        assert_eq!(MigrationStage::parse(stage.as_str()), Ok(stage));
    }
    assert!(matches!(
        MigrationStage::parse("half-done"),
        Err(PersonalizationDomainError::UnknownMigrationStage(_))
    ));
}

#[test]
fn a_written_but_unverified_record_is_not_yet_usable() {
    // The file exists but has not been proven readable. Handing a caller a record that might be
    // torn is worse than telling them it is not there yet.
    assert!(!MigrationStage::V2Written.has_usable_memory());
    assert!(!MigrationStage::Discovered.has_usable_memory());
    assert!(!MigrationStage::ManifestWritten.has_usable_memory());
    assert!(!MigrationStage::Failed.has_usable_memory());

    for stage in [
        MigrationStage::V2Verified,
        MigrationStage::ProjectionWritten,
        MigrationStage::LegacyRemoved,
        MigrationStage::DerivedRebuilt,
        MigrationStage::Completed,
    ] {
        assert!(stage.has_usable_memory(), "{stage:?} should be usable");
    }
}

#[test]
fn stages_are_ordered_so_a_resume_can_compare_progress() {
    assert!(MigrationStage::Discovered < MigrationStage::ManifestWritten);
    assert!(MigrationStage::ManifestWritten < MigrationStage::V2Written);
    assert!(MigrationStage::V2Written < MigrationStage::V2Verified);
    assert!(MigrationStage::V2Verified < MigrationStage::ProjectionWritten);
    assert!(MigrationStage::ProjectionWritten < MigrationStage::LegacyRemoved);
    assert!(MigrationStage::LegacyRemoved < MigrationStage::DerivedRebuilt);
    assert!(MigrationStage::DerivedRebuilt < MigrationStage::Completed);
}
