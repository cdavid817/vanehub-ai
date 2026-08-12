use super::overlay_dto as dto;
use super::overlay_error::OverlayCommandError;
use crate::contexts::tooling::skills::api::OverlayMutationOperation;
use crate::contexts::tooling::skills::application as model;
use crate::contexts::tooling::skills::domain::{
    OverlayConflictState, OverlayMutationState, OverlayScope, OverlayTrustState, SkillId,
    SkillLayer,
};

pub(crate) fn target(
    input: dto::OverlayTargetInput,
) -> Result<(SkillId, OverlayScope, Option<String>), OverlayCommandError> {
    let skill_id = SkillId::parse(&input.skill_id)
        .map_err(|error| OverlayCommandError::validation(error.to_string()))?;
    let scope = OverlayScope::parse(&input.scope)
        .ok_or_else(|| OverlayCommandError::validation("Unknown Overlay scope"))?;
    let workspace = input
        .workspace_path
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if scope == OverlayScope::Project && workspace.is_none() {
        return Err(OverlayCommandError::validation(
            "Project Overlay scope requires one workspace path",
        ));
    }
    let workspace_identity = (scope == OverlayScope::Project)
        .then_some(workspace)
        .flatten();
    Ok((skill_id, scope, workspace_identity))
}

fn witnesses(input: dto::OverlayWitnessesInput) -> model::OverlayWitnesses {
    model::OverlayWitnesses {
        expected_overlay_revision: input.expected_overlay_revision,
        expected_base_instruction_hash: input.expected_base_instruction_hash,
        expected_base_package_hash: input.expected_base_package_hash,
        expected_payload_hash: input.expected_payload_hash,
        expected_pinned: input.expected_pinned,
    }
}

fn request(
    target: dto::OverlayTargetInput,
    witnesses_input: dto::OverlayWitnessesInput,
    mutation: model::OverlayMutation,
) -> Result<model::OverlayMutationRequest, OverlayCommandError> {
    let (canonical_skill_id, scope, workspace_identity) = self::target(target)?;
    Ok(model::OverlayMutationRequest {
        canonical_skill_id,
        scope,
        workspace_identity,
        witnesses: witnesses(witnesses_input),
        mutation,
    })
}

pub(crate) fn patch(
    input: dto::OverlayPatchInput,
) -> Result<model::OverlayMutationRequest, OverlayCommandError> {
    request(
        input.target,
        input.witnesses,
        model::OverlayMutation::ExactPatch {
            old_string: input.old_string,
            new_string: input.new_string,
            replace_all: input.replace_all,
        },
    )
}

pub(crate) fn guidance(
    input: dto::OverlayGuidanceInput,
) -> Result<model::OverlayMutationRequest, OverlayCommandError> {
    request(
        input.target,
        input.witnesses,
        model::OverlayMutation::LearnedGuidance {
            guidance: input.guidance,
        },
    )
}

pub(crate) fn file(
    input: dto::OverlayFileInput,
) -> Result<model::OverlayMutationRequest, OverlayCommandError> {
    request(
        input.target,
        input.witnesses,
        model::OverlayMutation::SupportingFile {
            logical_path: input.logical_path,
            media_type: input.media_type,
            content: input.content,
        },
    )
}

fn mutation_state(
    input: dto::OverlayMutationStateInput,
    revert: bool,
) -> Result<model::OverlayMutationRequest, OverlayCommandError> {
    let mutation = if revert {
        model::OverlayMutation::Revert {
            mutation_id: input.mutation_id,
        }
    } else {
        model::OverlayMutation::Disable {
            mutation_id: input.mutation_id,
        }
    };
    request(input.target, input.witnesses, mutation)
}

fn mutation_operation(
    kind: &str,
    patch: OverlayMutationOperation,
    guidance: OverlayMutationOperation,
    file: OverlayMutationOperation,
) -> Result<OverlayMutationOperation, OverlayCommandError> {
    match kind {
        "patch" => Ok(patch),
        "learnedGuidance" => Ok(guidance),
        "supportingFile" => Ok(file),
        _ => Err(OverlayCommandError::validation(
            "Unknown Overlay mutation kind",
        )),
    }
}

pub(crate) fn disable_mutation_state(
    input: dto::OverlayMutationStateInput,
) -> Result<(model::OverlayMutationRequest, OverlayMutationOperation), OverlayCommandError> {
    let operation = mutation_operation(
        &input.mutation_kind,
        OverlayMutationOperation::DisablePatch,
        OverlayMutationOperation::DisableGuidance,
        OverlayMutationOperation::DisableFile,
    )?;
    Ok((mutation_state(input, false)?, operation))
}

pub(crate) fn revert_mutation_state(
    input: dto::OverlayMutationStateInput,
) -> Result<(model::OverlayMutationRequest, OverlayMutationOperation), OverlayCommandError> {
    let operation = mutation_operation(
        &input.mutation_kind,
        OverlayMutationOperation::RevertPatch,
        OverlayMutationOperation::RevertGuidance,
        OverlayMutationOperation::RevertFile,
    )?;
    Ok((mutation_state(input, true)?, operation))
}

pub(crate) fn preview(
    input: dto::OverlayPreviewInput,
) -> Result<model::OverlayMutationRequest, OverlayCommandError> {
    let mutation = match input.mutation {
        dto::OverlayMutationInput::ExactPatch {
            old_string,
            new_string,
            replace_all,
        } => model::OverlayMutation::ExactPatch {
            old_string,
            new_string,
            replace_all,
        },
        dto::OverlayMutationInput::LearnedGuidance { guidance } => {
            model::OverlayMutation::LearnedGuidance { guidance }
        }
        dto::OverlayMutationInput::SupportingFile {
            logical_path,
            media_type,
            content,
        } => model::OverlayMutation::SupportingFile {
            logical_path,
            media_type,
            content,
        },
        dto::OverlayMutationInput::Disable { mutation_id } => {
            model::OverlayMutation::Disable { mutation_id }
        }
        dto::OverlayMutationInput::Revert { mutation_id } => {
            model::OverlayMutation::Revert { mutation_id }
        }
    };
    request(input.target, input.witnesses, mutation)
}

pub(crate) fn import(
    input: dto::OverlayImportInput,
) -> Result<model::OverlayImportRequest, OverlayCommandError> {
    let (canonical_skill_id, scope, workspace_identity) = target(input.target)?;
    Ok(model::OverlayImportRequest {
        canonical_skill_id,
        scope,
        workspace_identity,
        source_name: input.source_name,
        archive: input.archive,
        witnesses: witnesses(input.witnesses),
    })
}

pub(crate) fn promotion(
    input: dto::OverlayPromotionInput,
) -> Result<model::OverlayPromotionRequest, OverlayCommandError> {
    let (canonical_skill_id, scope, workspace_identity) = target(input.target)?;
    Ok(model::OverlayPromotionRequest {
        canonical_skill_id,
        scope,
        workspace_identity,
        reviewed_revision: input.reviewed_revision,
        reviewed_document_hash: input.reviewed_document_hash,
        reviewed_scan: scan_from_dto(input.reviewed_scan),
        witnesses: witnesses(input.witnesses),
    })
}

pub(crate) fn reconciliation(
    input: dto::OverlayReconciliationInput,
) -> Result<model::OverlayReconciliationRequest, OverlayCommandError> {
    let (canonical_skill_id, scope, workspace_identity) = target(input.target)?;
    let choices = input
        .choices
        .into_iter()
        .map(|choice| {
            let resolution = match choice.resolution.as_str() {
                "ignore" => model::OverlayConflictResolution::Ignore,
                "editPatch" => model::OverlayConflictResolution::EditPatch {
                    old_string: choice.old_string.ok_or_else(|| {
                        OverlayCommandError::validation("Edited conflict requires oldString")
                    })?,
                    new_string: choice.new_string.ok_or_else(|| {
                        OverlayCommandError::validation("Edited conflict requires newString")
                    })?,
                    replace_all: choice.replace_all.unwrap_or(false),
                },
                _ => {
                    return Err(OverlayCommandError::validation(
                        "Unknown conflict resolution",
                    ))
                }
            };
            Ok(model::OverlayReconciliationChoice {
                conflict_id: choice.conflict_id,
                resolution,
            })
        })
        .collect::<Result<Vec<_>, OverlayCommandError>>()?;
    Ok(model::OverlayReconciliationRequest {
        canonical_skill_id,
        scope,
        workspace_identity,
        witnesses: witnesses(input.witnesses),
        choices,
    })
}

pub(crate) fn summary(value: model::OverlaySummary) -> dto::OverlaySummary {
    dto::OverlaySummary {
        canonical_skill_id: value.canonical_skill_id.as_str().to_string(),
        base_layer: layer(value.base_layer),
        status: status(value.status),
        needs_reconcile: value.needs_reconcile,
        pinned: value.pinned,
        base_instruction_hash: value.base_instruction_hash,
        base_package_hash: value.base_package_hash,
        effective_hash: value.effective_hash,
        last_healthy_scope: value.last_healthy_scope.map(scope),
        scopes: value.scopes.into_iter().map(scope_summary).collect(),
        scopes_truncated: value.scopes_truncated,
    }
}

pub(crate) fn detail(value: model::OverlayDetail) -> dto::OverlayDetail {
    dto::OverlayDetail {
        summary: summary(value.summary),
        base_instructions: bounded(value.base_instructions),
        effective_instructions: bounded(value.effective_instructions),
        diff: diff(value.diff),
        scope_diffs: value.scope_diffs.into_iter().map(scope_diff).collect(),
        scope_diffs_truncated: value.scope_diffs_truncated,
        mutations: value.mutations.into_iter().map(mutation_summary).collect(),
        mutations_truncated: value.mutations_truncated,
        resources: value.resources.into_iter().map(resource).collect(),
        resources_truncated: value.resources_truncated,
        conflicts: value.conflicts.into_iter().map(conflict).collect(),
        conflicts_truncated: value.conflicts_truncated,
    }
}

fn scope_diff(value: model::OverlayScopeDiff) -> dto::OverlayScopeDiff {
    dto::OverlayScopeDiff {
        scope: scope(value.scope),
        revision: value.revision,
        input_hash: value.input_hash,
        output_hash: value.output_hash,
        diff: diff(value.diff),
    }
}

pub(crate) fn preview_to_dto(value: model::OverlayPreview) -> dto::OverlayPreview {
    dto::OverlayPreview {
        witnesses: witnesses_to_dto(value.witnesses),
        tentative_revision: value.tentative_revision,
        scan: scan(value.scan),
        diff: diff(value.diff),
        conflicts: value.conflicts.into_iter().map(conflict).collect(),
        conflicts_truncated: value.conflicts_truncated,
        can_commit: value.can_commit,
    }
}

pub(crate) fn outcome(value: model::OverlayMutationOutcome) -> dto::OverlayMutationOutcome {
    dto::OverlayMutationOutcome {
        summary: summary(value.summary),
        committed_revision: value.committed_revision,
        diff: diff(value.diff),
    }
}

pub(crate) fn import_review(value: model::OverlayImportReview) -> dto::OverlayImportReview {
    dto::OverlayImportReview {
        source_summary: value.source_summary,
        revision: value.revision,
        document_hash: value.document_hash,
        scan: scan(value.scan),
        diff: diff(value.diff),
        mutations: value.mutations.into_iter().map(mutation_summary).collect(),
        mutations_truncated: value.mutations_truncated,
        resources: value.resources.into_iter().map(resource).collect(),
        resources_truncated: value.resources_truncated,
        conflicts: value.conflicts.into_iter().map(conflict).collect(),
        conflicts_truncated: value.conflicts_truncated,
    }
}

pub(crate) fn history(value: model::OverlayHistoryPage) -> dto::OverlayHistoryPage {
    dto::OverlayHistoryPage {
        entries: value
            .entries
            .into_iter()
            .map(|entry| dto::OverlayHistoryEntry {
                event_id: entry.event_id,
                canonical_skill_id: entry.canonical_skill_id.as_str().to_string(),
                scope: scope(entry.scope),
                prior_revision: entry.prior_revision,
                next_revision: entry.next_revision,
                actor: match entry.actor {
                    model::OverlayActor::User => "user",
                    model::OverlayActor::System => "system",
                }
                .to_string(),
                action: history_action(entry.action),
                timestamp: entry.timestamp,
                prior_document_hash: entry.prior_document_hash,
                next_document_hash: entry.next_document_hash,
                scanner_version: entry.scanner_version,
                safe_outcome: entry.safe_outcome,
                prior_event_hash: entry.prior_event_hash,
                event_hash: entry.event_hash,
            })
            .collect(),
        next_cursor: value.next_cursor,
        integrity: match value.integrity {
            model::OverlayPageIntegrity::Verified => "verified".to_string(),
            model::OverlayPageIntegrity::Failed(code) => {
                format!("failed:{}", code.as_str())
            }
        },
    }
}

pub(crate) fn reconciliation_preview(
    value: model::OverlayReconciliationPreview,
) -> dto::OverlayReconciliationPreview {
    dto::OverlayReconciliationPreview {
        witnesses: witnesses_to_dto(value.witnesses),
        witnessed_base: reconciliation_base(value.witnessed_base),
        current_base: reconciliation_base(value.current_base),
        proposed_effective: dto::OverlayReconciliationProposedResult {
            effective_hash: value.proposed_effective.effective_hash,
            instructions: bounded(value.proposed_effective.instructions),
            resources: value
                .proposed_effective
                .resources
                .into_iter()
                .map(resource)
                .collect(),
            resources_truncated: value.proposed_effective.resources_truncated,
        },
        conflict_choices: value
            .conflict_choices
            .into_iter()
            .map(|choice| dto::OverlayReconciliationConflictChoice {
                conflict: conflict(choice.conflict),
                selected_resolution: choice.selected_resolution.map(
                    |resolution| match resolution {
                        model::OverlayConflictResolution::EditPatch { .. } => {
                            "editPatch".to_string()
                        }
                        model::OverlayConflictResolution::Ignore => "ignore".to_string(),
                    },
                ),
            })
            .collect(),
        conflicts_truncated: value.conflicts_truncated,
        final_diff: diff(value.final_diff),
        final_diff_complete: value.final_diff_complete,
        can_commit: value.can_commit,
    }
}

fn reconciliation_base(
    value: model::OverlayReconciliationBaseSnapshot,
) -> dto::OverlayReconciliationBaseSnapshot {
    dto::OverlayReconciliationBaseSnapshot {
        base_identity: value.base_identity,
        base_layer: layer(value.base_layer),
        instruction_hash: value.instruction_hash,
        package_hash: value.package_hash,
        instructions: value.instructions.map(bounded),
    }
}

fn bounded(value: model::OverlayBoundedText) -> dto::OverlayBoundedText {
    dto::OverlayBoundedText {
        content: value.content,
        total_characters: value.total_characters,
        truncated: value.truncated,
    }
}

fn diff(value: model::OverlayDiff) -> dto::OverlayDiff {
    dto::OverlayDiff {
        base_hash: value.base_hash,
        effective_hash: value.effective_hash,
        added_characters: value.added_characters,
        removed_characters: value.removed_characters,
        hunks: value
            .hunks
            .into_iter()
            .map(|hunk| dto::OverlayDiffHunk {
                label: hunk.label,
                before: bounded(hunk.before),
                after: bounded(hunk.after),
            })
            .collect(),
        hunks_truncated: value.hunks_truncated,
    }
}

fn scope_summary(value: model::OverlayScopeSummary) -> dto::OverlayScopeSummary {
    dto::OverlayScopeSummary {
        scope: scope(value.scope),
        revision: value.revision,
        trust: trust(value.trust),
        status: scope_status(value.status),
        active_mutation_count: value.active_mutation_count,
        conflict_count: value.conflict_count,
        base_hash_changed: value.base_hash_changed,
        needs_reconcile: value.needs_reconcile,
    }
}

fn mutation_summary(value: model::OverlayMutationSummary) -> dto::OverlayMutationSummary {
    dto::OverlayMutationSummary {
        id: value.id,
        kind: match value.kind {
            model::OverlayMutationKind::Patch => "patch",
            model::OverlayMutationKind::LearnedGuidance => "learnedGuidance",
            model::OverlayMutationKind::SupportingFile => "supportingFile",
        }
        .to_string(),
        scope: scope(value.scope),
        state: mutation_state_name(value.state),
        created_at: value.created_at,
        updated_at: value.updated_at,
    }
}

fn conflict(value: model::OverlayConflictSummary) -> dto::OverlayConflictSummary {
    dto::OverlayConflictSummary {
        id: value.id,
        mutation_id: value.mutation_id,
        safe_reason: value.safe_reason,
        state: match value.state {
            OverlayConflictState::Active => "active",
            OverlayConflictState::Resolved => "resolved",
            OverlayConflictState::Ignored => "ignored",
        }
        .to_string(),
        resolution_revision: value.resolution_revision,
    }
}

fn resource(value: model::OverlayResourceSummary) -> dto::OverlayResourceSummary {
    dto::OverlayResourceSummary {
        mutation_id: value.mutation_id,
        logical_path: value.logical_path,
        media_type: value.media_type,
        size_bytes: value.size_bytes,
        content_hash: value.content_hash,
        effective_scope: scope(value.effective_scope),
        state: mutation_state_name(value.state),
        shadowed: value
            .shadowed
            .into_iter()
            .map(|shadow| dto::OverlayResourceShadow {
                scope: shadow.scope.map(scope),
                base_layer: shadow.base_layer.map(layer),
                content_hash: shadow.content_hash,
            })
            .collect(),
        shadowed_truncated: value.shadowed_truncated,
    }
}

fn scan(value: model::OverlayScanResult) -> dto::OverlayScanResult {
    dto::OverlayScanResult {
        scanner_version: value.scanner_version,
        passed: value.passed,
        safe_rule_ids: value.safe_rule_ids,
        rule_ids_truncated: value.rule_ids_truncated,
    }
}

fn scan_from_dto(value: dto::OverlayScanResult) -> model::OverlayScanResult {
    model::OverlayScanResult {
        scanner_version: value.scanner_version,
        passed: value.passed,
        safe_rule_ids: value.safe_rule_ids,
        rule_ids_truncated: value.rule_ids_truncated,
    }
}

fn witnesses_to_dto(value: model::OverlayWitnesses) -> dto::OverlayWitnessesInputOutput {
    dto::OverlayWitnessesInputOutput {
        expected_overlay_revision: value.expected_overlay_revision,
        expected_base_instruction_hash: value.expected_base_instruction_hash,
        expected_base_package_hash: value.expected_base_package_hash,
        expected_payload_hash: value.expected_payload_hash,
        expected_pinned: value.expected_pinned,
    }
}

fn scope(value: OverlayScope) -> String {
    value.as_str().to_string()
}

fn layer(value: SkillLayer) -> String {
    value.as_str().to_string()
}

fn trust(value: OverlayTrustState) -> String {
    match value {
        OverlayTrustState::Trusted => "trusted",
        OverlayTrustState::Untrusted => "untrusted",
    }
    .to_string()
}

fn mutation_state_name(value: OverlayMutationState) -> String {
    match value {
        OverlayMutationState::Active => "active",
        OverlayMutationState::Disabled => "disabled",
        OverlayMutationState::Reverted => "reverted",
    }
    .to_string()
}

fn status(value: model::OverlayStatus) -> String {
    match value {
        model::OverlayStatus::None => "none",
        model::OverlayStatus::Healthy => "healthy",
        model::OverlayStatus::Untrusted => "untrusted",
        model::OverlayStatus::NeedsReconciliation => "needsReconciliation",
        model::OverlayStatus::Blocked => "blocked",
        model::OverlayStatus::IntegrityFailure => "integrityFailure",
    }
    .to_string()
}

fn scope_status(value: model::OverlayScopeStatus) -> String {
    match value {
        model::OverlayScopeStatus::Applied => "applied",
        model::OverlayScopeStatus::Untrusted => "untrusted",
        model::OverlayScopeStatus::NeedsReconciliation => "needsReconciliation",
        model::OverlayScopeStatus::BlockedByEarlierScope => "blockedByEarlierScope",
        model::OverlayScopeStatus::IntegrityFailure => "integrityFailure",
    }
    .to_string()
}

fn history_action(value: model::OverlayHistoryAction) -> String {
    match value {
        model::OverlayHistoryAction::Create => "create",
        model::OverlayHistoryAction::Patch => "patch",
        model::OverlayHistoryAction::Learn => "learn",
        model::OverlayHistoryAction::File => "file",
        model::OverlayHistoryAction::Import => "import",
        model::OverlayHistoryAction::Promote => "promote",
        model::OverlayHistoryAction::Disable => "disable",
        model::OverlayHistoryAction::Revert => "revert",
        model::OverlayHistoryAction::Reconcile => "reconcile",
        model::OverlayHistoryAction::Conflict => "conflict",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_keeps_active_workspace_separate_from_non_project_identity() {
        let (_, scope, identity) = target(dto::OverlayTargetInput {
            skill_id: "developer".to_string(),
            scope: "user".to_string(),
            workspace_path: Some("C:/work/project".to_string()),
        })
        .expect("valid user target");
        assert_eq!(scope, OverlayScope::User);
        assert_eq!(identity, None);
    }

    #[test]
    fn project_target_requires_and_retains_workspace_identity() {
        let error = target(dto::OverlayTargetInput {
            skill_id: "developer".to_string(),
            scope: "project".to_string(),
            workspace_path: None,
        });
        assert!(error.is_err());

        let (_, _, identity) = target(dto::OverlayTargetInput {
            skill_id: "developer".to_string(),
            scope: "project".to_string(),
            workspace_path: Some("C:/work/project".to_string()),
        })
        .expect("valid project target");
        assert_eq!(identity.as_deref(), Some("C:/work/project"));
    }
}
