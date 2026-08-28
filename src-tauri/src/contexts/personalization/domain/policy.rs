use serde::{Deserialize, Serialize};

use super::error::PersonalizationDomainError;
use super::scope::PersonalizationPolicyScope;

/// Per-field bound on stored custom-instruction text, counted in Unicode characters rather than
/// bytes so a user writing CJK is not silently held to a third of the advertised limit.
pub(crate) const INSTRUCTION_FIELD_MAX_CHARS: usize = 3_000;

/// Reserved for user-created personalization profiles. Storing it from the start means adding
/// profiles later does not require rewriting every persisted policy row.
pub(crate) const DEFAULT_POLICY_SET_ID: &str = "default";

/// A boolean policy dimension at a non-global scope, where "unset" is a distinct third state.
///
/// Two-state storage cannot express "this workspace has no opinion about extraction", which is the
/// difference between an override that follows a later global change and one that pins the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PolicyToggle {
    Inherit,
    Enabled,
    Disabled,
}

impl PolicyToggle {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Inherit => "inherit",
            Self::Enabled => "enabled",
            Self::Disabled => "disabled",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, PersonalizationDomainError> {
        match value {
            "inherit" => Ok(Self::Inherit),
            "enabled" => Ok(Self::Enabled),
            "disabled" => Ok(Self::Disabled),
            other => Err(PersonalizationDomainError::UnknownPolicyToggle(
                other.to_string(),
            )),
        }
    }

    /// Applies this layer on top of what lower-precedence layers already resolved.
    pub(crate) fn resolve_over(self, inherited: bool) -> bool {
        match self {
            Self::Inherit => inherited,
            Self::Enabled => true,
            Self::Disabled => false,
        }
    }
}

/// How a layer's instruction text combines with what lower-precedence layers contributed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InstructionMergeMode {
    /// Contribute nothing and change nothing.
    Inherit,
    /// Keep inherited segments and add this layer's non-empty fields after them.
    Append,
    /// Drop lower-precedence *user* segments and use this layer's fields. Core, safety, role, and
    /// runtime instructions are outside personalization and are never dropped.
    Replace,
    /// Drop every user-personalization segment for this request.
    Disabled,
}

impl InstructionMergeMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Inherit => "inherit",
            Self::Append => "append",
            Self::Replace => "replace",
            Self::Disabled => "disabled",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, PersonalizationDomainError> {
        match value {
            "inherit" => Ok(Self::Inherit),
            "append" => Ok(Self::Append),
            "replace" => Ok(Self::Replace),
            "disabled" => Ok(Self::Disabled),
            other => Err(PersonalizationDomainError::UnknownMergeMode(
                other.to_string(),
            )),
        }
    }
}

/// Durable per-session personalization behavior.
///
/// A hard restriction rather than another policy layer: it is applied last and can only narrow
/// what the resolved policy allows, so no override can widen a temporary session back into
/// long-term memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SessionPersonalizationMode {
    #[default]
    Standard,
    ProjectOnly,
    Temporary,
}

impl SessionPersonalizationMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::ProjectOnly => "project-only",
            Self::Temporary => "temporary",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, PersonalizationDomainError> {
        match value {
            "standard" => Ok(Self::Standard),
            "project-only" => Ok(Self::ProjectOnly),
            "temporary" => Ok(Self::Temporary),
            other => Err(PersonalizationDomainError::UnknownSessionMode(
                other.to_string(),
            )),
        }
    }

    /// Project-only has nothing to scope to without a workspace, so creation must be refused
    /// rather than silently degraded to standard.
    pub(crate) fn requires_workspace(self) -> bool {
        matches!(self, Self::ProjectOnly)
    }
}

/// A stale or unissued expected revision. Carries both numbers so the UI can explain the conflict
/// without guessing which side moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RevisionConflict {
    pub(crate) expected: u64,
    pub(crate) current: u64,
}

/// The result of an expected-revision policy write.
///
/// A conflict is a normal outcome carrying the current record, not an error: the UI needs the
/// server's version to offer a comparison, and it must keep the user's draft either way. Returning
/// `Err` here would push callers toward discarding the draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PatchPolicyResult {
    Updated(PersonalizationPolicyRecord),
    Conflict {
        current: PersonalizationPolicyRecord,
    },
}

/// One persisted policy layer.
///
/// Deliberately holds no timestamps: `created_at`/`updated_at` are persistence metadata owned by
/// the repository row, and keeping them out means the domain needs no clock and stays trivially
/// testable. `revision` is here because optimistic concurrency is a domain rule, not a storage
/// detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PersonalizationPolicyRecord {
    scope: PersonalizationPolicyScope,
    policy_set_id: String,
    instruction_merge_mode: InstructionMergeMode,
    about_user: String,
    style_rules: String,
    memory_read_mode: PolicyToggle,
    explicit_save_mode: PolicyToggle,
    automatic_extraction_mode: PolicyToggle,
    global_memory_access_mode: PolicyToggle,
    revision: u64,
}

/// A partial update. Every field is optional so writing one dimension cannot republish the others
/// — the concrete failure mode that whole-`AppSettings` saves had.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PersonalizationPolicyPatch {
    pub(crate) instruction_merge_mode: Option<InstructionMergeMode>,
    pub(crate) about_user: Option<String>,
    pub(crate) style_rules: Option<String>,
    pub(crate) memory_read_mode: Option<PolicyToggle>,
    pub(crate) explicit_save_mode: Option<PolicyToggle>,
    pub(crate) automatic_extraction_mode: Option<PolicyToggle>,
    pub(crate) global_memory_access_mode: Option<PolicyToggle>,
}

impl PersonalizationPolicyRecord {
    /// The row a fresh installation starts from and a migrated installation lands on.
    ///
    /// Enabled, not fail-closed: this preserves behavior for users who already had memory and
    /// instructions working. Fail-closed applies when no validated policy can be *read* at all,
    /// which is a different situation and is expressed by the snapshot, not by this row.
    pub(crate) fn default_global() -> Self {
        Self {
            scope: PersonalizationPolicyScope::Global,
            policy_set_id: DEFAULT_POLICY_SET_ID.to_string(),
            instruction_merge_mode: InstructionMergeMode::Append,
            about_user: String::new(),
            style_rules: String::new(),
            memory_read_mode: PolicyToggle::Enabled,
            explicit_save_mode: PolicyToggle::Enabled,
            automatic_extraction_mode: PolicyToggle::Enabled,
            global_memory_access_mode: PolicyToggle::Enabled,
            revision: 0,
        }
    }

    /// A newly created override that changes nothing until the user sets something.
    pub(crate) fn inheriting(scope: PersonalizationPolicyScope) -> Self {
        Self {
            scope,
            policy_set_id: DEFAULT_POLICY_SET_ID.to_string(),
            instruction_merge_mode: InstructionMergeMode::Inherit,
            about_user: String::new(),
            style_rules: String::new(),
            memory_read_mode: PolicyToggle::Inherit,
            explicit_save_mode: PolicyToggle::Inherit,
            automatic_extraction_mode: PolicyToggle::Inherit,
            global_memory_access_mode: PolicyToggle::Inherit,
            revision: 0,
        }
    }

    pub(crate) fn scope(&self) -> &PersonalizationPolicyScope {
        &self.scope
    }

    pub(crate) fn policy_set_id(&self) -> &str {
        &self.policy_set_id
    }

    pub(crate) fn instruction_merge_mode(&self) -> InstructionMergeMode {
        self.instruction_merge_mode
    }

    pub(crate) fn about_user(&self) -> &str {
        &self.about_user
    }

    pub(crate) fn style_rules(&self) -> &str {
        &self.style_rules
    }

    pub(crate) fn memory_read_mode(&self) -> PolicyToggle {
        self.memory_read_mode
    }

    pub(crate) fn explicit_save_mode(&self) -> PolicyToggle {
        self.explicit_save_mode
    }

    pub(crate) fn automatic_extraction_mode(&self) -> PolicyToggle {
        self.automatic_extraction_mode
    }

    pub(crate) fn global_memory_access_mode(&self) -> PolicyToggle {
        self.global_memory_access_mode
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn set_instruction_merge_mode(&mut self, mode: InstructionMergeMode) {
        self.instruction_merge_mode = mode;
    }

    pub(crate) fn set_about_user(&mut self, value: String) {
        self.about_user = value;
    }

    pub(crate) fn set_style_rules(&mut self, value: String) {
        self.style_rules = value;
    }

    pub(crate) fn set_memory_read_mode(&mut self, toggle: PolicyToggle) {
        self.memory_read_mode = toggle;
    }

    pub(crate) fn set_explicit_save_mode(&mut self, toggle: PolicyToggle) {
        self.explicit_save_mode = toggle;
    }

    pub(crate) fn set_automatic_extraction_mode(&mut self, toggle: PolicyToggle) {
        self.automatic_extraction_mode = toggle;
    }

    pub(crate) fn set_global_memory_access_mode(&mut self, toggle: PolicyToggle) {
        self.global_memory_access_mode = toggle;
    }

    pub(crate) fn set_policy_set_id(&mut self, value: String) {
        self.policy_set_id = value;
    }

    pub(crate) fn set_revision(&mut self, revision: u64) {
        self.revision = revision;
    }

    /// An expectation that does not name the current revision is a conflict in both directions.
    /// A higher-than-current expectation is not "close enough": it means the caller is reasoning
    /// about a revision this store never issued.
    pub(crate) fn check_expected_revision(
        &self,
        expected: Option<u64>,
    ) -> Result<(), RevisionConflict> {
        match expected {
            None => Ok(()),
            Some(expected) if expected == self.revision => Ok(()),
            Some(expected) => Err(RevisionConflict {
                expected,
                current: self.revision,
            }),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), PersonalizationDomainError> {
        if matches!(self.scope, PersonalizationPolicyScope::Global) {
            if matches!(self.instruction_merge_mode, InstructionMergeMode::Inherit) {
                return Err(PersonalizationDomainError::GlobalScopeCannotInherit {
                    field: "instruction_merge_mode",
                });
            }
            for (field, toggle) in [
                ("memory_read_mode", self.memory_read_mode),
                ("explicit_save_mode", self.explicit_save_mode),
                ("automatic_extraction_mode", self.automatic_extraction_mode),
                ("global_memory_access_mode", self.global_memory_access_mode),
            ] {
                if matches!(toggle, PolicyToggle::Inherit) {
                    return Err(PersonalizationDomainError::GlobalScopeCannotInherit { field });
                }
            }
        }
        validate_instruction_field("about_user", &self.about_user)?;
        validate_instruction_field("style_rules", &self.style_rules)?;
        Ok(())
    }

    /// Applies a patch and advances the revision, or refuses and leaves the caller's record
    /// untouched. Validation runs on the *result*, so a patch cannot arrive at an invalid row by
    /// combining individually plausible fields.
    pub(crate) fn apply(
        mut self,
        patch: PersonalizationPolicyPatch,
    ) -> Result<Self, PersonalizationDomainError> {
        if let Some(mode) = patch.instruction_merge_mode {
            self.instruction_merge_mode = mode;
        }
        if let Some(about_user) = patch.about_user {
            self.about_user = about_user;
        }
        if let Some(style_rules) = patch.style_rules {
            self.style_rules = style_rules;
        }
        if let Some(toggle) = patch.memory_read_mode {
            self.memory_read_mode = toggle;
        }
        if let Some(toggle) = patch.explicit_save_mode {
            self.explicit_save_mode = toggle;
        }
        if let Some(toggle) = patch.automatic_extraction_mode {
            self.automatic_extraction_mode = toggle;
        }
        if let Some(toggle) = patch.global_memory_access_mode {
            self.global_memory_access_mode = toggle;
        }
        self.validate()?;
        self.revision = self.revision.saturating_add(1);
        Ok(self)
    }
}

fn validate_instruction_field(
    field: &'static str,
    value: &str,
) -> Result<(), PersonalizationDomainError> {
    let actual = value.chars().count();
    if actual > INSTRUCTION_FIELD_MAX_CHARS {
        return Err(PersonalizationDomainError::InstructionFieldTooLong {
            field,
            limit: INSTRUCTION_FIELD_MAX_CHARS,
            actual,
        });
    }
    Ok(())
}
