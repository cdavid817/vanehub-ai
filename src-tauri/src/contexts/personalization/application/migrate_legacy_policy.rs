use super::error::PersonalizationApplicationError;
use crate::contexts::personalization::domain::{
    AgentId, InstructionMergeMode, PersonalizationPolicyRecord, PersonalizationPolicyScope,
    PolicyToggle,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// The stable Agent id OnePiece is registered under.
///
/// Named once, here, because the tool-assisted extraction toggle only ever governed OnePiece's own
/// compaction. This is a migration fact about one historical setting, not a policy Agent list —
/// nothing else in this context branches on an Agent id.
pub(crate) const ONEPIECE_AGENT_ID: &str = "onepiece";

/// What the legacy `AppSettings` held, with "never saved" distinguishable from "saved as false".
///
/// Every field is optional on purpose. The settings type stores concrete values with defaults, so
/// by the time it is read a user who explicitly turned memory off and a user who never touched it
/// look identical. Only the caller reading the persisted keys can tell them apart, and the
/// distinction has to survive: overwriting an explicit `false` with the default `true` would
/// silently re-enable memory for someone who turned it off.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LegacyPersonalizationSettings {
    pub(crate) about_user: Option<String>,
    pub(crate) style_rules: Option<String>,
    pub(crate) custom_instructions_enabled: Option<bool>,
    pub(crate) memory_enabled: Option<bool>,
    pub(crate) tool_assisted_extraction_enabled: Option<bool>,
}

impl LegacyPersonalizationSettings {
    pub(crate) fn is_empty(&self) -> bool {
        self.about_user.is_none()
            && self.style_rules.is_none()
            && self.custom_instructions_enabled.is_none()
            && self.memory_enabled.is_none()
            && self.tool_assisted_extraction_enabled.is_none()
    }
}

/// The rows a legacy migration produces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MigratedPolicy {
    pub(crate) global: PersonalizationPolicyRecord,
    /// Present only when OnePiece's tool-assisted extraction differed from the global default.
    ///
    /// Writing an override that says the same thing as the layer above it would create a row the
    /// user never asked for, which then stops following later global changes — an override is a
    /// pin, so an unnecessary one is a silent behavior change.
    pub(crate) onepiece_override: Option<PersonalizationPolicyRecord>,
}

/// Maps the legacy fields onto the dedicated policy.
///
/// Pure: it produces the rows, and the caller commits them in one transaction with the migration
/// marker. Keeping the mapping free of persistence is what lets every distinction below —
/// `None` vs `Some(false)` vs `Some("")` — be tested without a database.
pub(crate) fn map_legacy_settings(
    legacy: &LegacyPersonalizationSettings,
) -> Result<MigratedPolicy> {
    let mut global = PersonalizationPolicyRecord::default_global();

    // Absent means "never saved", so the default stands. Present means the user's value wins, and
    // an explicitly empty string is a value: it says "I cleared this", not "I never set it".
    if let Some(about_user) = legacy.about_user.as_ref() {
        global.set_about_user(about_user.clone());
    }
    if let Some(style_rules) = legacy.style_rules.as_ref() {
        global.set_style_rules(style_rules.clone());
    }

    // The single host-level instruction toggle becomes the global merge mode. `Append` rather than
    // `Replace` because there is nothing below global to replace, and it is the mode a later Agent
    // or workspace override composes with.
    if let Some(enabled) = legacy.custom_instructions_enabled {
        global.set_instruction_merge_mode(if enabled {
            InstructionMergeMode::Append
        } else {
            InstructionMergeMode::Disabled
        });
    }

    // One legacy switch governed reading, saving, and extraction together, so it maps onto all
    // three. Splitting them is the point of the new model, but migration must reproduce the old
    // behavior exactly — inventing a split here would change what an existing user experiences.
    if let Some(enabled) = legacy.memory_enabled {
        let toggle = toggle_for(enabled);
        global.set_memory_read_mode(toggle);
        global.set_explicit_save_mode(toggle);
        global.set_automatic_extraction_mode(toggle);
        // Global memory access is not the same switch: it decides whether *global-scoped* records
        // are eligible, which had no legacy equivalent because every memory was global. Turning it
        // off here would hide every migrated memory.
    }

    global.validate()?;

    let onepiece_override = onepiece_extraction_override(legacy, &global)?;
    Ok(MigratedPolicy {
        global,
        onepiece_override,
    })
}

fn toggle_for(enabled: bool) -> PolicyToggle {
    if enabled {
        PolicyToggle::Enabled
    } else {
        PolicyToggle::Disabled
    }
}

/// Produces an Agent override only when tool-assisted extraction disagreed with the global default.
fn onepiece_extraction_override(
    legacy: &LegacyPersonalizationSettings,
    global: &PersonalizationPolicyRecord,
) -> Result<Option<PersonalizationPolicyRecord>> {
    let Some(tool_assisted) = legacy.tool_assisted_extraction_enabled else {
        return Ok(None);
    };
    let effective_global = matches!(global.automatic_extraction_mode(), PolicyToggle::Enabled);
    if tool_assisted == effective_global {
        return Ok(None);
    }

    let agent_id = AgentId::parse(ONEPIECE_AGENT_ID)?;
    let mut record =
        PersonalizationPolicyRecord::inheriting(PersonalizationPolicyScope::Agent { agent_id });
    record.set_automatic_extraction_mode(toggle_for(tool_assisted));
    record.validate()?;
    Ok(Some(record))
}

/// Projects the dedicated policy back onto the legacy field shape.
///
/// The compatibility window's read side: the old settings surface keeps working while the new UI
/// is built, but it reads through to the policy rather than from its own copy. Two stores that
/// both claim to be authoritative is the state this change exists to end.
pub(crate) fn project_to_legacy_settings(
    global: &PersonalizationPolicyRecord,
    onepiece_override: Option<&PersonalizationPolicyRecord>,
) -> LegacyPersonalizationSettings {
    let extraction = onepiece_override
        .map(PersonalizationPolicyRecord::automatic_extraction_mode)
        .filter(|toggle| !matches!(toggle, PolicyToggle::Inherit))
        .unwrap_or_else(|| global.automatic_extraction_mode());

    LegacyPersonalizationSettings {
        about_user: Some(global.about_user().to_string()),
        style_rules: Some(global.style_rules().to_string()),
        custom_instructions_enabled: Some(!matches!(
            global.instruction_merge_mode(),
            InstructionMergeMode::Disabled
        )),
        memory_enabled: Some(matches!(global.memory_read_mode(), PolicyToggle::Enabled)),
        tool_assisted_extraction_enabled: Some(matches!(extraction, PolicyToggle::Enabled)),
    }
}
