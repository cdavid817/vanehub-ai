use sha2::{Digest, Sha256};

use super::policy::{
    InstructionMergeMode, PersonalizationPolicyPatch, PersonalizationPolicyRecord, PolicyToggle,
    SessionPersonalizationMode,
};
use super::scope::PersonalizationPolicyScope;
use super::snapshot::{
    EffectiveMemoryAccess, EffectivePersonalizationSnapshot, ExcludedInstructionSegment,
    InstructionExclusionReason, InstructionField, InstructionMergeAction, PersonalizationExclusion,
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

/// What one consistent read found for one scope key.
///
/// `Absent` is a finding, not a gap. A query that failed and a query that proved no override exists
/// look identical if both are represented by a missing entry, and the difference decides whether a
/// cached bundle may be reused: proving there was no workspace override is exactly what lets a
/// later resolution skip re-reading it, while a failed read must never be cached as "none".
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PolicyLayerState {
    Present(PersonalizationPolicyRecord),
    Absent,
}

impl PolicyLayerState {
    pub(crate) fn record(&self) -> Option<&PersonalizationPolicyRecord> {
        match self {
            Self::Present(record) => Some(record),
            Self::Absent => None,
        }
    }
}

/// Every scope key one resolution needs, read together, with each key's finding.
///
/// Ordered by precedence so a reader cannot reconstruct the order wrongly, and complete so that
/// "this key was not asked for" and "this key has no override" are different states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PolicyResolutionBundle {
    /// In precedence order: global, Agent, workspace, workspace-Agent. Workspace keys are absent
    /// from the list entirely when there is no workspace, which is different from present-and-empty.
    pub(crate) layers: Vec<(PersonalizationPolicyScope, PolicyLayerState)>,
}

impl PolicyResolutionBundle {
    pub(crate) fn state(&self, scope: &PersonalizationPolicyScope) -> Option<&PolicyLayerState> {
        self.layers
            .iter()
            .find(|(key, _)| key == scope)
            .map(|(_, state)| state)
    }

    /// Turns the bundle into the layered shape the resolver consumes.
    pub(crate) fn into_layers(self) -> PersonalizationLayers {
        let mut layers = PersonalizationLayers::default();
        for (scope, state) in self.layers {
            let Some(record) = state.record().cloned() else {
                continue;
            };
            match scope {
                PersonalizationPolicyScope::Global => layers.global = Some(record),
                PersonalizationPolicyScope::Agent { .. } => layers.agent = Some(record),
                PersonalizationPolicyScope::Workspace { .. } => layers.workspace = Some(record),
                PersonalizationPolicyScope::WorkspaceAgent { .. } => {
                    layers.workspace_agent = Some(record)
                }
            }
        }
        layers
    }
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

    let (mut effective_instruction_mode, mut instruction_segments, mut excluded_segments) =
        resolve_instructions(&layers);
    let mut access = resolve_toggles(&layers);

    if let Some(session_override) = layers.session_override.as_ref() {
        apply_session_override(
            session_override,
            &context,
            &mut effective_instruction_mode,
            &mut instruction_segments,
            &mut excluded_segments,
            &mut access,
        );
    }

    apply_session_mode(&context, &mut access, &mut exclusions);
    apply_capabilities(
        capabilities,
        &mut instruction_segments,
        &mut excluded_segments,
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
        excluded_instruction_segments: excluded_segments,
        memory_access: access,
        exclusions,
        warnings,
    }
}

/// The instruction merge state machine, layer by layer in precedence order.
///
/// Four transitions, and every one of them is about what happens to the segments *below*:
///
/// - `Inherit` keeps the current state and contributes nothing of its own, so a preview keeps
///   naming the layer that actually decided rather than the last one that ran;
/// - `Append` adds this layer's non-empty fields to whatever survived;
/// - `Replace` clears everything below and then adds this layer's non-empty fields;
/// - `Disabled` clears everything and adds nothing.
///
/// A higher layer may re-establish segments after a lower one disabled or replaced them — the
/// machine has no terminal state — which is why `Disabled` is not a short circuit.
///
/// Core, system, safety, role and runtime instructions never enter here and are never cleared by
/// any of these transitions. This machine only ever sees the two user-authored fields.
fn resolve_instructions(
    layers: &PersonalizationLayers,
) -> (
    InstructionMergeMode,
    Vec<ResolvedInstructionSegment>,
    Vec<ExcludedInstructionSegment>,
) {
    let mut mode = InstructionMergeMode::Disabled;
    let mut segments: Vec<ResolvedInstructionSegment> = Vec::new();
    let mut excluded: Vec<ExcludedInstructionSegment> = Vec::new();
    for layer in layers.durable_layers() {
        match layer.instruction_merge_mode() {
            InstructionMergeMode::Inherit => {
                excluded.extend(inherited_exclusions(layer));
            }
            InstructionMergeMode::Append => {
                mode = InstructionMergeMode::Append;
                let (included, empty) = layer_segments(layer, InstructionMergeAction::Appended);
                segments.extend(included);
                excluded.extend(empty);
            }
            InstructionMergeMode::Replace => {
                mode = InstructionMergeMode::Replace;
                excluded.extend(
                    segments
                        .drain(..)
                        .map(|segment| {
                            segment.excluded_by(InstructionExclusionReason::ReplacedByHigherLayer)
                        })
                        .collect::<Vec<_>>(),
                );
                let (included, empty) = layer_segments(layer, InstructionMergeAction::Replaced);
                segments.extend(included);
                excluded.extend(empty);
            }
            InstructionMergeMode::Disabled => {
                mode = InstructionMergeMode::Disabled;
                excluded.extend(
                    segments
                        .drain(..)
                        .map(|segment| {
                            segment.excluded_by(InstructionExclusionReason::DisabledByHigherLayer)
                        })
                        .collect::<Vec<_>>(),
                );
            }
        }
    }
    (mode, segments, excluded)
}

fn layer_segments(
    layer: &PersonalizationPolicyRecord,
    action: InstructionMergeAction,
) -> (
    Vec<ResolvedInstructionSegment>,
    Vec<ExcludedInstructionSegment>,
) {
    ResolvedInstructionSegment::from_scope(
        layer.scope(),
        layer.revision(),
        action,
        layer.about_user(),
        layer.style_rules(),
    )
}

/// An inheriting layer's own fields, reported as excluded so a preview can say why text stored
/// there is not being used.
fn inherited_exclusions(layer: &PersonalizationPolicyRecord) -> Vec<ExcludedInstructionSegment> {
    [
        (InstructionField::AboutUser, layer.about_user()),
        (InstructionField::StyleRules, layer.style_rules()),
    ]
    .into_iter()
    .filter(|(_, text)| !text.is_empty())
    .map(|(field, _)| ExcludedInstructionSegment {
        field,
        scope_kind: layer.scope().scope_kind(),
        scope_key: layer.scope().scope_key(),
        policy_revision: layer.revision(),
        reason: InstructionExclusionReason::InheritedLayer,
    })
    .collect()
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

#[allow(clippy::too_many_arguments)]
fn apply_session_override(
    patch: &PersonalizationPolicyPatch,
    context: &PersonalizationResolutionContext,
    mode: &mut InstructionMergeMode,
    segments: &mut Vec<ResolvedInstructionSegment>,
    excluded: &mut Vec<ExcludedInstructionSegment>,
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
    let scope_key = format!("session/{}", context.session_id);
    let fields = [
        (
            InstructionField::AboutUser,
            patch.about_user.clone().unwrap_or_default(),
        ),
        (
            InstructionField::StyleRules,
            patch.style_rules.clone().unwrap_or_default(),
        ),
    ];
    // A session override has no durable revision — it lives with the session — so it reports zero
    // rather than borrowing a policy row's number it does not belong to.
    let session_segments = |action: InstructionMergeAction| -> Vec<ResolvedInstructionSegment> {
        fields
            .iter()
            .filter(|(_, text)| !text.is_empty())
            .map(|(field, text)| ResolvedInstructionSegment {
                field: *field,
                scope_kind: "session",
                scope_key: scope_key.clone(),
                policy_revision: 0,
                merge_action: action,
                text: text.clone(),
            })
            .collect()
    };
    let clear = |segments: &mut Vec<ResolvedInstructionSegment>,
                 excluded: &mut Vec<ExcludedInstructionSegment>,
                 reason: InstructionExclusionReason| {
        excluded.extend(
            segments
                .drain(..)
                .map(|segment| segment.excluded_by(reason))
                .collect::<Vec<_>>(),
        );
    };

    match override_mode {
        InstructionMergeMode::Inherit => {}
        InstructionMergeMode::Append => {
            *mode = InstructionMergeMode::Append;
            segments.extend(session_segments(InstructionMergeAction::Appended));
        }
        InstructionMergeMode::Replace => {
            *mode = InstructionMergeMode::Replace;
            clear(
                segments,
                excluded,
                InstructionExclusionReason::ReplacedByHigherLayer,
            );
            segments.extend(session_segments(InstructionMergeAction::Replaced));
        }
        InstructionMergeMode::Disabled => {
            *mode = InstructionMergeMode::Disabled;
            clear(
                segments,
                excluded,
                InstructionExclusionReason::DisabledByHigherLayer,
            );
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
    excluded: &mut Vec<ExcludedInstructionSegment>,
    access: &mut EffectiveMemoryAccess,
    warnings: &mut Vec<PersonalizationWarning>,
    exclusions: &mut Vec<PersonalizationExclusion>,
) {
    let mut unsupported_override = false;
    if !capabilities.supports_custom_instructions && !segments.is_empty() {
        // The resolved policy is left intact for the preview to show; only what is *applied* is
        // emptied, and each dropped field says why. A capability the runtime lacks is not a reason
        // to forget what the user configured.
        excluded.extend(
            segments
                .drain(..)
                .map(|segment| segment.excluded_by(InstructionExclusionReason::RuntimeCapability))
                .collect::<Vec<_>>(),
        );
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
