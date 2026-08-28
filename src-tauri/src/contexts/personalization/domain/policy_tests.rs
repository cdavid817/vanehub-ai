use super::policy::{
    InstructionMergeMode, PersonalizationPolicyPatch, PersonalizationPolicyRecord, PolicyToggle,
    RevisionConflict, SessionPersonalizationMode, INSTRUCTION_FIELD_MAX_CHARS,
};
use super::scope::{AgentId, PersonalizationPolicyScope, WorkspaceKey};
use super::PersonalizationDomainError;

fn agent_scope() -> PersonalizationPolicyScope {
    PersonalizationPolicyScope::Agent {
        agent_id: AgentId::parse("claude-code").expect("agent"),
    }
}

fn workspace_scope() -> PersonalizationPolicyScope {
    PersonalizationPolicyScope::Workspace {
        workspace_key: WorkspaceKey::parse("ws_1").expect("workspace"),
    }
}

#[test]
fn the_default_global_policy_preserves_pre_change_behavior() {
    // Migration maps an existing installation's toggles onto this row, and a fresh installation
    // starts from it. Both must keep behaving the way the product did before this change, which
    // means enabled — fail-closed is the *fallback* when no policy can be read, not the default.
    let record = PersonalizationPolicyRecord::default_global();
    assert_eq!(record.scope(), &PersonalizationPolicyScope::Global);
    assert_eq!(
        record.instruction_merge_mode(),
        InstructionMergeMode::Append
    );
    assert_eq!(record.memory_read_mode(), PolicyToggle::Enabled);
    assert_eq!(record.explicit_save_mode(), PolicyToggle::Enabled);
    assert_eq!(record.automatic_extraction_mode(), PolicyToggle::Enabled);
    assert_eq!(record.global_memory_access_mode(), PolicyToggle::Enabled);
    assert!(record.about_user().is_empty());
    assert!(record.style_rules().is_empty());
    assert_eq!(record.revision(), 0);
    record
        .validate()
        .expect("the default global row must be valid");
}

#[test]
fn a_global_row_may_not_store_inherit() {
    // There is nothing below global to inherit from. Allowing it would make the resolved value
    // depend on the built-in fallback in a way the UI could not explain.
    let mut record = PersonalizationPolicyRecord::default_global();
    record.set_instruction_merge_mode(InstructionMergeMode::Inherit);
    assert!(matches!(
        record.validate(),
        Err(PersonalizationDomainError::GlobalScopeCannotInherit { .. })
    ));

    for setter in [
        PersonalizationPolicyRecord::set_memory_read_mode,
        PersonalizationPolicyRecord::set_explicit_save_mode,
        PersonalizationPolicyRecord::set_automatic_extraction_mode,
        PersonalizationPolicyRecord::set_global_memory_access_mode,
    ] {
        let mut record = PersonalizationPolicyRecord::default_global();
        setter(&mut record, PolicyToggle::Inherit);
        assert!(matches!(
            record.validate(),
            Err(PersonalizationDomainError::GlobalScopeCannotInherit { .. })
        ));
    }
}

#[test]
fn a_non_global_row_defaults_to_inheriting_everything() {
    // A freshly created override must change nothing until the user actually sets something.
    let record = PersonalizationPolicyRecord::inheriting(agent_scope());
    assert_eq!(
        record.instruction_merge_mode(),
        InstructionMergeMode::Inherit
    );
    assert_eq!(record.memory_read_mode(), PolicyToggle::Inherit);
    assert_eq!(record.explicit_save_mode(), PolicyToggle::Inherit);
    assert_eq!(record.automatic_extraction_mode(), PolicyToggle::Inherit);
    assert_eq!(record.global_memory_access_mode(), PolicyToggle::Inherit);
    record.validate().expect("an all-inherit override is valid");
}

#[test]
fn instruction_fields_are_bounded_at_three_thousand_characters() {
    assert_eq!(INSTRUCTION_FIELD_MAX_CHARS, 3_000);
    let mut record = PersonalizationPolicyRecord::inheriting(workspace_scope());

    record.set_about_user("x".repeat(INSTRUCTION_FIELD_MAX_CHARS));
    record.validate().expect("exactly at the limit is allowed");

    record.set_about_user("x".repeat(INSTRUCTION_FIELD_MAX_CHARS + 1));
    assert!(matches!(
        record.validate(),
        Err(PersonalizationDomainError::InstructionFieldTooLong {
            field: "about_user",
            ..
        })
    ));

    let mut record = PersonalizationPolicyRecord::inheriting(workspace_scope());
    record.set_style_rules("x".repeat(INSTRUCTION_FIELD_MAX_CHARS + 1));
    assert!(matches!(
        record.validate(),
        Err(PersonalizationDomainError::InstructionFieldTooLong {
            field: "style_rules",
            ..
        })
    ));
}

#[test]
fn the_instruction_bound_counts_characters_not_bytes() {
    // A user writing Chinese would otherwise hit roughly a third of the advertised limit.
    let mut record = PersonalizationPolicyRecord::inheriting(workspace_scope());
    let multibyte = "策".repeat(INSTRUCTION_FIELD_MAX_CHARS);
    assert!(
        multibyte.len() > INSTRUCTION_FIELD_MAX_CHARS,
        "precondition"
    );
    record.set_about_user(multibyte);
    record
        .validate()
        .expect("3,000 multibyte characters must be accepted");
}

#[test]
fn a_patch_leaves_unmentioned_fields_alone() {
    // The whole point of moving off whole-`AppSettings` saves: writing one field must not
    // republish the others.
    let mut record = PersonalizationPolicyRecord::inheriting(agent_scope());
    record.set_about_user("keep me".to_string());
    record.set_memory_read_mode(PolicyToggle::Disabled);

    let patch = PersonalizationPolicyPatch {
        style_rules: Some("be terse".to_string()),
        ..PersonalizationPolicyPatch::default()
    };
    let updated = record.apply(patch).expect("patch applies");

    assert_eq!(updated.style_rules(), "be terse");
    assert_eq!(updated.about_user(), "keep me");
    assert_eq!(updated.memory_read_mode(), PolicyToggle::Disabled);
}

#[test]
fn applying_a_patch_advances_the_revision_exactly_once() {
    let record = PersonalizationPolicyRecord::inheriting(agent_scope());
    let before = record.revision();
    let updated = record
        .apply(PersonalizationPolicyPatch {
            about_user: Some("hello".to_string()),
            ..PersonalizationPolicyPatch::default()
        })
        .expect("patch applies");
    assert_eq!(updated.revision(), before + 1);
}

#[test]
fn an_invalid_patch_is_rejected_without_advancing_the_revision() {
    // The native boundary validates independently of the UI, and a rejected write must leave the
    // persisted revision untouched or the client's next expected-revision check would be wrong.
    let record = PersonalizationPolicyRecord::inheriting(agent_scope());
    let before = record.revision();
    let error = record
        .clone()
        .apply(PersonalizationPolicyPatch {
            about_user: Some("x".repeat(INSTRUCTION_FIELD_MAX_CHARS + 1)),
            ..PersonalizationPolicyPatch::default()
        })
        .expect_err("oversized field must be rejected");
    assert!(matches!(
        error,
        PersonalizationDomainError::InstructionFieldTooLong { .. }
    ));
    assert_eq!(record.revision(), before);
}

#[test]
fn a_stale_expected_revision_is_a_conflict_rather_than_a_write() {
    let mut record = PersonalizationPolicyRecord::inheriting(agent_scope());
    record.set_revision(7);

    assert_eq!(record.check_expected_revision(Some(7)), Ok(()));
    assert_eq!(
        record.check_expected_revision(Some(6)),
        Err(RevisionConflict {
            expected: 6,
            current: 7,
        })
    );
    // A newer-than-current expectation is equally a conflict: it means the caller is reasoning
    // about a revision this store never issued.
    assert_eq!(
        record.check_expected_revision(Some(8)),
        Err(RevisionConflict {
            expected: 8,
            current: 7,
        })
    );
}

#[test]
fn an_absent_expected_revision_is_accepted_for_first_write_paths() {
    // Creating the row for a scope that has none yet has no revision to expect.
    let record = PersonalizationPolicyRecord::inheriting(agent_scope());
    assert_eq!(record.check_expected_revision(None), Ok(()));
}

#[test]
fn policy_toggle_resolves_as_a_tri_state() {
    // Inherit is the only one that reads what came before it; the other two are absolute.
    assert!(PolicyToggle::Inherit.resolve_over(true));
    assert!(!PolicyToggle::Inherit.resolve_over(false));
    assert!(PolicyToggle::Enabled.resolve_over(false));
    assert!(!PolicyToggle::Disabled.resolve_over(true));
}

#[test]
fn session_modes_parse_and_render_their_persisted_values() {
    for (mode, text) in [
        (SessionPersonalizationMode::Standard, "standard"),
        (SessionPersonalizationMode::ProjectOnly, "project-only"),
        (SessionPersonalizationMode::Temporary, "temporary"),
    ] {
        assert_eq!(mode.as_str(), text);
        assert_eq!(SessionPersonalizationMode::parse(text), Ok(mode));
    }
    assert!(matches!(
        SessionPersonalizationMode::parse("clean-room"),
        Err(PersonalizationDomainError::UnknownSessionMode(_))
    ));
}

#[test]
fn an_unspecified_session_mode_is_standard() {
    // Every session persisted before this field existed migrates to standard.
    assert_eq!(
        SessionPersonalizationMode::default(),
        SessionPersonalizationMode::Standard
    );
}

#[test]
fn only_project_only_mode_requires_a_workspace() {
    assert!(SessionPersonalizationMode::ProjectOnly.requires_workspace());
    assert!(!SessionPersonalizationMode::Standard.requires_workspace());
    assert!(!SessionPersonalizationMode::Temporary.requires_workspace());
}

#[test]
fn merge_modes_parse_and_render_their_persisted_values() {
    for (mode, text) in [
        (InstructionMergeMode::Inherit, "inherit"),
        (InstructionMergeMode::Append, "append"),
        (InstructionMergeMode::Replace, "replace"),
        (InstructionMergeMode::Disabled, "disabled"),
    ] {
        assert_eq!(mode.as_str(), text);
        assert_eq!(InstructionMergeMode::parse(text), Ok(mode));
    }
    assert!(matches!(
        InstructionMergeMode::parse("prepend"),
        Err(PersonalizationDomainError::UnknownMergeMode(_))
    ));
}

#[test]
fn policy_toggles_parse_and_render_their_persisted_values() {
    for (toggle, text) in [
        (PolicyToggle::Inherit, "inherit"),
        (PolicyToggle::Enabled, "enabled"),
        (PolicyToggle::Disabled, "disabled"),
    ] {
        assert_eq!(toggle.as_str(), text);
        assert_eq!(PolicyToggle::parse(text), Ok(toggle));
    }
    assert!(matches!(
        PolicyToggle::parse("maybe"),
        Err(PersonalizationDomainError::UnknownPolicyToggle(_))
    ));
}
