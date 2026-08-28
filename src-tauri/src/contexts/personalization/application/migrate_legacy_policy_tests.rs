use super::migrate_legacy_policy::{
    map_legacy_settings, project_to_legacy_settings, LegacyPersonalizationSettings,
    ONEPIECE_AGENT_ID,
};
use crate::contexts::personalization::domain::{
    AgentId, InstructionMergeMode, PersonalizationPolicyScope, PolicyToggle,
};

fn all_saved() -> LegacyPersonalizationSettings {
    LegacyPersonalizationSettings {
        about_user: Some("I write Rust".to_string()),
        style_rules: Some("Be terse".to_string()),
        custom_instructions_enabled: Some(true),
        memory_enabled: Some(true),
        tool_assisted_extraction_enabled: Some(true),
    }
}

#[test]
fn a_fresh_installation_maps_to_the_default_global_policy() {
    // Nothing was ever saved, so every default stands and no override is invented.
    let migrated = map_legacy_settings(&LegacyPersonalizationSettings::default()).expect("map");
    assert!(LegacyPersonalizationSettings::default().is_empty());
    assert_eq!(migrated.global.scope(), &PersonalizationPolicyScope::Global);
    assert_eq!(
        migrated.global.instruction_merge_mode(),
        InstructionMergeMode::Append
    );
    assert_eq!(migrated.global.memory_read_mode(), PolicyToggle::Enabled);
    assert_eq!(migrated.global.explicit_save_mode(), PolicyToggle::Enabled);
    assert_eq!(
        migrated.global.automatic_extraction_mode(),
        PolicyToggle::Enabled
    );
    assert!(migrated.global.about_user().is_empty());
    assert!(migrated.onepiece_override.is_none());
}

#[test]
fn every_saved_field_reaches_the_global_policy() {
    let migrated = map_legacy_settings(&all_saved()).expect("map");
    assert_eq!(migrated.global.about_user(), "I write Rust");
    assert_eq!(migrated.global.style_rules(), "Be terse");
    assert_eq!(
        migrated.global.instruction_merge_mode(),
        InstructionMergeMode::Append
    );
    assert_eq!(migrated.global.memory_read_mode(), PolicyToggle::Enabled);
    assert!(migrated.onepiece_override.is_none());
}

#[test]
fn an_explicit_false_is_never_overwritten_by_the_default() {
    // The concrete failure this prevents: a user who turned memory off finds it back on after
    // upgrading, because the migration could not tell "off" from "never touched".
    let legacy = LegacyPersonalizationSettings {
        memory_enabled: Some(false),
        ..all_saved()
    };
    let migrated = map_legacy_settings(&legacy).expect("map");
    assert_eq!(migrated.global.memory_read_mode(), PolicyToggle::Disabled);
    assert_eq!(migrated.global.explicit_save_mode(), PolicyToggle::Disabled);
    assert_eq!(
        migrated.global.automatic_extraction_mode(),
        PolicyToggle::Disabled
    );
}

#[test]
fn a_disabled_instruction_toggle_becomes_a_disabled_merge_mode() {
    let legacy = LegacyPersonalizationSettings {
        custom_instructions_enabled: Some(false),
        ..all_saved()
    };
    let migrated = map_legacy_settings(&legacy).expect("map");
    assert_eq!(
        migrated.global.instruction_merge_mode(),
        InstructionMergeMode::Disabled
    );
    // The text is preserved even while disabled: re-enabling must restore what the user wrote.
    assert_eq!(migrated.global.about_user(), "I write Rust");
}

#[test]
fn an_explicitly_cleared_field_is_distinct_from_one_that_was_never_set() {
    // `Some("")` says "I cleared this"; `None` says "I never set it". Only the first should
    // overwrite anything.
    let cleared = map_legacy_settings(&LegacyPersonalizationSettings {
        about_user: Some(String::new()),
        ..all_saved()
    })
    .expect("map");
    assert_eq!(cleared.global.about_user(), "");

    let never_set = map_legacy_settings(&LegacyPersonalizationSettings {
        about_user: None,
        ..all_saved()
    })
    .expect("map");
    assert_eq!(
        never_set.global.about_user(),
        "",
        "the default is also empty, so this asserts the path, not the value"
    );
    assert_eq!(never_set.global.style_rules(), "Be terse");
}

#[test]
fn memory_disabled_does_not_disable_global_memory_access() {
    // Global-memory access decides whether *globally scoped* records are eligible, which had no
    // legacy equivalent because every memory was global. Turning it off would hide every migrated
    // memory even after the user re-enabled memory.
    let migrated = map_legacy_settings(&LegacyPersonalizationSettings {
        memory_enabled: Some(false),
        ..all_saved()
    })
    .expect("map");
    assert_eq!(
        migrated.global.global_memory_access_mode(),
        PolicyToggle::Enabled
    );
}

#[test]
fn a_onepiece_override_appears_only_when_extraction_disagrees_with_the_global_default() {
    // Writing an override that repeats the layer above it creates a pin the user never asked for,
    // which then stops following later global changes.
    let agreeing = map_legacy_settings(&all_saved()).expect("map");
    assert!(agreeing.onepiece_override.is_none());

    let disagreeing = map_legacy_settings(&LegacyPersonalizationSettings {
        tool_assisted_extraction_enabled: Some(false),
        ..all_saved()
    })
    .expect("map");
    let override_record = disagreeing
        .onepiece_override
        .expect("an override is required when the values differ");
    assert_eq!(
        override_record.scope(),
        &PersonalizationPolicyScope::Agent {
            agent_id: AgentId::parse(ONEPIECE_AGENT_ID).expect("agent"),
        }
    );
    assert_eq!(
        override_record.automatic_extraction_mode(),
        PolicyToggle::Disabled
    );
    // Everything the override does not speak to keeps inheriting.
    assert_eq!(override_record.memory_read_mode(), PolicyToggle::Inherit);
    assert_eq!(
        override_record.instruction_merge_mode(),
        InstructionMergeMode::Inherit
    );
}

#[test]
fn an_override_also_appears_when_extraction_is_on_but_memory_is_off() {
    // The inverse disagreement: memory off makes the global extraction default disabled, so a
    // tool-assisted toggle left on is a genuine per-Agent difference.
    let migrated = map_legacy_settings(&LegacyPersonalizationSettings {
        memory_enabled: Some(false),
        tool_assisted_extraction_enabled: Some(true),
        ..all_saved()
    })
    .expect("map");
    assert_eq!(
        migrated
            .onepiece_override
            .expect("override")
            .automatic_extraction_mode(),
        PolicyToggle::Enabled
    );
}

#[test]
fn an_absent_tool_assisted_toggle_produces_no_override() {
    let migrated = map_legacy_settings(&LegacyPersonalizationSettings {
        tool_assisted_extraction_enabled: None,
        ..all_saved()
    })
    .expect("map");
    assert!(migrated.onepiece_override.is_none());
}

#[test]
fn the_projection_round_trips_the_legacy_shape() {
    // The compatibility window's read side: the old settings surface keeps working, but it reads
    // through to the policy rather than from its own copy.
    for legacy in [
        all_saved(),
        LegacyPersonalizationSettings {
            memory_enabled: Some(false),
            ..all_saved()
        },
        LegacyPersonalizationSettings {
            custom_instructions_enabled: Some(false),
            ..all_saved()
        },
        LegacyPersonalizationSettings {
            tool_assisted_extraction_enabled: Some(false),
            ..all_saved()
        },
    ] {
        let migrated = map_legacy_settings(&legacy).expect("map");
        let projected =
            project_to_legacy_settings(&migrated.global, migrated.onepiece_override.as_ref());
        assert_eq!(
            projected, legacy,
            "the projection must return exactly what was migrated"
        );
    }
}

#[test]
fn the_projection_reads_extraction_through_the_override_when_one_exists() {
    let migrated = map_legacy_settings(&LegacyPersonalizationSettings {
        tool_assisted_extraction_enabled: Some(false),
        ..all_saved()
    })
    .expect("map");
    let projected =
        project_to_legacy_settings(&migrated.global, migrated.onepiece_override.as_ref());
    assert_eq!(projected.tool_assisted_extraction_enabled, Some(false));

    // Without the override in hand, the projection falls back to the global value rather than
    // inventing one.
    let without = project_to_legacy_settings(&migrated.global, None);
    assert_eq!(without.tool_assisted_extraction_enabled, Some(true));
}

#[test]
fn an_inheriting_override_does_not_shadow_the_global_value() {
    let migrated = map_legacy_settings(&all_saved()).expect("map");
    let mut inheriting =
        crate::contexts::personalization::domain::PersonalizationPolicyRecord::inheriting(
            PersonalizationPolicyScope::Agent {
                agent_id: AgentId::parse(ONEPIECE_AGENT_ID).expect("agent"),
            },
        );
    inheriting.set_automatic_extraction_mode(PolicyToggle::Inherit);
    let projected = project_to_legacy_settings(&migrated.global, Some(&inheriting));
    assert_eq!(projected.tool_assisted_extraction_enabled, Some(true));
}
