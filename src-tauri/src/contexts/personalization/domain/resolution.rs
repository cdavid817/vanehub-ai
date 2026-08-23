use sha2::{Digest, Sha256};

use super::policy::{
    InstructionMergeMode, PersonalizationPolicyPatch, PersonalizationPolicyRecord, PolicyToggle,
    SessionPersonalizationMode,
};
use super::snapshot::{
    EffectiveMemoryAccess, EffectivePersonalizationSnapshot, PersonalizationExclusion,
    PersonalizationExclusionReason, PersonalizationResolutionContext,
    PersonalizationRuntimeCapabilities, PersonalizationWarning, PersonalizationWarningCode,
    ResolvedInstructionSegment,
};

/// The durable rows that apply to one resolution context, plus the session's own override.
///
/// The caller selects these by scope key; this type only fixes their order. `global` is optional
/// because "no global row" is a real state — it is what an installation looks like before
/// migration completes — and it is the state that must fail closed rather than be filled in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PersonalizationLayers {
    pub(crate) global: Option<PersonalizationPolicyRecord>,
    pub(crate) agent: Option<PersonalizationPolicyRecord>,
    pub(crate) workspace: Option<PersonalizationPolicyRecord>,
    pub(crate) workspace_agent: Option<PersonalizationPolicyRecord>,
    /// Stored with the session record rather than as a durable policy row, so it disappears with
    /// the session. Shares the patch shape because it is exactly the same "set some dimensions,
    /// leave the rest alone" idea.
    pub(crate) session_override: Option<PersonalizationPolicyPatch>,
}

impl PersonalizationLayers {
    fn durable_layers(&self) -> impl Iterator<Item = &PersonalizationPolicyRecord> {
        [
            self.global.as_ref(),
            self.agent.as_ref(),
            self.workspace.as_ref(),
            self.workspace_agent.as_ref(),
        ]
        .into_iter()
        .flatten()
    }
}

/// Whether stored personalization data is currently trustworthy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MaintenanceState {
    /// Bumped by each completed migration. Part of the revision token so a snapshot taken before a
    /// migration cannot be mistaken for one taken after.
    pub(crate) migration_generation: u64,
    pub(crate) migration_complete: bool,
    pub(crate) repair_required: bool,
}

/// Resolves one immutable snapshot.
///
/// Order is fixed and total: built-in safe defaults, global, Agent, workspace, workspace-Agent,
/// session override, then hard session-mode restrictions, runtime capabilities, and maintenance
/// state. The last three can only ever narrow — that is what makes "a temporary session cannot be
/// talked back into long-term memory" a property rather than a convention.
pub(crate) fn resolve(
    context: PersonalizationResolutionContext,
    layers: PersonalizationLayers,
    capabilities: PersonalizationRuntimeCapabilities,
    maintenance: MaintenanceState,
) -> EffectivePersonalizationSnapshot {
    let Some(global) = layers.global.as_ref() else {
        return EffectivePersonalizationSnapshot::fail_closed(
            context,
            PersonalizationWarningCode::NoValidatedPolicy,
        );
    };
    let revision_token = revision_token(&context, &layers, maintenance);
    let _ = global;

    let mut warnings = Vec::new();
    let mut exclusions = Vec::new();

    let (mut effective_instruction_mode, mut instruction_segments) = resolve_instructions(&layers);
    let mut access = resolve_toggles(&layers);

    if let Some(session_override) = layers.session_override.as_ref() {
        apply_session_override(
            session_override,
            &context,
            &mut effective_instruction_mode,
            &mut instruction_segments,
            &mut access,
        );
    }

    apply_session_mode(&context, &mut access, &mut exclusions);
    apply_capabilities(
        capabilities,
        &mut instruction_segments,
        &mut access,
        &mut warnings,
        &mut exclusions,
    );
    apply_maintenance(maintenance, &mut access, &mut warnings, &mut exclusions);

    if instruction_segments.is_empty()
        && !matches!(effective_instruction_mode, InstructionMergeMode::Disabled)
    {
        // Nothing survived, whatever the stored mode said. Reporting the stored mode here would
        // make the preview claim an append that produced no text.
        effective_instruction_mode = InstructionMergeMode::Disabled;
    }

    EffectivePersonalizationSnapshot {
        revision_token,
        context,
        effective_instruction_mode,
        instruction_segments,
        memory_access: access,
        exclusions,
        warnings,
    }
}

fn resolve_instructions(
    layers: &PersonalizationLayers,
) -> (InstructionMergeMode, Vec<ResolvedInstructionSegment>) {
    let mut mode = InstructionMergeMode::Disabled;
    let mut segments = Vec::new();
    for layer in layers.durable_layers() {
        match layer.instruction_merge_mode() {
            // Contributes nothing and does not become the effective mode, so a preview keeps
            // naming the layer that actually decided.
            InstructionMergeMode::Inherit => {}
            InstructionMergeMode::Append => {
                mode = InstructionMergeMode::Append;
                push_segment(&mut segments, layer);
            }
            InstructionMergeMode::Replace => {
                mode = InstructionMergeMode::Replace;
                segments.clear();
                push_segment(&mut segments, layer);
            }
            InstructionMergeMode::Disabled => {
                mode = InstructionMergeMode::Disabled;
                segments.clear();
            }
        }
    }
    (mode, segments)
}

fn push_segment(
    segments: &mut Vec<ResolvedInstructionSegment>,
    layer: &PersonalizationPolicyRecord,
) {
    if let Some(segment) = ResolvedInstructionSegment::from_scope(
        layer.scope(),
        layer.about_user(),
        layer.style_rules(),
    ) {
        segments.push(segment);
    }
}

/// Built-in safe defaults are all-denied; the global row, which may not inherit, is what turns
/// them on. An installation whose global row is missing never reaches here.
fn resolve_toggles(layers: &PersonalizationLayers) -> EffectiveMemoryAccess {
    let mut access = EffectiveMemoryAccess::denied();
    for layer in layers.durable_layers() {
        access.read = layer.memory_read_mode().resolve_over(access.read);
        access.explicit_save = layer
            .explicit_save_mode()
            .resolve_over(access.explicit_save);
        access.automatic_extraction = layer
            .automatic_extraction_mode()
            .resolve_over(access.automatic_extraction);
        access.global_memory = layer
            .global_memory_access_mode()
            .resolve_over(access.global_memory);
    }
    access
}

fn apply_session_override(
    patch: &PersonalizationPolicyPatch,
    context: &PersonalizationResolutionContext,
    mode: &mut InstructionMergeMode,
    segments: &mut Vec<ResolvedInstructionSegment>,
    access: &mut EffectiveMemoryAccess,
) {
    if let Some(toggle) = patch.memory_read_mode {
        access.read = toggle.resolve_over(access.read);
    }
    if let Some(toggle) = patch.explicit_save_mode {
        access.explicit_save = toggle.resolve_over(access.explicit_save);
    }
    if let Some(toggle) = patch.automatic_extraction_mode {
        access.automatic_extraction = toggle.resolve_over(access.automatic_extraction);
    }
    if let Some(toggle) = patch.global_memory_access_mode {
        access.global_memory = toggle.resolve_over(access.global_memory);
    }

    let Some(override_mode) = patch.instruction_merge_mode else {
        return;
    };
    let about_user = patch.about_user.clone().unwrap_or_default();
    let style_rules = patch.style_rules.clone().unwrap_or_default();
    let session_segment =
        (!about_user.is_empty() || !style_rules.is_empty()).then(|| ResolvedInstructionSegment {
            scope_kind: "session",
            scope_key: format!("session/{}", context.session_id),
            about_user,
            style_rules,
        });
    match override_mode {
        InstructionMergeMode::Inherit => {}
        InstructionMergeMode::Append => {
            *mode = InstructionMergeMode::Append;
            segments.extend(session_segment);
        }
        InstructionMergeMode::Replace => {
            *mode = InstructionMergeMode::Replace;
            segments.clear();
            segments.extend(session_segment);
        }
        InstructionMergeMode::Disabled => {
            *mode = InstructionMergeMode::Disabled;
            segments.clear();
        }
    }
}

/// Hard restrictions. Applied after every layer precisely so that no override can widen them.
fn apply_session_mode(
    context: &PersonalizationResolutionContext,
    access: &mut EffectiveMemoryAccess,
    exclusions: &mut Vec<PersonalizationExclusion>,
) {
    match context.session_mode {
        SessionPersonalizationMode::Standard => {
            access.workspace = context.workspace.as_ref().map(|w| w.key().clone());
        }
        SessionPersonalizationMode::ProjectOnly => {
            let Some(workspace) = context.workspace.as_ref() else {
                // Creation is supposed to reject this. Reaching resolution without a workspace
                // means something upstream failed, and "read everything global" is the one
                // interpretation a project-isolated session must never degrade to.
                *access = EffectiveMemoryAccess::denied();
                exclusions.push(PersonalizationExclusion {
                    reason: PersonalizationExclusionReason::ProjectOnlySession,
                    count: 0,
                });
                return;
            };
            access.global_memory = false;
            access.workspace = Some(workspace.key().clone());
            exclusions.push(PersonalizationExclusion {
                reason: PersonalizationExclusionReason::ProjectOnlySession,
                count: 0,
            });
        }
        SessionPersonalizationMode::Temporary => {
            *access = EffectiveMemoryAccess::denied();
            exclusions.push(PersonalizationExclusion {
                reason: PersonalizationExclusionReason::TemporarySession,
                count: 0,
            });
        }
    }
}

fn apply_capabilities(
    capabilities: PersonalizationRuntimeCapabilities,
    segments: &mut Vec<ResolvedInstructionSegment>,
    access: &mut EffectiveMemoryAccess,
    warnings: &mut Vec<PersonalizationWarning>,
    exclusions: &mut Vec<PersonalizationExclusion>,
) {
    let mut unsupported_override = false;
    if !capabilities.supports_custom_instructions && !segments.is_empty() {
        segments.clear();
        unsupported_override = true;
    }
    if !capabilities.supports_memory_index && access.read {
        access.read = false;
        unsupported_override = true;
    }
    if !capabilities.supports_automatic_extraction && access.automatic_extraction {
        access.automatic_extraction = false;
        unsupported_override = true;
    }
    if unsupported_override {
        warnings.push(PersonalizationWarning::new(
            PersonalizationWarningCode::UnsupportedCapabilityOverride,
        ));
        exclusions.push(PersonalizationExclusion {
            reason: PersonalizationExclusionReason::RuntimeCapability,
            count: 0,
        });
    }
}

fn apply_maintenance(
    maintenance: MaintenanceState,
    access: &mut EffectiveMemoryAccess,
    warnings: &mut Vec<PersonalizationWarning>,
    exclusions: &mut Vec<PersonalizationExclusion>,
) {
    if !maintenance.migration_complete {
        *access = EffectiveMemoryAccess::denied();
        warnings.push(PersonalizationWarning::new(
            PersonalizationWarningCode::MigrationIncomplete,
        ));
        exclusions.push(PersonalizationExclusion {
            reason: PersonalizationExclusionReason::UnsafeMaintenanceState,
            count: 0,
        });
    }
    if maintenance.repair_required {
        // Repair is a per-record condition; an unprojected memory is excluded individually rather
        // than blinding the whole session.
        warnings.push(PersonalizationWarning::new(
            PersonalizationWarningCode::RepairRequired,
        ));
    }
}

/// A stable, safe fingerprint of everything that decided this snapshot.
///
/// Hashes identities, revisions, and modes — never instruction or memory text. The token is
/// diagnostic metadata that reaches logs and the frontend, so anything hashed into it must be
/// something we would be willing to correlate across records.
fn revision_token(
    context: &PersonalizationResolutionContext,
    layers: &PersonalizationLayers,
    maintenance: MaintenanceState,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"personalization-snapshot-v1");
    hasher.update(b"\x1fagent=");
    hasher.update(context.agent_id.as_str().as_bytes());
    hasher.update(b"\x1fmode=");
    hasher.update(context.session_mode.as_str().as_bytes());
    hasher.update(b"\x1fworkspace=");
    hasher.update(
        context
            .workspace
            .as_ref()
            .map(|workspace| workspace.key().as_str())
            .unwrap_or("-")
            .as_bytes(),
    );
    hasher.update(b"\x1fmigration=");
    hasher.update(maintenance.migration_generation.to_string().as_bytes());
    for layer in layers.durable_layers() {
        hasher.update(b"\x1flayer=");
        hasher.update(layer.scope().scope_key().as_bytes());
        hasher.update(b"@");
        hasher.update(layer.revision().to_string().as_bytes());
    }
    if let Some(session_override) = layers.session_override.as_ref() {
        hasher.update(b"\x1fsession-override=");
        hasher.update(session_override_fingerprint(session_override).as_bytes());
    }
    let digest = hasher.finalize();
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Modes and toggles are safe to fingerprint directly; the override's instruction *text* is not,
/// so only its presence is recorded.
fn session_override_fingerprint(patch: &PersonalizationPolicyPatch) -> String {
    fn toggle(value: Option<PolicyToggle>) -> &'static str {
        value.map(PolicyToggle::as_str).unwrap_or("-")
    }
    format!(
        "{}:{}:{}:{}:{}:{}:{}",
        patch
            .instruction_merge_mode
            .map(InstructionMergeMode::as_str)
            .unwrap_or("-"),
        u8::from(patch.about_user.is_some()),
        u8::from(patch.style_rules.is_some()),
        toggle(patch.memory_read_mode),
        toggle(patch.explicit_save_mode),
        toggle(patch.automatic_extraction_mode),
        toggle(patch.global_memory_access_mode),
    )
}
