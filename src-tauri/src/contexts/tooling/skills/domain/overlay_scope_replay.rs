#![cfg_attr(not(test), allow(dead_code))]

use super::{
    apply_overlay_resource_scope, merge_overlay_resources, replay_exact_patches, BaseSkillResource,
    EffectiveSkillResource, ExactPatchConflict, LearnedGuidanceConflict,
    LearnedGuidanceConflictReason, OverlayConflictState, OverlayDocument, OverlayLearnBlock,
    OverlayMutationState, OverlayOrigin, OverlayScope, OverlayTrustState,
    LEARNED_GUIDANCE_END_MARKER, LEARNED_GUIDANCE_HEADING, LEARNED_GUIDANCE_START_MARKER,
};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OverlayIntegrityFailure {
    DocumentHashMismatch,
    PayloadHashMismatch { mutation_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OverlayScopeConflict {
    ExactPatch(ExactPatchConflict),
    LearnedGuidance(LearnedGuidanceConflict),
    Existing { mutation_id: String },
}

impl OverlayScopeConflict {
    pub(crate) fn mutation_id(&self) -> Option<&str> {
        match self {
            Self::ExactPatch(conflict) => Some(&conflict.patch_id),
            Self::LearnedGuidance(conflict) => conflict.block_id.as_deref(),
            Self::Existing { mutation_id } => Some(mutation_id),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OverlayScopeReplayStatus {
    Applied,
    Untrusted,
    NeedsReconciliation,
    Conflict(OverlayScopeConflict),
    IntegrityFailure(OverlayIntegrityFailure),
    Blocked { failed_scope: OverlayScope },
}

#[derive(Clone)]
pub(crate) struct OverlayScopeReplayInput<'a> {
    document: &'a OverlayDocument,
    integrity_failure: Option<OverlayIntegrityFailure>,
    trust_policy: OverlayReplayTrustPolicy,
    base_hash_changed: bool,
}

#[derive(Clone, Copy)]
enum OverlayReplayTrustPolicy {
    Enforce,
    ReviewUntrustedImport,
}

impl<'a> OverlayScopeReplayInput<'a> {
    pub(crate) fn verified(document: &'a OverlayDocument) -> Self {
        Self {
            document,
            integrity_failure: None,
            trust_policy: OverlayReplayTrustPolicy::Enforce,
            base_hash_changed: false,
        }
    }

    pub(crate) fn base_drift(document: &'a OverlayDocument) -> Self {
        Self {
            document,
            integrity_failure: None,
            trust_policy: OverlayReplayTrustPolicy::Enforce,
            base_hash_changed: true,
        }
    }

    pub(crate) fn untrusted_import_review(document: &'a OverlayDocument) -> Option<Self> {
        (document.trust().origin() == OverlayOrigin::Imported
            && document.trust().state() == OverlayTrustState::Untrusted)
            .then_some(Self {
                document,
                integrity_failure: None,
                trust_policy: OverlayReplayTrustPolicy::ReviewUntrustedImport,
                base_hash_changed: false,
            })
    }

    pub(crate) fn integrity_failure(
        document: &'a OverlayDocument,
        failure: OverlayIntegrityFailure,
    ) -> Self {
        Self {
            document,
            integrity_failure: Some(failure),
            trust_policy: OverlayReplayTrustPolicy::Enforce,
            base_hash_changed: false,
        }
    }

    fn is_replay_eligible(&self) -> bool {
        match self.trust_policy {
            OverlayReplayTrustPolicy::Enforce => self
                .document
                .trust()
                .is_trusted_for_revision(self.document.revision()),
            OverlayReplayTrustPolicy::ReviewUntrustedImport => {
                self.document.trust().origin() == OverlayOrigin::Imported
                    && self.document.trust().state() == OverlayTrustState::Untrusted
            }
        }
    }

    fn applies_to(&self, active_workspace: Option<&str>) -> bool {
        self.document.scope() != OverlayScope::Project
            || active_workspace
                .is_some_and(|workspace| self.document.workspace_identity() == Some(workspace))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectiveOverlaySnapshot {
    instructions: String,
    resources: Vec<EffectiveSkillResource>,
    instruction_hash: String,
    effective_hash: String,
}

impl EffectiveOverlaySnapshot {
    fn new(instructions: String, resources: Vec<EffectiveSkillResource>) -> Self {
        let instruction_hash = sha256(instructions.as_bytes());
        let effective_hash = effective_hash(&instructions, &resources);
        Self {
            instructions,
            resources,
            instruction_hash,
            effective_hash,
        }
    }

    pub(crate) fn instructions(&self) -> &str {
        &self.instructions
    }

    pub(crate) fn instruction_hash(&self) -> &str {
        &self.instruction_hash
    }

    pub(crate) fn effective_hash(&self) -> &str {
        &self.effective_hash
    }

    pub(crate) fn resources(&self) -> &[EffectiveSkillResource] {
        &self.resources
    }

    pub(crate) fn resource(&self, logical_path: &str) -> Option<&EffectiveSkillResource> {
        self.resources
            .binary_search_by(|resource| resource.logical_path.as_str().cmp(logical_path))
            .ok()
            .map(|index| &self.resources[index])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayScopeReplayResult {
    scope: OverlayScope,
    revision: u64,
    status: OverlayScopeReplayStatus,
    input_hash: String,
    output: Option<EffectiveOverlaySnapshot>,
    last_healthy_hash: String,
}

impl OverlayScopeReplayResult {
    pub(crate) fn scope(&self) -> OverlayScope {
        self.scope
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn status(&self) -> &OverlayScopeReplayStatus {
        &self.status
    }

    pub(crate) fn input_hash(&self) -> &str {
        &self.input_hash
    }

    pub(crate) fn output(&self) -> Option<&EffectiveOverlaySnapshot> {
        self.output.as_ref()
    }

    pub(crate) fn output_hash(&self) -> Option<&str> {
        self.output
            .as_ref()
            .map(EffectiveOverlaySnapshot::effective_hash)
    }

    pub(crate) fn last_healthy_hash(&self) -> &str {
        &self.last_healthy_hash
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayScopeReplay {
    base: EffectiveOverlaySnapshot,
    scope_results: Vec<OverlayScopeReplayResult>,
    effective: EffectiveOverlaySnapshot,
}

impl OverlayScopeReplay {
    pub(crate) fn base(&self) -> &EffectiveOverlaySnapshot {
        &self.base
    }

    pub(crate) fn scope_results(&self) -> &[OverlayScopeReplayResult] {
        &self.scope_results
    }

    pub(crate) fn effective(&self) -> &EffectiveOverlaySnapshot {
        &self.effective
    }
}

pub(crate) fn replay_overlay_scope_chain(
    base_instructions: &str,
    base_resources: &[BaseSkillResource],
    scope_inputs: &[OverlayScopeReplayInput<'_>],
    active_workspace: Option<&str>,
    maximum_shadow_summaries: usize,
) -> OverlayScopeReplay {
    let base_resource_replay = merge_overlay_resources(
        base_resources,
        &[],
        active_workspace,
        maximum_shadow_summaries,
    );
    let base = EffectiveOverlaySnapshot::new(
        base_instructions.to_string(),
        base_resource_replay.resources().to_vec(),
    );
    let mut effective = base.clone();
    let mut results = Vec::new();
    let mut blocked_by = None;
    let mut ordered_inputs = scope_inputs
        .iter()
        .filter(|input| input.applies_to(active_workspace))
        .collect::<Vec<_>>();
    ordered_inputs.sort_by_key(|input| input.document.scope());

    for input in ordered_inputs {
        let input_hash = effective.effective_hash.clone();
        let scope = input.document.scope();
        if let Some(failed_scope) = blocked_by {
            results.push(failed_result(
                input,
                input_hash,
                OverlayScopeReplayStatus::Blocked { failed_scope },
                &effective,
            ));
            continue;
        }
        if !input.is_replay_eligible() {
            results.push(failed_result(
                input,
                input_hash,
                OverlayScopeReplayStatus::Untrusted,
                &effective,
            ));
            continue;
        }
        if let Some(failure) = &input.integrity_failure {
            results.push(failed_result(
                input,
                input_hash,
                OverlayScopeReplayStatus::IntegrityFailure(failure.clone()),
                &effective,
            ));
            blocked_by = Some(scope);
            continue;
        }
        if let Some(conflict) = input
            .document
            .conflicts
            .iter()
            .find(|conflict| conflict.state() == OverlayConflictState::Active)
        {
            results.push(failed_result(
                input,
                input_hash,
                OverlayScopeReplayStatus::Conflict(OverlayScopeConflict::Existing {
                    mutation_id: conflict.mutation_id().to_string(),
                }),
                &effective,
            ));
            blocked_by = Some(scope);
            continue;
        }

        if input.base_hash_changed {
            match replay_scope(&effective, input.document, maximum_shadow_summaries) {
                Ok(tentative) => {
                    results.push(OverlayScopeReplayResult {
                        scope,
                        revision: input.document.revision(),
                        status: OverlayScopeReplayStatus::NeedsReconciliation,
                        input_hash,
                        output: Some(tentative),
                        last_healthy_hash: effective.effective_hash.clone(),
                    });
                    blocked_by = Some(scope);
                }
                Err(conflict) => {
                    results.push(failed_result(
                        input,
                        input_hash,
                        OverlayScopeReplayStatus::Conflict(conflict),
                        &effective,
                    ));
                    blocked_by = Some(scope);
                }
            }
            continue;
        }

        match replay_scope(&effective, input.document, maximum_shadow_summaries) {
            Ok(output) => {
                results.push(OverlayScopeReplayResult {
                    scope,
                    revision: input.document.revision(),
                    status: OverlayScopeReplayStatus::Applied,
                    input_hash,
                    last_healthy_hash: output.effective_hash.clone(),
                    output: Some(output.clone()),
                });
                effective = output;
            }
            Err(conflict) => {
                results.push(failed_result(
                    input,
                    input_hash,
                    OverlayScopeReplayStatus::Conflict(conflict),
                    &effective,
                ));
                blocked_by = Some(scope);
            }
        }
    }

    OverlayScopeReplay {
        base,
        scope_results: results,
        effective,
    }
}

fn replay_scope(
    current: &EffectiveOverlaySnapshot,
    document: &OverlayDocument,
    maximum_shadow_summaries: usize,
) -> Result<EffectiveOverlaySnapshot, OverlayScopeConflict> {
    let patched = replay_exact_patches(&current.instructions, &document.patches)
        .map_err(OverlayScopeConflict::ExactPatch)?;
    let instructions = append_scope_guidance(patched.content(), &document.learn_blocks)
        .map_err(OverlayScopeConflict::LearnedGuidance)?;
    let resources = apply_overlay_resource_scope(
        &current.resources,
        document.scope(),
        document.workspace_identity(),
        &document.files,
        maximum_shadow_summaries,
    );
    Ok(EffectiveOverlaySnapshot::new(
        instructions,
        resources.resources().to_vec(),
    ))
}

fn failed_result(
    input: &OverlayScopeReplayInput<'_>,
    input_hash: String,
    status: OverlayScopeReplayStatus,
    effective: &EffectiveOverlaySnapshot,
) -> OverlayScopeReplayResult {
    OverlayScopeReplayResult {
        scope: input.document.scope(),
        revision: input.document.revision(),
        status,
        input_hash,
        output: None,
        last_healthy_hash: effective.effective_hash.clone(),
    }
}

fn append_scope_guidance(
    content: &str,
    blocks: &[OverlayLearnBlock],
) -> Result<String, LearnedGuidanceConflict> {
    let active_blocks = blocks
        .iter()
        .filter(|block| block.state() == OverlayMutationState::Active)
        .collect::<Vec<_>>();
    for block in &active_blocks {
        if contains_guidance_delimiter(&block.guidance) {
            return Err(LearnedGuidanceConflict {
                block_id: Some(block.id.clone()),
                reason: LearnedGuidanceConflictReason::DelimiterInjection,
            });
        }
    }
    if active_blocks.is_empty() {
        return Ok(content.to_string());
    }

    let delimiter_count = [
        LEARNED_GUIDANCE_START_MARKER,
        LEARNED_GUIDANCE_END_MARKER,
        LEARNED_GUIDANCE_HEADING,
    ]
    .into_iter()
    .map(|marker| content.match_indices(marker).count())
    .collect::<Vec<_>>();
    if delimiter_count.iter().all(|count| *count == 0) {
        let mut rendered = content.to_string();
        rendered.push_str("\n\n");
        rendered.push_str(LEARNED_GUIDANCE_START_MARKER);
        rendered.push('\n');
        rendered.push_str(LEARNED_GUIDANCE_HEADING);
        append_blocks(&mut rendered, &active_blocks);
        rendered.push('\n');
        rendered.push_str(LEARNED_GUIDANCE_END_MARKER);
        return Ok(rendered);
    }
    if !delimiter_count.iter().all(|count| *count == 1) {
        return Err(existing_delimiter_conflict());
    }

    let Some(start_index) = content.find(LEARNED_GUIDANCE_START_MARKER) else {
        return Err(existing_delimiter_conflict());
    };
    let Some(heading_index) = content.find(LEARNED_GUIDANCE_HEADING) else {
        return Err(existing_delimiter_conflict());
    };
    let Some(end_index) = content.find(LEARNED_GUIDANCE_END_MARKER) else {
        return Err(existing_delimiter_conflict());
    };
    if !(start_index < heading_index && heading_index < end_index) {
        return Err(existing_delimiter_conflict());
    }
    let mut rendered = content[..end_index].to_string();
    append_blocks(&mut rendered, &active_blocks);
    rendered.push('\n');
    rendered.push_str(&content[end_index..]);
    Ok(rendered)
}

fn append_blocks(rendered: &mut String, blocks: &[&OverlayLearnBlock]) {
    for block in blocks {
        rendered.push_str("\n\n");
        rendered.push_str(&block.guidance);
    }
}

fn contains_guidance_delimiter(value: &str) -> bool {
    value.contains(LEARNED_GUIDANCE_START_MARKER)
        || value.contains(LEARNED_GUIDANCE_END_MARKER)
        || value.contains(LEARNED_GUIDANCE_HEADING)
}

fn existing_delimiter_conflict() -> LearnedGuidanceConflict {
    LearnedGuidanceConflict {
        block_id: None,
        reason: LearnedGuidanceConflictReason::DelimiterAlreadyPresent,
    }
}

fn effective_hash(instructions: &str, resources: &[EffectiveSkillResource]) -> String {
    let mut hasher = Sha256::new();
    update_hash_field(&mut hasher, instructions.as_bytes());
    for resource in resources {
        update_hash_field(&mut hasher, resource.logical_path.as_bytes());
        update_hash_field(&mut hasher, resource.media_type.as_bytes());
        update_hash_field(&mut hasher, &resource.size_bytes.to_le_bytes());
        update_hash_field(&mut hasher, resource.content_hash.as_bytes());
    }
    hex_digest(hasher.finalize().as_ref())
}

fn sha256(value: &[u8]) -> String {
    hex_digest(Sha256::digest(value).as_ref())
}

fn hex_digest(value: &[u8]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn update_hash_field(hasher: &mut Sha256, value: &[u8]) {
    hasher.update(value.len().to_le_bytes());
    hasher.update(value);
}
