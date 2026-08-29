//! The boundary the settings and memory screens will call, exercised over the real store.

use super::compatibility_tests::{fixture, mark_ready, seed};
use crate::contexts::personalization::application::{
    CreateMemoryInput, PersonalizationApplicationError, UpdateMemoryPatch,
};
use crate::contexts::personalization::domain::{
    AgentId, InstructionMergeMode, MemoryAudience, MemoryProvenance, MemoryQuery, MemoryScope,
    MemoryScopeFilter, MemorySensitivity, MemorySource, MemoryStatus, MemoryType,
    PersonalizationPolicyPatch, PersonalizationPolicyScope, PolicyToggle,
};

fn agent_scope(agent_id: &str) -> PersonalizationPolicyScope {
    PersonalizationPolicyScope::Agent {
        agent_id: AgentId::parse(agent_id).expect("agent id"),
    }
}

fn instructions_patch(text: &str) -> PersonalizationPolicyPatch {
    PersonalizationPolicyPatch {
        style_rules: Some(text.to_string()),
        instruction_merge_mode: Some(InstructionMergeMode::Append),
        ..PersonalizationPolicyPatch::default()
    }
}

/// A layer with no row inherits, and that is different from a layer configured to nothing.
///
/// A screen that rendered "nothing stored" as "an override set to empty" would show the user a
/// choice they never made, and saving from that screen would then write it.
#[test]
fn a_scope_with_no_stored_policy_reports_absent_rather_than_empty() {
    let fixture = fixture("management-absent-policy");

    let policy = fixture
        .api
        .policy(&agent_scope("onepiece"))
        .expect("policy read");

    assert!(policy.is_none());
}

/// Creating a layer, then editing it from a stale copy.
///
/// Two screens editing the same layer is the ordinary case; last-response-wins would silently
/// discard whichever save arrived first, which is exactly what an expected revision prevents.
#[test]
fn patching_a_policy_from_a_stale_revision_is_a_conflict() {
    let fixture = fixture("management-policy-conflict");
    let created = fixture
        .api
        .patch_policy(
            &agent_scope("onepiece"),
            None,
            instructions_patch("Be terse."),
        )
        .expect("create layer");
    let updated = fixture
        .api
        .patch_policy(
            &agent_scope("onepiece"),
            Some(created.revision()),
            instructions_patch("Be brief."),
        )
        .expect("first edit");
    assert!(updated.revision() > created.revision());

    // A second screen still holding the original revision.
    let stale = fixture.api.patch_policy(
        &agent_scope("onepiece"),
        Some(created.revision()),
        instructions_patch("Be terser."),
    );

    assert!(matches!(
        stale,
        Err(PersonalizationApplicationError::RevisionConflict(_))
    ));
    assert_eq!(
        fixture
            .api
            .policy(&agent_scope("onepiece"))
            .expect("policy")
            .expect("record")
            .style_rules(),
        "Be brief.",
        "the save that lost the race must not have overwritten the one that won"
    );
}

/// A saved policy is what the next resolution reads.
///
/// The cached bundle is dropped by the write itself, so a value the user just replaced cannot
/// answer the following resolution. Without that, a setting would appear to take effect only after
/// something else happened to evict the cache.
#[test]
fn a_saved_policy_reaches_the_next_resolution() {
    let fixture = fixture("management-policy-visible");
    mark_ready(&fixture);
    fixture
        .api
        .patch_policy(
            &PersonalizationPolicyScope::Global,
            None,
            instructions_patch("Be terse."),
        )
        .expect("seed global");

    let before = fixture.api.legacy_settings().expect("settings");
    fixture
        .api
        .patch_policy(
            &PersonalizationPolicyScope::Global,
            Some(before.revision),
            instructions_patch("Answer in Chinese."),
        )
        .expect("save");

    let after = fixture.api.legacy_settings().expect("settings");
    assert_eq!(
        after.settings.style_rules.as_deref(),
        Some("Answer in Chinese.")
    );
}

/// A memory the user writes themselves is an active record straight away.
#[test]
fn a_user_created_memory_is_active_immediately_and_appears_in_a_page() {
    let fixture = fixture("management-create");
    mark_ready(&fixture);

    let record = fixture
        .api
        .create_memory(CreateMemoryInput {
            name: "npm-only".to_string(),
            description: "Package manager".to_string(),
            memory_type: MemoryType::Project,
            content: "Never pnpm in this repo.".to_string(),
            scope: MemoryScope::Global,
            audience: MemoryAudience::AllAgents,
            status: MemoryStatus::Active,
            source: MemorySource::ExplicitUser,
            provenance: MemoryProvenance::default(),
            sensitivity: MemorySensitivity::Normal,
        })
        .expect("create");

    let page = fixture
        .api
        .list_memories(&MemoryQuery::default())
        .expect("page");
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].name, "npm-only");
    let detail = fixture
        .api
        .memory_detail(&record.id)
        .expect("detail")
        .expect("record");
    assert_eq!(detail.content, "Never pnpm in this repo.");
}

/// Whitespace is not content.
///
/// The bounded-length check alone admits it — three spaces are three characters — so the guard is
/// on the create path, where the writer that used to trim no longer exists.
#[test]
fn a_memory_whose_content_is_only_whitespace_is_refused() {
    let fixture = fixture("management-blank");
    mark_ready(&fixture);

    let refused = fixture.api.create_memory(CreateMemoryInput {
        name: "blank".to_string(),
        description: "Nothing".to_string(),
        memory_type: MemoryType::Project,
        content: "   ".to_string(),
        scope: MemoryScope::Global,
        audience: MemoryAudience::AllAgents,
        status: MemoryStatus::Active,
        source: MemorySource::ExplicitUser,
        provenance: MemoryProvenance::default(),
        sensitivity: MemorySensitivity::Normal,
    });

    assert!(refused.is_err());
    assert!(fixture
        .api
        .list_memories(&MemoryQuery::default())
        .expect("page")
        .items
        .is_empty());
}

/// An edit from a stale copy is refused rather than applied.
#[test]
fn updating_a_memory_from_a_stale_revision_is_a_conflict() {
    let fixture = fixture("management-update-conflict");
    mark_ready(&fixture);
    let record = seed(
        &fixture,
        "npm-only",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    fixture
        .api
        .update_memory(
            &record.id,
            record.revision,
            UpdateMemoryPatch {
                content: Some("First edit.".to_string()),
                ..UpdateMemoryPatch::default()
            },
        )
        .expect("first edit");

    let stale = fixture.api.update_memory(
        &record.id,
        record.revision,
        UpdateMemoryPatch {
            content: Some("Second edit from a stale screen.".to_string()),
            ..UpdateMemoryPatch::default()
        },
    );

    assert!(matches!(
        stale,
        Err(PersonalizationApplicationError::RevisionConflict(_))
    ));
    let detail = fixture
        .api
        .memory_detail(&record.id)
        .expect("detail")
        .expect("record");
    assert_eq!(detail.content, "First edit.");
}

/// What a reset would remove, counted before anything is removed.
#[test]
fn a_reset_preview_counts_what_is_there_without_removing_it() {
    let fixture = fixture("management-reset-preview");
    mark_ready(&fixture);
    seed(
        &fixture,
        "first",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );
    seed(
        &fixture,
        "second",
        MemoryScope::Global,
        MemoryAudience::AllAgents,
    );

    let counts = fixture
        .api
        .preview_memory_reset(&MemoryScopeFilter::Any, &[MemoryStatus::Active])
        .expect("preview");

    assert_eq!(counts.matched, 2);
    assert_eq!(
        fixture
            .api
            .list_memories(&MemoryQuery::default())
            .expect("page")
            .items
            .len(),
        2,
        "a preview must not remove anything"
    );
}

/// Reads answer empty rather than failing while migration owns the directory.
///
/// A read that saw a half-migrated set would be worse than one that saw nothing, and a screen that
/// could not render at all during startup maintenance would be worse than one showing an empty
/// list it will refresh.
#[test]
fn reads_answer_empty_while_migration_owns_the_directory() {
    let fixture = fixture("management-not-ready");

    let page = fixture
        .api
        .list_memories(&MemoryQuery::default())
        .expect("page");
    let write = fixture.api.create_memory(CreateMemoryInput {
        name: "npm-only".to_string(),
        description: "Package manager".to_string(),
        memory_type: MemoryType::Project,
        content: "Never pnpm in this repo.".to_string(),
        scope: MemoryScope::Global,
        audience: MemoryAudience::AllAgents,
        status: MemoryStatus::Active,
        source: MemorySource::ExplicitUser,
        provenance: MemoryProvenance::default(),
        sensitivity: MemorySensitivity::Normal,
    });

    assert!(page.items.is_empty());
    // A write, by contrast, is refused with a typed error: a caller told a save succeeded when it
    // did not would have no way to notice.
    assert!(matches!(
        write,
        Err(PersonalizationApplicationError::MaintenanceRequired)
    ));
}

/// The global policy toggles the memory switch every runtime reads.
#[test]
fn a_policy_patch_can_turn_memory_off_for_every_runtime() {
    let fixture = fixture("management-memory-toggle");
    mark_ready(&fixture);
    fixture
        .api
        .patch_policy(
            &PersonalizationPolicyScope::Global,
            None,
            PersonalizationPolicyPatch {
                memory_read_mode: Some(PolicyToggle::Enabled),
                ..PersonalizationPolicyPatch::default()
            },
        )
        .expect("seed global");
    let revision = fixture.api.legacy_settings().expect("settings").revision;

    fixture
        .api
        .patch_policy(
            &PersonalizationPolicyScope::Global,
            Some(revision),
            PersonalizationPolicyPatch {
                memory_read_mode: Some(PolicyToggle::Disabled),
                ..PersonalizationPolicyPatch::default()
            },
        )
        .expect("disable");

    assert_eq!(
        fixture
            .api
            .legacy_settings()
            .expect("settings")
            .settings
            .memory_enabled,
        Some(false)
    );
}
