use super::policy::{
    InstructionMergeMode, PersonalizationPolicyPatch, PersonalizationPolicyRecord, PolicyToggle,
    SessionPersonalizationMode,
};
use super::resolution::{resolve, MaintenanceState, PersonalizationLayers};
use super::scope::{
    AgentId, AgentRuntimeKind, PersonalizationPolicyScope, SessionId, WorkspaceIdentity,
    WorkspaceKey, WorkspaceKind,
};
use super::snapshot::{
    EffectivePersonalizationSnapshot, PersonalizationResolutionContext,
    PersonalizationRuntimeCapabilities, PersonalizationWarningCode, FAIL_CLOSED_REVISION_TOKEN,
};

fn agent() -> AgentId {
    AgentId::parse("claude-code").expect("agent")
}

fn workspace_key() -> WorkspaceKey {
    WorkspaceKey::parse("ws_1").expect("workspace")
}

fn full_capabilities() -> PersonalizationRuntimeCapabilities {
    PersonalizationRuntimeCapabilities {
        supports_custom_instructions: true,
        supports_memory_index: true,
        supports_selected_memory_bodies: true,
        supports_automatic_extraction: true,
    }
}

fn healthy() -> MaintenanceState {
    MaintenanceState {
        migration_generation: 2,
        migration_complete: true,
        repair_required: false,
    }
}

fn context(
    mode: SessionPersonalizationMode,
    with_workspace: bool,
) -> PersonalizationResolutionContext {
    PersonalizationResolutionContext {
        agent_id: agent(),
        session_id: SessionId::parse("ses_1").expect("session"),
        workspace: with_workspace.then(|| {
            WorkspaceIdentity::new(
                workspace_key(),
                "D:/work/app".to_string(),
                WorkspaceKind::Local,
            )
        }),
        runtime_kind: AgentRuntimeKind::Cli,
        session_mode: mode,
    }
}

fn global_with(about: &str, style: &str) -> PersonalizationPolicyRecord {
    let mut record = PersonalizationPolicyRecord::default_global();
    record.set_about_user(about.to_string());
    record.set_style_rules(style.to_string());
    record
}

fn override_at(
    scope: PersonalizationPolicyScope,
    mode: InstructionMergeMode,
    about: &str,
) -> PersonalizationPolicyRecord {
    let mut record = PersonalizationPolicyRecord::inheriting(scope);
    record.set_instruction_merge_mode(mode);
    record.set_about_user(about.to_string());
    record
}

fn layers_with_global(global: PersonalizationPolicyRecord) -> PersonalizationLayers {
    PersonalizationLayers {
        global: Some(global),
        ..PersonalizationLayers::default()
    }
}

fn snapshot(
    layers: PersonalizationLayers,
    mode: SessionPersonalizationMode,
) -> EffectivePersonalizationSnapshot {
    resolve(context(mode, true), layers, full_capabilities(), healthy())
}

#[test]
fn a_missing_global_policy_fails_closed_rather_than_falling_open() {
    // "No validated policy" must never resolve to "memory enabled". This is the one direction the
    // fallback is not allowed to guess in.
    let result = resolve(
        context(SessionPersonalizationMode::Standard, true),
        PersonalizationLayers::default(),
        full_capabilities(),
        healthy(),
    );
    assert_eq!(result.revision_token, FAIL_CLOSED_REVISION_TOKEN);
    assert!(!result.has_user_instructions());
    assert!(result.memory_access.denies_everything());
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.code == PersonalizationWarningCode::NoValidatedPolicy));
}

#[test]
fn the_global_layer_alone_supplies_instructions_and_memory_access() {
    let result = snapshot(
        layers_with_global(global_with("I use Rust", "Be terse")),
        SessionPersonalizationMode::Standard,
    );
    assert_eq!(result.instruction_segments.len(), 1);
    assert_eq!(result.instruction_segments[0].scope_kind, "global");
    assert_eq!(result.instruction_segments[0].about_user, "I use Rust");
    assert_eq!(result.instruction_segments[0].style_rules, "Be terse");
    assert!(result.memory_access.read);
    assert!(result.memory_access.explicit_save);
    assert!(result.memory_access.automatic_extraction);
    assert!(result.memory_access.global_memory);
    assert_eq!(result.memory_access.workspace, Some(workspace_key()));
}

#[test]
fn an_empty_global_row_contributes_no_segment() {
    // A segment with two empty fields would render an empty personalization section.
    let result = snapshot(
        layers_with_global(PersonalizationPolicyRecord::default_global()),
        SessionPersonalizationMode::Standard,
    );
    assert!(result.instruction_segments.is_empty());
    assert!(
        result.memory_access.read,
        "empty text must not disable memory"
    );
}

#[test]
fn append_keeps_inherited_segments_and_adds_its_own_after_them() {
    let mut layers = layers_with_global(global_with("global about", ""));
    layers.agent = Some(override_at(
        PersonalizationPolicyScope::Agent { agent_id: agent() },
        InstructionMergeMode::Append,
        "agent about",
    ));
    let result = snapshot(layers, SessionPersonalizationMode::Standard);

    let rendered: Vec<&str> = result
        .instruction_segments
        .iter()
        .map(|segment| segment.about_user.as_str())
        .collect();
    assert_eq!(rendered, vec!["global about", "agent about"]);
    assert_eq!(
        result.effective_instruction_mode,
        InstructionMergeMode::Append
    );
}

#[test]
fn replace_discards_lower_precedence_user_segments() {
    let mut layers = layers_with_global(global_with("global about", "global style"));
    layers.workspace = Some(override_at(
        PersonalizationPolicyScope::Workspace {
            workspace_key: workspace_key(),
        },
        InstructionMergeMode::Replace,
        "workspace about",
    ));
    let result = snapshot(layers, SessionPersonalizationMode::Standard);

    assert_eq!(result.instruction_segments.len(), 1);
    assert_eq!(result.instruction_segments[0].about_user, "workspace about");
    assert_eq!(result.instruction_segments[0].scope_kind, "workspace");
    assert_eq!(
        result.effective_instruction_mode,
        InstructionMergeMode::Replace
    );
}

#[test]
fn a_later_append_survives_an_earlier_replace() {
    // Precedence is "later layers override earlier layers", so a replace at the workspace layer
    // clears what came before it but cannot bind a workspace-Agent layer that appends after it.
    let mut layers = layers_with_global(global_with("global about", ""));
    layers.workspace = Some(override_at(
        PersonalizationPolicyScope::Workspace {
            workspace_key: workspace_key(),
        },
        InstructionMergeMode::Replace,
        "workspace about",
    ));
    layers.workspace_agent = Some(override_at(
        PersonalizationPolicyScope::WorkspaceAgent {
            workspace_key: workspace_key(),
            agent_id: agent(),
        },
        InstructionMergeMode::Append,
        "seat about",
    ));
    let result = snapshot(layers, SessionPersonalizationMode::Standard);

    let rendered: Vec<&str> = result
        .instruction_segments
        .iter()
        .map(|segment| segment.about_user.as_str())
        .collect();
    assert_eq!(rendered, vec!["workspace about", "seat about"]);
}

#[test]
fn disabled_omits_every_user_segment() {
    let mut layers = layers_with_global(global_with("global about", "global style"));
    layers.agent = Some(override_at(
        PersonalizationPolicyScope::Agent { agent_id: agent() },
        InstructionMergeMode::Disabled,
        "ignored",
    ));
    let result = snapshot(layers, SessionPersonalizationMode::Standard);

    assert!(result.instruction_segments.is_empty());
    assert_eq!(
        result.effective_instruction_mode,
        InstructionMergeMode::Disabled
    );
    // Disabling instructions says nothing about memory.
    assert!(result.memory_access.read);
}

#[test]
fn an_inheriting_layer_changes_nothing() {
    let mut layers = layers_with_global(global_with("global about", ""));
    layers.agent = Some(PersonalizationPolicyRecord::inheriting(
        PersonalizationPolicyScope::Agent { agent_id: agent() },
    ));
    let result = snapshot(layers, SessionPersonalizationMode::Standard);

    assert_eq!(result.instruction_segments.len(), 1);
    assert_eq!(result.instruction_segments[0].scope_kind, "global");
    assert!(result.memory_access.read);
}

#[test]
fn a_workspace_override_outranks_a_generic_agent_override() {
    // Deliberate: project guidance beats a broad per-Agent preference. A workspace-Agent row is
    // the way to make one Agent an exception inside one project.
    let mut layers = layers_with_global(global_with("", ""));
    let mut agent_layer =
        PersonalizationPolicyRecord::inheriting(PersonalizationPolicyScope::Agent {
            agent_id: agent(),
        });
    agent_layer.set_memory_read_mode(PolicyToggle::Disabled);
    let mut workspace_layer =
        PersonalizationPolicyRecord::inheriting(PersonalizationPolicyScope::Workspace {
            workspace_key: workspace_key(),
        });
    workspace_layer.set_memory_read_mode(PolicyToggle::Enabled);
    layers.agent = Some(agent_layer);
    layers.workspace = Some(workspace_layer);

    let result = snapshot(layers, SessionPersonalizationMode::Standard);
    assert!(result.memory_access.read, "workspace must win over Agent");
}

#[test]
fn a_workspace_agent_row_is_the_exception_that_wins_inside_one_workspace() {
    let mut layers = layers_with_global(global_with("", ""));
    let mut workspace_layer =
        PersonalizationPolicyRecord::inheriting(PersonalizationPolicyScope::Workspace {
            workspace_key: workspace_key(),
        });
    workspace_layer.set_memory_read_mode(PolicyToggle::Enabled);
    let mut seat_layer =
        PersonalizationPolicyRecord::inheriting(PersonalizationPolicyScope::WorkspaceAgent {
            workspace_key: workspace_key(),
            agent_id: agent(),
        });
    seat_layer.set_memory_read_mode(PolicyToggle::Disabled);
    layers.workspace = Some(workspace_layer);
    layers.workspace_agent = Some(seat_layer);

    let result = snapshot(layers, SessionPersonalizationMode::Standard);
    assert!(!result.memory_access.read);
}

#[test]
fn a_session_override_is_applied_after_every_durable_layer() {
    let mut layers = layers_with_global(global_with("", ""));
    let mut seat_layer =
        PersonalizationPolicyRecord::inheriting(PersonalizationPolicyScope::WorkspaceAgent {
            workspace_key: workspace_key(),
            agent_id: agent(),
        });
    seat_layer.set_automatic_extraction_mode(PolicyToggle::Enabled);
    layers.workspace_agent = Some(seat_layer);
    layers.session_override = Some(PersonalizationPolicyPatch {
        automatic_extraction_mode: Some(PolicyToggle::Disabled),
        ..PersonalizationPolicyPatch::default()
    });

    let result = snapshot(layers, SessionPersonalizationMode::Standard);
    assert!(!result.memory_access.automatic_extraction);
}

#[test]
fn temporary_mode_denies_every_memory_dimension_but_keeps_instructions() {
    // Language and response-style preferences are exactly what a user still wants in a throwaway
    // session; long-term memory is exactly what they do not.
    let result = snapshot(
        layers_with_global(global_with("I use Rust", "Be terse")),
        SessionPersonalizationMode::Temporary,
    );
    assert!(result.has_user_instructions());
    assert!(result.memory_access.denies_everything());
    assert_eq!(result.memory_access.workspace, None);
}

#[test]
fn no_policy_override_can_reopen_memory_in_a_temporary_session() {
    // Session mode is a hard restriction applied last, not another layer that a higher-precedence
    // row can outrank.
    let mut layers = layers_with_global(global_with("", ""));
    let mut seat_layer =
        PersonalizationPolicyRecord::inheriting(PersonalizationPolicyScope::WorkspaceAgent {
            workspace_key: workspace_key(),
            agent_id: agent(),
        });
    seat_layer.set_memory_read_mode(PolicyToggle::Enabled);
    seat_layer.set_explicit_save_mode(PolicyToggle::Enabled);
    seat_layer.set_automatic_extraction_mode(PolicyToggle::Enabled);
    seat_layer.set_global_memory_access_mode(PolicyToggle::Enabled);
    layers.workspace_agent = Some(seat_layer);
    layers.session_override = Some(PersonalizationPolicyPatch {
        memory_read_mode: Some(PolicyToggle::Enabled),
        ..PersonalizationPolicyPatch::default()
    });

    let result = snapshot(layers, SessionPersonalizationMode::Temporary);
    assert!(result.memory_access.denies_everything());
}

#[test]
fn project_only_mode_excludes_global_memory_and_pins_the_workspace() {
    let result = snapshot(
        layers_with_global(global_with("keep me", "")),
        SessionPersonalizationMode::ProjectOnly,
    );
    assert!(result.has_user_instructions(), "instructions still apply");
    assert!(result.memory_access.read);
    assert!(!result.memory_access.global_memory);
    assert_eq!(result.memory_access.workspace, Some(workspace_key()));
    assert!(result.exclusions.iter().any(|exclusion| exclusion.reason
        == super::snapshot::PersonalizationExclusionReason::ProjectOnlySession));
}

#[test]
fn project_only_without_a_workspace_denies_memory_rather_than_reading_globally() {
    // Creation is supposed to reject this, so reaching resolution means something upstream failed.
    // Degrading to "read everything global" would be the worst possible interpretation.
    let result = resolve(
        context(SessionPersonalizationMode::ProjectOnly, false),
        layers_with_global(global_with("", "")),
        full_capabilities(),
        healthy(),
    );
    assert!(result.memory_access.denies_everything());
}

#[test]
fn a_runtime_without_extraction_capability_never_extracts() {
    let capabilities = PersonalizationRuntimeCapabilities {
        supports_automatic_extraction: false,
        ..full_capabilities()
    };
    let result = resolve(
        context(SessionPersonalizationMode::Standard, true),
        layers_with_global(global_with("", "")),
        capabilities,
        healthy(),
    );
    assert!(!result.memory_access.automatic_extraction);
    assert!(result.memory_access.read, "index injection is unaffected");
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.code == PersonalizationWarningCode::UnsupportedCapabilityOverride));
}

#[test]
fn a_runtime_without_instruction_support_receives_no_segments() {
    let capabilities = PersonalizationRuntimeCapabilities {
        supports_custom_instructions: false,
        ..full_capabilities()
    };
    let result = resolve(
        context(SessionPersonalizationMode::Standard, true),
        layers_with_global(global_with("I use Rust", "")),
        capabilities,
        healthy(),
    );
    assert!(!result.has_user_instructions());
}

#[test]
fn a_runtime_without_memory_index_support_reads_no_memory() {
    let capabilities = PersonalizationRuntimeCapabilities {
        supports_memory_index: false,
        ..full_capabilities()
    };
    let result = resolve(
        context(SessionPersonalizationMode::Standard, true),
        layers_with_global(global_with("", "")),
        capabilities,
        healthy(),
    );
    assert!(!result.memory_access.read);
}

#[test]
fn incomplete_migration_keeps_memory_unavailable_without_touching_instructions() {
    let maintenance = MaintenanceState {
        migration_complete: false,
        ..healthy()
    };
    let result = resolve(
        context(SessionPersonalizationMode::Standard, true),
        layers_with_global(global_with("keep me", "")),
        full_capabilities(),
        maintenance,
    );
    assert!(result.has_user_instructions());
    assert!(result.memory_access.denies_everything());
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.code == PersonalizationWarningCode::MigrationIncomplete));
}

#[test]
fn a_repair_required_state_warns_without_denying_memory_wholesale() {
    // Repair is per-record: one unprojected memory must not blind the whole session.
    let maintenance = MaintenanceState {
        repair_required: true,
        ..healthy()
    };
    let result = resolve(
        context(SessionPersonalizationMode::Standard, true),
        layers_with_global(global_with("", "")),
        full_capabilities(),
        maintenance,
    );
    assert!(result.memory_access.read);
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.code == PersonalizationWarningCode::RepairRequired));
}

#[test]
fn the_revision_token_is_deterministic_for_the_same_inputs() {
    let build = || {
        snapshot(
            layers_with_global(global_with("a", "b")),
            SessionPersonalizationMode::Standard,
        )
    };
    assert_eq!(build().revision_token, build().revision_token);
}

#[test]
fn the_revision_token_does_not_hash_instruction_content() {
    // The token travels into diagnostics and logs. Two rows at the same revision that differ only
    // in text must be indistinguishable through it.
    let left = snapshot(
        layers_with_global(global_with("secret plan alpha", "")),
        SessionPersonalizationMode::Standard,
    );
    let right = snapshot(
        layers_with_global(global_with("completely different text", "")),
        SessionPersonalizationMode::Standard,
    );
    assert_eq!(left.revision_token, right.revision_token);
}

#[test]
fn the_revision_token_changes_with_revision_session_mode_and_workspace() {
    let base = snapshot(
        layers_with_global(global_with("a", "")),
        SessionPersonalizationMode::Standard,
    );

    let mut bumped_record = global_with("a", "");
    bumped_record.set_revision(9);
    let bumped = snapshot(
        layers_with_global(bumped_record),
        SessionPersonalizationMode::Standard,
    );
    assert_ne!(base.revision_token, bumped.revision_token);

    let other_mode = snapshot(
        layers_with_global(global_with("a", "")),
        SessionPersonalizationMode::ProjectOnly,
    );
    assert_ne!(base.revision_token, other_mode.revision_token);

    let mut other_workspace_context = context(SessionPersonalizationMode::Standard, true);
    other_workspace_context.workspace = Some(WorkspaceIdentity::new(
        WorkspaceKey::parse("ws_other").expect("workspace"),
        "D:/work/app".to_string(),
        WorkspaceKind::Local,
    ));
    let other_workspace = resolve(
        other_workspace_context,
        layers_with_global(global_with("a", "")),
        full_capabilities(),
        healthy(),
    );
    assert_ne!(base.revision_token, other_workspace.revision_token);

    let other_generation = resolve(
        context(SessionPersonalizationMode::Standard, true),
        layers_with_global(global_with("a", "")),
        full_capabilities(),
        MaintenanceState {
            migration_generation: 3,
            ..healthy()
        },
    );
    assert_ne!(base.revision_token, other_generation.revision_token);
}

#[test]
fn the_revision_token_separates_two_seats_in_the_same_session() {
    // A multi-Agent seat resolves with its own stable Agent id; two seats must not share a token
    // that diagnostics would then treat as one resolution.
    let base = snapshot(
        layers_with_global(global_with("a", "")),
        SessionPersonalizationMode::Standard,
    );
    let mut other_seat = context(SessionPersonalizationMode::Standard, true);
    other_seat.agent_id = AgentId::parse("codex-cli").expect("agent");
    let other = resolve(
        other_seat,
        layers_with_global(global_with("a", "")),
        full_capabilities(),
        healthy(),
    );
    assert_ne!(base.revision_token, other.revision_token);
}
