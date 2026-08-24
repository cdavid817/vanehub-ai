//! The stable `legacy source identity -> memory id` mapping.
//!
//! v2 permits duplicate display names, so the old contract — saving under an existing name replaces
//! it — can no longer be implemented by searching for a name: the search can return several, and
//! choosing among them positionally would silently overwrite one of the user's memories. These
//! tests pin the alias that replaces the search.

use super::legacy_memory_bridge_tests::{fixture, mark_ready, now, reopen, seed, Fixture};
use super::{CompatibilitySaveInput, PersonalizationApi};
use crate::contexts::agent_runtime::application::{AgentMemoryPort, MemorySource, SaveMemoryInput};
use crate::contexts::agent_runtime::domain::MemoryType as RuntimeMemoryType;
use crate::contexts::personalization::application::{
    MigrationJournalPort, PersonalizationApplicationError,
};
use crate::contexts::personalization::domain::{
    LegacySourceId, MemoryAudience, MemoryScope, MemoryType as GovernedType, MigrationJournalEntry,
    MigrationStage,
};

fn save_named(fixture: &Fixture, name: &str, content: &str) {
    fixture
        .bridge
        .save(SaveMemoryInput {
            agent_id: "onepiece",
            folder: None,
            name: Some(name),
            description: Some("Package manager"),
            memory_type: Some(RuntimeMemoryType::Project),
            content,
            source: MemorySource::Explicit,
        })
        .expect("save");
}

fn legacy_id(name: &str) -> LegacySourceId {
    LegacySourceId::from_display_name(name).expect("legacy identity")
}

fn alias_target(fixture: &Fixture, name: &str) -> Option<String> {
    fixture
        .journal
        .get(&legacy_id(name))
        .expect("journal read")
        .and_then(|entry| entry.memory_id)
        .map(|id| format!("{id}.md"))
}

fn api(fixture: &Fixture) -> &PersonalizationApi {
    &fixture.api
}

#[test]
fn a_first_save_persists_an_alias_to_the_record_it_created() {
    let fixture = fixture("alias-created");
    mark_ready(&fixture);
    save_named(&fixture, "Use npm", "First");

    let listed = fixture.bridge.list_all().expect("listing");
    assert_eq!(
        alias_target(&fixture, "Use npm"),
        Some(listed[0].id.clone()),
        "the alias points at the record the save produced"
    );
}

#[test]
fn an_alias_addresses_its_exact_record_even_when_another_shares_the_name() {
    // The case a name search cannot answer: two visible records called "Use npm". Without the
    // alias, picking either would silently overwrite a memory the user did not mean.
    let fixture = fixture("alias-exact");
    mark_ready(&fixture);
    save_named(&fixture, "Use npm", "Aliased original");
    let aliased = fixture.bridge.list_all().expect("listing")[0].id.clone();

    // A second record with the same name, created outside the compatibility surface — exactly what
    // v2 now permits.
    let duplicate = seed(
        &fixture,
        "Use npm",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    assert_eq!(fixture.bridge.list_all().expect("listing").len(), 2);

    save_named(&fixture, "Use npm", "Updated through the alias");

    let listing = fixture.bridge.list_all().expect("listing");
    assert_eq!(listing.len(), 2, "no third record was created");
    let updated = listing
        .iter()
        .find(|memory| memory.id == aliased)
        .expect("the aliased record");
    assert_eq!(updated.content, "Updated through the alias");
    assert_eq!(
        fixture
            .service
            .detail(&duplicate.id)
            .expect("detail")
            .expect("still there")
            .content,
        "content for Use npm",
        "the record the alias does not point at is untouched"
    );
}

#[test]
fn two_same_named_records_without_an_alias_are_refused_rather_than_guessed_between() {
    let fixture = fixture("alias-ambiguous");
    mark_ready(&fixture);
    seed(
        &fixture,
        "Use npm",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    seed(
        &fixture,
        "Use npm",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );

    let error = api(&fixture)
        .save_compatibility_memory(CompatibilitySaveInput {
            agent_id: Some("onepiece".to_string()),
            workspace: None,
            name: "Use npm".to_string(),
            description: "Package manager".to_string(),
            memory_type: Some(GovernedType::Project),
            content: "Which one?".to_string(),
            is_automatic: false,
        })
        .expect_err("ambiguity must be refused");
    assert_eq!(
        error,
        PersonalizationApplicationError::AmbiguousLegacyName { matches: 2 }
    );
    assert_eq!(
        fixture.bridge.list_all().expect("listing").len(),
        2,
        "a refused save creates nothing"
    );
}

#[test]
fn a_single_pre_existing_match_is_adopted_and_gains_an_alias() {
    // How a memory that predates the journal acquires an identity: exactly one visible record with
    // this name is unambiguous, so it is adopted rather than duplicated.
    let fixture = fixture("alias-adopt");
    mark_ready(&fixture);
    let existing = seed(
        &fixture,
        "Use npm",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    assert_eq!(alias_target(&fixture, "Use npm"), None);

    save_named(&fixture, "Use npm", "Adopted");

    assert_eq!(fixture.bridge.list_all().expect("listing").len(), 1);
    assert_eq!(
        alias_target(&fixture, "Use npm"),
        Some(existing.file_name())
    );
    assert_eq!(
        fixture
            .service
            .detail(&existing.id)
            .expect("detail")
            .expect("exists")
            .content,
        "Adopted"
    );
}

#[test]
fn an_alias_survives_a_restart() {
    let original = fixture("alias-restart");
    mark_ready(&original);
    save_named(&original, "Use npm", "Before restart");
    let before = original.bridge.list_all().expect("listing")[0].id.clone();

    let reopened = reopen(original.directory_path.clone(), None);
    save_named(&reopened, "Use npm", "After restart");

    let listing = reopened.bridge.list_all().expect("listing");
    assert_eq!(
        listing.len(),
        1,
        "the alias survived, so nothing duplicated"
    );
    assert_eq!(listing[0].id, before);
    assert_eq!(listing[0].content, "After restart");
}

#[test]
fn an_alias_pointing_at_a_deleted_record_does_not_block_the_name() {
    let fixture = fixture("alias-dangling");
    mark_ready(&fixture);
    save_named(&fixture, "Use npm", "Original");
    let original = fixture.bridge.list_all().expect("listing")[0].id.clone();

    fixture.bridge.delete(&original).expect("delete");
    assert!(fixture.bridge.list_all().expect("listing").is_empty());

    save_named(&fixture, "Use npm", "Recreated");
    let listing = fixture.bridge.list_all().expect("listing");
    assert_eq!(listing.len(), 1);
    assert_ne!(listing[0].id, original, "a new record, not the dead one");
    assert_eq!(listing[0].content, "Recreated");
    assert_eq!(
        alias_target(&fixture, "Use npm"),
        Some(listing[0].id.clone()),
        "the stale alias was replaced rather than left dangling"
    );
}

#[test]
fn an_alias_recorded_before_its_record_was_verified_does_not_resolve() {
    // A journal row can exist while the v2 file is written but unproven. Addressing that record
    // would hand a caller something that might be torn, so an unverified stage does not resolve.
    let fixture = fixture("alias-unverified");
    mark_ready(&fixture);
    let record = seed(
        &fixture,
        "Use npm",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    fixture
        .journal
        .upsert(
            &MigrationJournalEntry {
                legacy_source_id: legacy_id("Some other name"),
                memory_id: Some(record.id.clone()),
                stage: MigrationStage::V2Written,
                legacy_backup_path: None,
                legacy_content_hash: None,
                last_error_code: None,
            },
            now(),
        )
        .expect("journal write");

    save_named(&fixture, "Some other name", "Created");

    assert_eq!(fixture.bridge.list_all().expect("listing").len(), 2);
    assert_eq!(
        fixture
            .service
            .detail(&record.id)
            .expect("detail")
            .expect("exists")
            .content,
        "content for Use npm",
        "the unverified alias target was not written through"
    );
}

#[test]
fn a_repeated_save_does_not_accumulate_alias_rows() {
    // Idempotence is keyed on the legacy source identity, not on content: an interrupted or
    // repeated run must not create a second alias for one source.
    let fixture = fixture("alias-idempotent");
    mark_ready(&fixture);
    for index in 0..4 {
        save_named(&fixture, "Use npm", &format!("Version {index}"));
    }
    assert_eq!(fixture.journal.list_all().expect("journal").len(), 1);
    assert_eq!(fixture.bridge.list_all().expect("listing").len(), 1);
}

#[test]
fn identical_content_under_two_names_stays_two_records_with_two_aliases() {
    // Deduplicating on content would merge two legacy sources that were always distinct.
    let fixture = fixture("alias-duplicate-content");
    mark_ready(&fixture);
    save_named(&fixture, "First name", "Exactly the same body");
    save_named(&fixture, "Second name", "Exactly the same body");

    assert_eq!(fixture.bridge.list_all().expect("listing").len(), 2);
    assert_eq!(fixture.journal.list_all().expect("journal").len(), 2);
    assert_ne!(
        alias_target(&fixture, "First name"),
        alias_target(&fixture, "Second name")
    );
}

#[test]
fn a_name_that_could_never_have_been_a_v1_filename_gets_no_alias() {
    let fixture = fixture("alias-impossible-name");
    mark_ready(&fixture);
    api(&fixture)
        .save_compatibility_memory(CompatibilitySaveInput {
            agent_id: Some("onepiece".to_string()),
            workspace: None,
            name: "Ratio 1:2 rules".to_string(),
            description: "A colon was never legal in a v1 filename".to_string(),
            memory_type: Some(GovernedType::Project),
            content: "Body".to_string(),
            is_automatic: false,
        })
        .expect("save");

    assert!(
        fixture.journal.list_all().expect("journal").is_empty(),
        "nothing under v1 could have created this memory, so it has no legacy identity"
    );
    assert_eq!(fixture.bridge.list_all().expect("listing").len(), 1);
}

#[test]
fn an_alias_update_goes_through_the_stable_id_and_its_current_revision() {
    // Blind-writing would lose an edit made between the read and the write. The alias path reads
    // the record and updates against the revision it just saw, so a concurrent change surfaces as
    // a conflict instead of being overwritten.
    let fixture = fixture("alias-revision");
    mark_ready(&fixture);
    save_named(&fixture, "Use npm", "First");
    let record_id = fixture.bridge.list_all().expect("listing")[0].id.clone();
    let stored = fixture
        .service
        .detail(
            &crate::contexts::personalization::domain::MemoryId::parse(
                record_id.trim_end_matches(".md"),
            )
            .expect("id"),
        )
        .expect("detail")
        .expect("exists");
    assert_eq!(stored.revision, 1);

    save_named(&fixture, "Use npm", "Second");
    let after = fixture
        .service
        .detail(&stored.id)
        .expect("detail")
        .expect("exists");
    assert_eq!(
        after.revision, 2,
        "the update advanced exactly one revision"
    );

    // A stale expected revision is refused by the store, which is what the alias path relies on.
    let conflict = fixture
        .service
        .update(
            &stored.id,
            1,
            crate::contexts::personalization::application::UpdateMemoryPatch {
                content: Some("Stale".to_string()),
                ..Default::default()
            },
        )
        .expect_err("a stale revision must be refused");
    assert!(matches!(
        conflict,
        PersonalizationApplicationError::RevisionConflict(_)
    ));
}
