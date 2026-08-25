use chrono::{TimeZone, Utc};

use super::memory::{
    eligibility, MemoryAudience, MemoryId, MemoryProvenance, MemoryRecord, MemoryScope,
    MemorySensitivity, MemorySource, MemoryStatus, MemoryType, MEMORY_AUDIENCE_MAX_AGENTS,
    MEMORY_CONTENT_MAX_CHARS, MEMORY_DESCRIPTION_MAX_CHARS, MEMORY_NAME_MAX_CHARS,
};
use super::scope::{AgentId, WorkspaceKey};
use super::snapshot::{EffectiveMemoryAccess, MemoryDeliveryMode, PersonalizationExclusionReason};
use super::PersonalizationDomainError;

fn agent(id: &str) -> AgentId {
    AgentId::parse(id).expect("agent")
}

fn workspace() -> WorkspaceKey {
    WorkspaceKey::parse("ws_1").expect("workspace")
}

fn memory_id() -> MemoryId {
    MemoryId::parse("01K2ABCDEFGHJKMNPQRSTVWXYZ").expect("memory id")
}

fn record(scope: MemoryScope, status: MemoryStatus, audience: MemoryAudience) -> MemoryRecord {
    MemoryRecord {
        id: memory_id(),
        name: "Use npm".to_string(),
        description: "Package-manager preference".to_string(),
        memory_type: MemoryType::Project,
        content: "Use npm for this repository.".to_string(),
        scope,
        audience,
        status,
        source: MemorySource::ExplicitUser,
        provenance: MemoryProvenance::default(),
        sensitivity: MemorySensitivity::Normal,
        revision: 1,
        created_at: Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2026, 8, 1, 0, 0, 0).unwrap(),
        verified_at: None,
        last_used_at: None,
        use_count: 0,
    }
}

fn full_access() -> EffectiveMemoryAccess {
    EffectiveMemoryAccess {
        read: true,
        explicit_save: true,
        automatic_extraction: true,
        global_memory: true,
        workspace: Some(workspace()),
        candidate_creation: true,
        retrieval_write: true,
        delivery: MemoryDeliveryMode::IndexWithSelectedBodies,
        block_reason: None,
    }
}

#[test]
fn memory_ids_are_generated_tokens_not_user_text() {
    // Filenames are derived from this, so anything that could traverse or collide across
    // case-insensitive filesystems has to be impossible by construction rather than by escaping.
    assert!(MemoryId::parse("01K2ABCDEFGHJKMNPQRSTVWXYZ").is_ok());
    assert!(MemoryId::parse("6f5e4d3c-2b1a-4098-8765-0f1e2d3c4b5a").is_ok());
    for rejected in [
        "",
        "..",
        "../escape",
        "has space",
        "has/slash",
        "has\\backslash",
        "has.dot",
        "short",
        &"a".repeat(65),
    ] {
        assert!(
            matches!(
                MemoryId::parse(rejected),
                Err(PersonalizationDomainError::InvalidMemoryId(_))
            ),
            "{rejected:?} must be rejected as a memory id"
        );
    }
}

#[test]
fn a_valid_record_passes_validation() {
    record(
        MemoryScope::Global,
        MemoryStatus::Active,
        MemoryAudience::AllAgents,
    )
    .validate()
    .expect("a well-formed record is valid");
}

#[test]
fn memory_field_limits_are_enforced_in_characters() {
    let mut subject = record(
        MemoryScope::Global,
        MemoryStatus::Active,
        MemoryAudience::AllAgents,
    );

    subject.name = String::new();
    assert!(matches!(
        subject.validate(),
        Err(PersonalizationDomainError::MemoryFieldEmpty { field: "name" })
    ));

    subject.name = "策".repeat(MEMORY_NAME_MAX_CHARS);
    subject.validate().expect("120 CJK characters must fit");

    subject.name = "x".repeat(MEMORY_NAME_MAX_CHARS + 1);
    assert!(matches!(
        subject.validate(),
        Err(PersonalizationDomainError::MemoryFieldTooLong { field: "name", .. })
    ));

    let mut subject = record(
        MemoryScope::Global,
        MemoryStatus::Active,
        MemoryAudience::AllAgents,
    );
    subject.description = String::new();
    subject.validate().expect("an empty description is allowed");
    subject.description = "x".repeat(MEMORY_DESCRIPTION_MAX_CHARS + 1);
    assert!(matches!(
        subject.validate(),
        Err(PersonalizationDomainError::MemoryFieldTooLong {
            field: "description",
            ..
        })
    ));

    let mut subject = record(
        MemoryScope::Global,
        MemoryStatus::Active,
        MemoryAudience::AllAgents,
    );
    subject.content = String::new();
    assert!(matches!(
        subject.validate(),
        Err(PersonalizationDomainError::MemoryFieldEmpty { field: "content" })
    ));
    subject.content = "x".repeat(MEMORY_CONTENT_MAX_CHARS + 1);
    assert!(matches!(
        subject.validate(),
        Err(PersonalizationDomainError::MemoryFieldTooLong {
            field: "content",
            ..
        })
    ));
}

#[test]
fn a_selected_audience_is_bounded_and_never_empty() {
    let mut subject = record(
        MemoryScope::Global,
        MemoryStatus::Active,
        MemoryAudience::SelectedAgents {
            agent_ids: Vec::new(),
        },
    );
    // An empty selected audience would mean "visible to nobody", which no UI action produces and
    // which reads as data loss rather than as a restriction.
    assert!(matches!(
        subject.validate(),
        Err(PersonalizationDomainError::EmptyMemoryAudience)
    ));

    subject.audience = MemoryAudience::SelectedAgents {
        agent_ids: (0..MEMORY_AUDIENCE_MAX_AGENTS)
            .map(|index| agent(&format!("agent-{index}")))
            .collect(),
    };
    subject.validate().expect("exactly at the limit is allowed");

    subject.audience = MemoryAudience::SelectedAgents {
        agent_ids: (0..=MEMORY_AUDIENCE_MAX_AGENTS)
            .map(|index| agent(&format!("agent-{index}")))
            .collect(),
    };
    assert!(matches!(
        subject.validate(),
        Err(PersonalizationDomainError::MemoryAudienceTooLarge { .. })
    ));
}

#[test]
fn a_new_record_must_declare_a_recognized_type() {
    // Legacy rows migrate as explicitly untyped; a new create may not.
    let mut subject = record(
        MemoryScope::Global,
        MemoryStatus::Active,
        MemoryAudience::AllAgents,
    );
    subject.memory_type = MemoryType::Untyped;
    subject.source = MemorySource::ExplicitUser;
    assert!(matches!(
        subject.validate(),
        Err(PersonalizationDomainError::UntypedMemoryRequiresLegacySource)
    ));

    subject.source = MemorySource::LegacyMigration;
    subject
        .validate()
        .expect("a migrated legacy record may stay untyped");
}

#[test]
fn an_active_global_memory_is_eligible_when_global_access_is_allowed() {
    let subject = record(
        MemoryScope::Global,
        MemoryStatus::Active,
        MemoryAudience::AllAgents,
    );
    assert_eq!(
        eligibility(&subject, &full_access(), &agent("claude-code")),
        Ok(())
    );
}

#[test]
fn a_candidate_is_never_eligible() {
    // Candidates must not reach a prompt, `MEMORY.md`, or the retrieval index before approval.
    let subject = record(
        MemoryScope::Global,
        MemoryStatus::Candidate,
        MemoryAudience::AllAgents,
    );
    assert_eq!(
        eligibility(&subject, &full_access(), &agent("claude-code")),
        Err(PersonalizationExclusionReason::PendingCandidate)
    );
}

#[test]
fn an_archived_memory_is_never_eligible() {
    let subject = record(
        MemoryScope::Global,
        MemoryStatus::Archived,
        MemoryAudience::AllAgents,
    );
    assert_eq!(
        eligibility(&subject, &full_access(), &agent("claude-code")),
        Err(PersonalizationExclusionReason::Archived)
    );
}

#[test]
fn disabled_read_excludes_everything_before_any_scope_check() {
    let subject = record(
        MemoryScope::Global,
        MemoryStatus::Active,
        MemoryAudience::AllAgents,
    );
    let access = EffectiveMemoryAccess {
        read: false,
        ..full_access()
    };
    assert_eq!(
        eligibility(&subject, &access, &agent("claude-code")),
        Err(PersonalizationExclusionReason::MemoryReadDisabled)
    );
}

#[test]
fn a_global_memory_is_excluded_when_global_access_is_disabled() {
    // This is the shape project-only mode produces, and the shape a workspace override produces.
    let subject = record(
        MemoryScope::Global,
        MemoryStatus::Active,
        MemoryAudience::AllAgents,
    );
    let access = EffectiveMemoryAccess {
        global_memory: false,
        ..full_access()
    };
    assert_eq!(
        eligibility(&subject, &access, &agent("claude-code")),
        Err(PersonalizationExclusionReason::GlobalMemoryDisabled)
    );
}

#[test]
fn a_workspace_memory_is_excluded_from_a_different_workspace() {
    let subject = record(
        MemoryScope::Workspace {
            workspace_key: WorkspaceKey::parse("ws_other").expect("workspace"),
        },
        MemoryStatus::Active,
        MemoryAudience::AllAgents,
    );
    assert_eq!(
        eligibility(&subject, &full_access(), &agent("claude-code")),
        Err(PersonalizationExclusionReason::OtherWorkspace)
    );
}

#[test]
fn a_workspace_memory_is_excluded_when_no_workspace_is_active() {
    let subject = record(
        MemoryScope::Workspace {
            workspace_key: workspace(),
        },
        MemoryStatus::Active,
        MemoryAudience::AllAgents,
    );
    let access = EffectiveMemoryAccess {
        workspace: None,
        ..full_access()
    };
    assert_eq!(
        eligibility(&subject, &access, &agent("claude-code")),
        Err(PersonalizationExclusionReason::OtherWorkspace)
    );
}

#[test]
fn a_matching_workspace_memory_stays_eligible_even_when_global_access_is_disabled() {
    let subject = record(
        MemoryScope::Workspace {
            workspace_key: workspace(),
        },
        MemoryStatus::Active,
        MemoryAudience::AllAgents,
    );
    let access = EffectiveMemoryAccess {
        global_memory: false,
        ..full_access()
    };
    assert_eq!(
        eligibility(&subject, &access, &agent("claude-code")),
        Ok(())
    );
}

#[test]
fn a_selected_audience_admits_only_the_listed_agents() {
    let subject = record(
        MemoryScope::Global,
        MemoryStatus::Active,
        MemoryAudience::SelectedAgents {
            agent_ids: vec![agent("codex-cli")],
        },
    );
    assert_eq!(
        eligibility(&subject, &full_access(), &agent("codex-cli")),
        Ok(())
    );
    assert_eq!(
        eligibility(&subject, &full_access(), &agent("claude-code")),
        Err(PersonalizationExclusionReason::AgentAudience)
    );
}

#[test]
fn the_producing_agent_gains_no_access_from_provenance_alone() {
    // Provenance is not authorization. The Agent that created a memory is excluded like any other
    // when the audience does not list it.
    let mut subject = record(
        MemoryScope::Global,
        MemoryStatus::Active,
        MemoryAudience::SelectedAgents {
            agent_ids: vec![agent("codex-cli")],
        },
    );
    subject.provenance = MemoryProvenance {
        source_agent_id: Some(agent("claude-code")),
        ..MemoryProvenance::default()
    };
    assert_eq!(
        eligibility(&subject, &full_access(), &agent("claude-code")),
        Err(PersonalizationExclusionReason::AgentAudience)
    );
}

#[test]
fn memory_enums_round_trip_through_their_persisted_strings() {
    for (value, text) in [
        (MemoryStatus::Candidate, "candidate"),
        (MemoryStatus::Active, "active"),
        (MemoryStatus::Archived, "archived"),
    ] {
        assert_eq!(value.as_str(), text);
        assert_eq!(MemoryStatus::parse(text), Ok(value));
    }
    for (value, text) in [
        (MemorySource::ExplicitUser, "explicit_user"),
        (MemorySource::OnePieceAutomatic, "onepiece_automatic"),
        (MemorySource::CliAutomatic, "cli_automatic"),
        (MemorySource::ModelMemoryTool, "model_memory_tool"),
        (MemorySource::LegacyMigration, "legacy_migration"),
        (MemorySource::ExternalFileEdit, "external_file_edit"),
    ] {
        assert_eq!(value.as_str(), text);
        assert_eq!(MemorySource::parse(text), Ok(value));
    }
    for (value, text) in [
        (MemoryType::User, "user"),
        (MemoryType::Feedback, "feedback"),
        (MemoryType::Project, "project"),
        (MemoryType::Reference, "reference"),
        (MemoryType::Untyped, "untyped"),
    ] {
        assert_eq!(value.as_str(), text);
        assert_eq!(MemoryType::parse(text), Ok(value));
    }
    for (value, text) in [
        (MemorySensitivity::Normal, "normal"),
        (MemorySensitivity::Sensitive, "sensitive"),
    ] {
        assert_eq!(value.as_str(), text);
        assert_eq!(MemorySensitivity::parse(text), Ok(value));
    }
    assert!(matches!(
        MemoryStatus::parse("pending"),
        Err(PersonalizationDomainError::UnknownMemoryStatus(_))
    ));
    assert!(matches!(
        MemoryType::parse("preference"),
        Err(PersonalizationDomainError::UnknownMemoryType(_))
    ));
}

#[test]
fn a_memory_filename_is_derived_only_from_the_immutable_id() {
    let subject = record(
        MemoryScope::Global,
        MemoryStatus::Active,
        MemoryAudience::AllAgents,
    );
    assert_eq!(subject.file_name(), "01K2ABCDEFGHJKMNPQRSTVWXYZ.md");

    // Renaming must not move the file: identity and display name are independent.
    let mut renamed = subject.clone();
    renamed.name = "Something else entirely".to_string();
    assert_eq!(renamed.file_name(), subject.file_name());
}

#[test]
fn two_records_may_share_a_display_name_without_sharing_a_file() {
    let left = record(
        MemoryScope::Global,
        MemoryStatus::Active,
        MemoryAudience::AllAgents,
    );
    let mut right = left.clone();
    right.id = MemoryId::parse("01K3ZZZZZZZZZZZZZZZZZZZZZZ").expect("memory id");
    assert_eq!(left.name, right.name);
    assert_ne!(left.file_name(), right.file_name());
}
