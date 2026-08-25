use std::sync::Arc;

use super::error::PersonalizationApplicationError;
use super::migrate_legacy_policy::{
    project_to_legacy_settings, LegacyPersonalizationSettings, ONEPIECE_AGENT_ID,
};
use super::policy_cache::LastKnownGoodPolicyCache;
use super::ports::{ClockPort, PolicyRepository};
use crate::contexts::personalization::domain::{
    AgentId, InstructionMergeMode, PatchPolicyResult, PersonalizationPolicyPatch,
    PersonalizationPolicyRecord, PersonalizationPolicyScope, PolicyToggle, RevisionConflict,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// The dedicated policy, rendered in the shape the pre-governance settings page understands.
///
/// Carries the revision it was read at because that page has no version of its own. Echoing this
/// back on save is what makes the compatibility window's concurrency real: the check is against the
/// revision the user's screen was rendered from, not against one the server re-reads at save time,
/// which would accept every write and be last-response-wins wearing an expected-revision costume.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LegacySettingsView {
    pub(crate) settings: LegacyPersonalizationSettings,
    pub(crate) revision: u64,
}

/// One field of the pre-governance settings page.
///
/// Typed per field rather than a whole-settings struct: a page that posts every field on every save
/// republishes the four the user did not touch, which is exactly how one screen's stale copy
/// silently reverts another's edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LegacySettingField {
    AboutUser(String),
    StyleRules(String),
    CustomInstructionsEnabled(bool),
    MemoryEnabled(bool),
    ToolAssistedExtractionEnabled(bool),
}

impl LegacySettingField {
    /// The camelCase key the pre-governance surface uses, or `None` for a key it does not own.
    ///
    /// Returning `None` rather than erroring is deliberate: the settings page carries many keys this
    /// context has no opinion about, and they must keep taking their existing path.
    pub(crate) fn from_key_and_value(key: &str, value: &str) -> Option<Self> {
        let flag = || value == "true";
        match key {
            "customInstructionsAboutUser" => Some(Self::AboutUser(value.to_string())),
            "customInstructionsStyleRules" => Some(Self::StyleRules(value.to_string())),
            "customInstructionsEnabled" => Some(Self::CustomInstructionsEnabled(flag())),
            "memoryEnabled" => Some(Self::MemoryEnabled(flag())),
            "memoryToolAssistedChatsEnabled" => Some(Self::ToolAssistedExtractionEnabled(flag())),
            _ => None,
        }
    }

    fn into_patch(self) -> PersonalizationPolicyPatch {
        let mut patch = PersonalizationPolicyPatch::default();
        match self {
            Self::AboutUser(value) => patch.about_user = Some(value),
            Self::StyleRules(value) => patch.style_rules = Some(value),
            // The legacy switch meant "use the instructions at all". Off maps to `Disabled` and on
            // to `Append`, which is what the page's behavior was: instructions added to whatever the
            // Agent already had, never replacing it.
            Self::CustomInstructionsEnabled(enabled) => {
                patch.instruction_merge_mode = Some(if enabled {
                    InstructionMergeMode::Append
                } else {
                    InstructionMergeMode::Disabled
                });
            }
            // One legacy switch governed both reading and saving, so both move together. Splitting
            // them here would let the old page put the policy into a state it cannot express and
            // therefore cannot show the user.
            Self::MemoryEnabled(enabled) => {
                let toggle = toggle(enabled);
                patch.memory_read_mode = Some(toggle);
                patch.explicit_save_mode = Some(toggle);
                patch.automatic_extraction_mode = Some(toggle);
            }
            Self::ToolAssistedExtractionEnabled(enabled) => {
                patch.automatic_extraction_mode = Some(toggle(enabled));
            }
        }
        patch
    }
}

fn toggle(enabled: bool) -> PolicyToggle {
    if enabled {
        PolicyToggle::Enabled
    } else {
        PolicyToggle::Disabled
    }
}

/// Read-through and write-through for the settings page that predates governance.
///
/// The dedicated policy is the source of truth from the moment migration completes. This exists so
/// the existing page keeps working against it until the new one lands — not so the legacy rows keep
/// a second opinion. Nothing here writes back to `AppSettings`.
pub(crate) struct LegacySettingsCompatibility {
    policies: Arc<dyn PolicyRepository>,
    clock: Arc<dyn ClockPort>,
    /// Dropped after every successful write. Invalidating here rather than through a general event
    /// bus keeps the rule where the write is: whoever changes the policy is who knows it changed.
    cache: Arc<LastKnownGoodPolicyCache>,
}

impl LegacySettingsCompatibility {
    pub(crate) fn new(
        policies: Arc<dyn PolicyRepository>,
        clock: Arc<dyn ClockPort>,
        cache: Arc<LastKnownGoodPolicyCache>,
    ) -> Self {
        Self {
            policies,
            clock,
            cache,
        }
    }

    /// The current policy in legacy shape.
    ///
    /// A missing global row is an error rather than a default: answering with defaults would show
    /// the user settings nobody stored, and a save from that screen would then write them.
    pub(crate) fn view(&self) -> Result<LegacySettingsView> {
        let global = self.global()?;
        let onepiece = self.onepiece_override()?;
        Ok(LegacySettingsView {
            settings: project_to_legacy_settings(&global, onepiece.as_ref()),
            revision: global.revision(),
        })
    }

    /// Applies one field, refusing a write whose expected revision is stale.
    ///
    /// Returns the fresh view so the caller can re-render from what was actually stored rather than
    /// from what it hoped it stored.
    pub(crate) fn apply(
        &self,
        field: LegacySettingField,
        expected_revision: u64,
    ) -> Result<LegacySettingsView> {
        let clears_extraction_pin = matches!(
            field,
            LegacySettingField::MemoryEnabled(_)
                | LegacySettingField::ToolAssistedExtractionEnabled(_)
        );
        let now = self.clock.now();
        let result = self.policies.patch(
            &PersonalizationPolicyScope::Global,
            Some(expected_revision),
            field.into_patch(),
            now,
        )?;
        let saved = match result {
            PatchPolicyResult::Updated(record) => record,
            // Typed rather than swallowed: the page has to keep the user's draft and show them what
            // the stored value actually became, which it cannot do if a conflict is
            // indistinguishable from a storage error.
            PatchPolicyResult::Conflict { current } => {
                return Err(PersonalizationApplicationError::RevisionConflict(
                    RevisionConflict {
                        expected: expected_revision,
                        current: current.revision(),
                    },
                ))
            }
        };

        // An extraction override that survives here would mask the value the user just chose, and
        // the page would show their edit as not having taken effect. Reset to `Inherit` rather than
        // deleted: inherit means "follow the layer above", which is exactly what an app-wide switch
        // asks for. This is a consequence of an edit the revision check already authorized, not a
        // second independent write, which is why it does not carry its own expected revision.
        if clears_extraction_pin {
            self.clear_extraction_pin(now)?;
        }

        // After the write, not before: a failed patch must not throw away a bundle that is still
        // correct.
        self.cache.invalidate();

        let onepiece = self.onepiece_override()?;
        Ok(LegacySettingsView {
            settings: project_to_legacy_settings(&saved, onepiece.as_ref()),
            revision: saved.revision(),
        })
    }

    fn global(&self) -> Result<PersonalizationPolicyRecord> {
        self.policies
            .load(&PersonalizationPolicyScope::Global)?
            .ok_or(PersonalizationApplicationError::NotFound)
    }

    fn onepiece_override(&self) -> Result<Option<PersonalizationPolicyRecord>> {
        let Ok(agent_id) = AgentId::parse(ONEPIECE_AGENT_ID) else {
            return Ok(None);
        };
        self.policies
            .load(&PersonalizationPolicyScope::Agent { agent_id })
    }

    fn clear_extraction_pin(&self, now: chrono::DateTime<chrono::Utc>) -> Result<()> {
        let Some(existing) = self.onepiece_override()? else {
            return Ok(());
        };
        if matches!(existing.automatic_extraction_mode(), PolicyToggle::Inherit) {
            return Ok(());
        }
        let Ok(agent_id) = AgentId::parse(ONEPIECE_AGENT_ID) else {
            return Ok(());
        };
        self.policies.patch(
            &PersonalizationPolicyScope::Agent { agent_id },
            Some(existing.revision()),
            PersonalizationPolicyPatch {
                automatic_extraction_mode: Some(PolicyToggle::Inherit),
                ..PersonalizationPolicyPatch::default()
            },
            now,
        )?;
        Ok(())
    }
}
