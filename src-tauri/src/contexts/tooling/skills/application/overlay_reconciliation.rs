#![cfg_attr(not(test), allow(dead_code))]

use super::{
    build_overlay_diff, replay_conflicts, OverlayApplicationError, OverlayBoundedText,
    OverlayConflictResolution, OverlayConflictSummary, OverlayEffectivePackageSnapshot,
    OverlayManifestSnapshot, OverlayPinSnapshot, OverlayReconciliationBaseSnapshot,
    OverlayReconciliationConflictChoice, OverlayReconciliationPreview,
    OverlayReconciliationPreviewInput, OverlayReconciliationProposedResult,
    OverlayReconciliationRequest, OverlayResourceShadow, OverlayResourceSummary,
    SkillApplicationError,
};
use crate::contexts::tooling::skills::domain::{
    replay_overlay_scope_chain, scan_overlay_text, EffectiveResourceSource, OverlayBaseWitness,
    OverlayConflict, OverlayConflictState, OverlayDocument, OverlayMutationState, OverlayScope,
    OverlayScopeReplay, OverlayScopeReplayInput, OverlayScopeReplayStatus, SkillLayer,
    DEFAULT_OVERLAY_LIMITS,
};
use std::collections::{BTreeMap, BTreeSet};

const MAXIMUM_RECONCILIATION_CONFLICTS: usize = 100;
const MAXIMUM_RECONCILIATION_RESOURCES: usize = 100;
const MAXIMUM_SHADOW_SUMMARIES: usize = 8;

pub(crate) struct OverlayReconciliationInput<'a> {
    pub(crate) request: &'a OverlayReconciliationRequest,
    pub(crate) base: &'a OverlayEffectivePackageSnapshot,
    pub(crate) current: &'a OverlayManifestSnapshot,
    pub(crate) applicable: &'a [OverlayManifestSnapshot],
    pub(crate) active_workspace: Option<&'a str>,
    pub(crate) pin: &'a OverlayPinSnapshot,
    pub(crate) timestamp: &'a str,
}

pub(crate) struct PreparedOverlayReconciliation {
    pub(crate) next_document: OverlayDocument,
    pub(crate) replay: OverlayScopeReplay,
    pub(crate) preview: OverlayReconciliationPreview,
    pub(crate) edited_patch: bool,
}

pub(crate) fn prepare_overlay_reconciliation(
    input: &OverlayReconciliationInput<'_>,
) -> Result<PreparedOverlayReconciliation, SkillApplicationError> {
    validate_reconciliation_context(input)?;
    let current_replay = replay_documents(input.base, input.applicable, input.active_workspace);
    let conflicts = target_conflicts(input.current, &current_replay);
    validate_choice_ids(input.request, &conflicts)?;

    let mut next_document = input.current.document.clone();
    let edited_patch = apply_choices(
        &mut next_document,
        input.request,
        &conflicts,
        &input.base.instruction_hash,
        input.timestamp,
    )?;
    next_document.base_witness = OverlayBaseWitness::new(
        &input.base.base_identity,
        &input.base.instruction_hash,
        &input.base.package_hash,
    )?;
    next_document.advance_revision(&input.current.document_hash, input.timestamp)?;
    finish_conflicts(&mut next_document, input.request, &conflicts)?;

    let next_snapshot = OverlayManifestSnapshot {
        document: next_document.clone(),
        document_hash: "reconciliation-preview".to_string(),
    };
    let next_applicable = replace_target(input.applicable, &next_snapshot);
    let replay = replay_documents(input.base, &next_applicable, input.active_workspace);
    let target_applied = replay.scope_results().iter().any(|result| {
        result.scope() == next_document.scope()
            && result.revision() == next_document.revision()
            && matches!(result.status(), OverlayScopeReplayStatus::Applied)
    });
    let unresolved_choices = conflict_choices(input.request, &conflicts);
    let all_choices_selected = unresolved_choices
        .iter()
        .all(|choice| choice.selected_resolution.is_some());
    let replay_has_conflicts = !replay_conflicts(&replay).is_empty();
    let final_diff_complete = target_applied && all_choices_selected && !replay_has_conflicts;
    let final_diff = build_overlay_diff(
        input.base,
        &replay,
        DEFAULT_OVERLAY_LIMITS.maximum_instruction_characters,
    );
    let mut resources = proposed_resources(&replay);
    let resources_truncated = resources.len() > MAXIMUM_RECONCILIATION_RESOURCES;
    resources.truncate(MAXIMUM_RECONCILIATION_RESOURCES);
    let conflicts_truncated = unresolved_choices.len() > MAXIMUM_RECONCILIATION_CONFLICTS;
    let preview = OverlayReconciliationPreview::from_input(OverlayReconciliationPreviewInput {
        witnesses: input.request.witnesses.clone(),
        witnessed_base: witnessed_base(input),
        current_base: current_base(input.base),
        proposed_effective: OverlayReconciliationProposedResult {
            effective_hash: replay.effective().effective_hash().to_string(),
            instructions: OverlayBoundedText::from_text(
                replay.effective().instructions(),
                DEFAULT_OVERLAY_LIMITS.maximum_instruction_characters,
            ),
            resources,
            resources_truncated,
        },
        conflict_choices: unresolved_choices
            .into_iter()
            .take(MAXIMUM_RECONCILIATION_CONFLICTS)
            .collect(),
        conflicts_truncated,
        final_diff,
        final_diff_complete,
    });
    Ok(PreparedOverlayReconciliation {
        next_document,
        replay,
        preview,
        edited_patch,
    })
}

fn validate_reconciliation_context(
    input: &OverlayReconciliationInput<'_>,
) -> Result<(), SkillApplicationError> {
    let request = input.request;
    let document = &input.current.document;
    let workspace_valid = match request.scope {
        OverlayScope::Project => request.workspace_identity.as_deref() == input.active_workspace,
        OverlayScope::System | OverlayScope::User => request.workspace_identity.is_none(),
    };
    if request.canonical_skill_id != input.base.canonical_skill_id
        || request.canonical_skill_id != document.canonical_skill_id
        || request.scope != document.scope()
        || request.workspace_identity.as_deref() != document.workspace_identity()
        || !workspace_valid
    {
        return Err(invalid("overlay-reconciliation-context-mismatch"));
    }
    if input.pin.pinned {
        return Err(OverlayApplicationError::PinnedRefusal {
            skill_id: request.canonical_skill_id.as_str().to_string(),
        }
        .into());
    }
    let base_changed = document.base_witness.base_identity != input.base.base_identity
        || document.base_witness.instruction_hash != input.base.instruction_hash
        || document.base_witness.package_hash != input.base.package_hash;
    let revision_changed = request.witnesses.expected_overlay_revision != Some(document.revision());
    let request_base_changed = request.witnesses.expected_base_instruction_hash
        != input.base.instruction_hash
        || request.witnesses.expected_base_package_hash != input.base.package_hash;
    let pin_changed = request.witnesses.expected_pinned != input.pin.pinned;
    if revision_changed || request_base_changed || pin_changed {
        return Err(OverlayApplicationError::StaleWitnesses {
            expected_revision: request.witnesses.expected_overlay_revision,
            current_revision: Some(document.revision()),
            base_changed: request_base_changed,
            payload_changed: false,
            pin_changed,
        }
        .into());
    }
    if !base_changed {
        return Err(invalid("overlay-reconciliation-base-unchanged"));
    }
    Ok(())
}

fn target_conflicts(
    current: &OverlayManifestSnapshot,
    replay: &OverlayScopeReplay,
) -> Vec<OverlayConflictSummary> {
    let mut conflicts = current
        .document
        .conflicts
        .iter()
        .filter(|conflict| conflict.state() == OverlayConflictState::Active)
        .map(|conflict| OverlayConflictSummary {
            id: conflict.id().to_string(),
            mutation_id: conflict.mutation_id().to_string(),
            safe_reason: conflict.reason.clone(),
            state: conflict.state(),
            resolution_revision: conflict.resolution_revision(),
        })
        .collect::<Vec<_>>();
    let generated_id = format!(
        "preview-{}-{}",
        current.document.scope().as_str(),
        current.document.revision()
    );
    conflicts.extend(
        replay_conflicts(replay)
            .into_iter()
            .filter(|conflict| conflict.id == generated_id),
    );
    conflicts.sort_by(|left, right| left.id.cmp(&right.id));
    conflicts.dedup_by(|left, right| left.id == right.id);
    conflicts
}

fn validate_choice_ids(
    request: &OverlayReconciliationRequest,
    conflicts: &[OverlayConflictSummary],
) -> Result<(), SkillApplicationError> {
    let known = conflicts
        .iter()
        .map(|conflict| conflict.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut selected = BTreeSet::new();
    for choice in &request.choices {
        if !known.contains(choice.conflict_id.as_str()) || !selected.insert(&choice.conflict_id) {
            return Err(invalid("overlay-reconciliation-conflict-choice-invalid"));
        }
    }
    Ok(())
}

fn apply_choices(
    document: &mut OverlayDocument,
    request: &OverlayReconciliationRequest,
    conflicts: &[OverlayConflictSummary],
    current_base_hash: &str,
    timestamp: &str,
) -> Result<bool, SkillApplicationError> {
    let by_id = conflicts
        .iter()
        .map(|conflict| (conflict.id.as_str(), conflict))
        .collect::<BTreeMap<_, _>>();
    let mut edited_patch = false;
    for choice in &request.choices {
        let conflict = by_id
            .get(choice.conflict_id.as_str())
            .ok_or_else(|| invalid("overlay-reconciliation-conflict-choice-invalid"))?;
        ensure_conflict_record(document, conflict)?;
        match &choice.resolution {
            OverlayConflictResolution::EditPatch {
                old_string,
                new_string,
                replace_all,
            } => {
                scan_reconciled_patch(old_string, new_string)?;
                let patch = document
                    .patches
                    .iter_mut()
                    .find(|patch| patch.id == conflict.mutation_id)
                    .ok_or_else(|| invalid("overlay-reconciliation-edit-target-invalid"))?;
                patch.edit_for_reconciliation(
                    old_string,
                    new_string,
                    *replace_all,
                    current_base_hash,
                    timestamp,
                )?;
                edited_patch = true;
            }
            OverlayConflictResolution::Ignore => {
                disable_mutation(document, &conflict.mutation_id, timestamp)?;
            }
        }
    }
    Ok(edited_patch)
}

fn ensure_conflict_record(
    document: &mut OverlayDocument,
    summary: &OverlayConflictSummary,
) -> Result<(), SkillApplicationError> {
    if document
        .conflicts
        .iter()
        .any(|conflict| conflict.id() == summary.id)
    {
        return Ok(());
    }
    document.conflicts.push(OverlayConflict::new(
        &summary.id,
        &summary.mutation_id,
        &summary.safe_reason,
        &document.base_witness.package_hash,
    )?);
    Ok(())
}

fn finish_conflicts(
    document: &mut OverlayDocument,
    request: &OverlayReconciliationRequest,
    conflicts: &[OverlayConflictSummary],
) -> Result<(), SkillApplicationError> {
    let resolution_by_id = request
        .choices
        .iter()
        .map(|choice| (choice.conflict_id.as_str(), &choice.resolution))
        .collect::<BTreeMap<_, _>>();
    let resolution_revision = document.revision();
    for summary in conflicts {
        let Some(resolution) = resolution_by_id.get(summary.id.as_str()) else {
            continue;
        };
        let conflict = document
            .conflicts
            .iter_mut()
            .find(|conflict| conflict.id() == summary.id)
            .ok_or_else(|| invalid("overlay-reconciliation-conflict-record-missing"))?;
        match resolution {
            OverlayConflictResolution::EditPatch { .. } => conflict.resolve(resolution_revision)?,
            OverlayConflictResolution::Ignore => conflict.ignore(resolution_revision)?,
        }
    }
    Ok(())
}

fn disable_mutation(
    document: &mut OverlayDocument,
    mutation_id: &str,
    timestamp: &str,
) -> Result<(), SkillApplicationError> {
    if let Some(patch) = document
        .patches
        .iter_mut()
        .find(|patch| patch.id == mutation_id)
    {
        patch.disable(timestamp)?;
        return Ok(());
    }
    if let Some(block) = document
        .learn_blocks
        .iter_mut()
        .find(|block| block.id == mutation_id)
    {
        block.disable(timestamp)?;
        return Ok(());
    }
    if let Some(file) = document
        .files
        .iter_mut()
        .find(|file| file.id == mutation_id)
    {
        file.disable(timestamp)?;
        return Ok(());
    }
    Err(invalid("overlay-reconciliation-ignore-target-invalid"))
}

fn scan_reconciled_patch(old_string: &str, new_string: &str) -> Result<(), SkillApplicationError> {
    if old_string.is_empty()
        || !scan_overlay_text(old_string).safe_rule_ids().is_empty()
        || !scan_overlay_text(new_string).safe_rule_ids().is_empty()
    {
        return Err(invalid("overlay-reconciliation-edited-patch-rejected"));
    }
    Ok(())
}

fn replay_documents(
    base: &OverlayEffectivePackageSnapshot,
    applicable: &[OverlayManifestSnapshot],
    active_workspace: Option<&str>,
) -> OverlayScopeReplay {
    let inputs = applicable
        .iter()
        .filter(|snapshot| {
            snapshot
                .document
                .trust()
                .is_trusted_for_revision(snapshot.document.revision())
        })
        .map(|snapshot| {
            let document = &snapshot.document;
            if document.base_witness.base_identity != base.base_identity
                || document.base_witness.instruction_hash != base.instruction_hash
                || document.base_witness.package_hash != base.package_hash
            {
                OverlayScopeReplayInput::base_drift(document)
            } else {
                OverlayScopeReplayInput::verified(document)
            }
        })
        .collect::<Vec<_>>();
    replay_overlay_scope_chain(
        &base.instructions,
        &base.resources,
        &inputs,
        active_workspace,
        MAXIMUM_SHADOW_SUMMARIES,
    )
}

fn replace_target(
    applicable: &[OverlayManifestSnapshot],
    next: &OverlayManifestSnapshot,
) -> Vec<OverlayManifestSnapshot> {
    let mut snapshots = applicable
        .iter()
        .filter(|snapshot| {
            snapshot.document.scope() != next.document.scope()
                || snapshot.document.workspace_identity() != next.document.workspace_identity()
        })
        .cloned()
        .collect::<Vec<_>>();
    snapshots.push(next.clone());
    snapshots.sort_by_key(|snapshot| snapshot.document.scope());
    snapshots
}

fn conflict_choices(
    request: &OverlayReconciliationRequest,
    conflicts: &[OverlayConflictSummary],
) -> Vec<OverlayReconciliationConflictChoice> {
    conflicts
        .iter()
        .map(|conflict| OverlayReconciliationConflictChoice {
            conflict: conflict.clone(),
            selected_resolution: request
                .choices
                .iter()
                .find(|choice| choice.conflict_id == conflict.id)
                .map(|choice| choice.resolution.clone()),
        })
        .collect()
}

fn witnessed_base(input: &OverlayReconciliationInput<'_>) -> OverlayReconciliationBaseSnapshot {
    let witness = &input.current.document.base_witness;
    let layer = witness
        .base_identity
        .split(':')
        .next()
        .and_then(SkillLayer::parse)
        .unwrap_or(input.base.base_layer);
    OverlayReconciliationBaseSnapshot {
        base_identity: witness.base_identity.clone(),
        base_layer: layer,
        instruction_hash: witness.instruction_hash.clone(),
        package_hash: witness.package_hash.clone(),
        instructions: None,
    }
}

fn current_base(base: &OverlayEffectivePackageSnapshot) -> OverlayReconciliationBaseSnapshot {
    OverlayReconciliationBaseSnapshot {
        base_identity: base.base_identity.clone(),
        base_layer: base.base_layer,
        instruction_hash: base.instruction_hash.clone(),
        package_hash: base.package_hash.clone(),
        instructions: Some(OverlayBoundedText::from_text(
            &base.instructions,
            DEFAULT_OVERLAY_LIMITS.maximum_instruction_characters,
        )),
    }
}

fn proposed_resources(replay: &OverlayScopeReplay) -> Vec<OverlayResourceSummary> {
    replay
        .effective()
        .resources()
        .iter()
        .filter_map(|resource| {
            let EffectiveResourceSource::Overlay {
                scope, mutation_id, ..
            } = &resource.source
            else {
                return None;
            };
            Some(OverlayResourceSummary {
                mutation_id: mutation_id.clone(),
                logical_path: resource.logical_path.clone(),
                media_type: resource.media_type.clone(),
                size_bytes: resource.size_bytes,
                content_hash: resource.content_hash.clone(),
                effective_scope: *scope,
                state: OverlayMutationState::Active,
                shadowed: resource
                    .shadowed
                    .iter()
                    .map(|shadow| match &shadow.source {
                        EffectiveResourceSource::Base { layer } => OverlayResourceShadow {
                            scope: None,
                            base_layer: Some(*layer),
                            content_hash: shadow.content_hash.clone(),
                        },
                        EffectiveResourceSource::Overlay { scope, .. } => OverlayResourceShadow {
                            scope: Some(*scope),
                            base_layer: None,
                            content_hash: shadow.content_hash.clone(),
                        },
                    })
                    .collect(),
                shadowed_truncated: resource.shadowed_truncated,
            })
        })
        .collect()
}

fn invalid(code: &str) -> SkillApplicationError {
    OverlayApplicationError::InvalidRequest {
        code: code.to_string(),
    }
    .into()
}
