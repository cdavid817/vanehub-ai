use super::overlay_witnesses::overlay_state_witnesses;
use crate::contexts::skill_evolution_curation::{application::*, domain::*};
use crate::contexts::tooling::skills::api::{
    OverlayDiff, OverlayError, OverlayMutation, OverlayMutationRequest, OverlayScope,
    OverlayScopeStatus, OverlayTrustState, OverlayWitnesses, SkillApi, SkillError, SkillId,
};

const CURATOR_DIFF_TEXT_LIMIT: usize = 8 * 1_024;

pub(crate) struct SkillApiCuratorPreviewer<'a> {
    skills: &'a SkillApi,
    active_workspace: Option<&'a str>,
}

impl<'a> SkillApiCuratorPreviewer<'a> {
    pub(crate) fn new(skills: &'a SkillApi, active_workspace: Option<&'a str>) -> Self {
        Self {
            skills,
            active_workspace,
        }
    }
}

impl CuratorOverlayPreviewPort for SkillApiCuratorPreviewer<'_> {
    fn preview(
        &self,
        binding: &CuratorPreviewBinding,
    ) -> Result<CuratorOverlayPreviewReceipt, CuratorOverlayPreviewError> {
        let skill_id = SkillId::parse(&binding.target_skill_id)
            .map_err(|_| error("preview.invalid-target", None))?;
        let scope = parse_scope(&binding.overlay_scope)?;
        if scope == OverlayScope::Project && self.active_workspace.is_none() {
            return Err(error("preview.active-workspace-required", None));
        }
        let summary = self
            .skills
            .overlay_summary(&skill_id, self.active_workspace)
            .map_err(map_skill_error)?;
        validate_live_summary(binding, &summary, scope)?;
        let state = overlay_state_witnesses(&summary, scope);
        let request = OverlayMutationRequest {
            canonical_skill_id: skill_id,
            scope,
            workspace_identity: (scope == OverlayScope::Project)
                .then(|| self.active_workspace.map(str::to_owned))
                .flatten(),
            witnesses: OverlayWitnesses {
                expected_overlay_revision: binding.overlay_revision,
                expected_base_instruction_hash: binding.base_instruction_hash.clone(),
                expected_base_package_hash: binding.base_package_hash.clone(),
                expected_payload_hash: None,
                expected_pinned: false,
            },
            mutation: mutation(&binding.mutation),
        };
        let preview = self
            .skills
            .overlay_preview(&request, self.active_workspace)
            .map_err(map_skill_error)?;
        if !preview.can_commit || !preview.conflicts.is_empty() {
            return Err(error(
                "preview.conflict-or-patch-ambiguity",
                Some(CuratorStalenessReason::ConflictChanged),
            ));
        }
        let current_scope = summary.scopes.iter().find(|entry| entry.scope == scope);
        let trusted = current_scope.is_none_or(|entry| {
            entry.trust == OverlayTrustState::Trusted && entry.status == OverlayScopeStatus::Applied
        });
        let witnesses = CuratorPreviewWitnesses {
            candidate_hash: binding.candidate_hash.clone(),
            draft_hash: binding.draft_hash.clone(),
            assessment_hash: binding.assessment_hash.clone(),
            target_revision: binding.target_revision.clone(),
            base_instruction_hash: summary.base_instruction_hash,
            base_package_hash: summary.base_package_hash,
            current_effective_hash: summary.effective_hash,
            proposed_effective_hash: preview.base_to_proposed.effective_hash.clone(),
            overlay_revision: binding.overlay_revision,
            pin_witness: state.pin,
            trust_witness: state.trust,
            conflict_witness: state.conflict,
            scanner_version: preview.scan.scanner_version.clone(),
            policy_hash: binding.policy_hash.clone(),
        };
        Ok(CuratorOverlayPreviewReceipt {
            diffs: CuratorPreviewDiffs {
                base_to_current: project_diff(&preview.base_to_current),
                current_to_proposed: project_diff(&preview.current_to_proposed),
                base_to_proposed: project_diff(&preview.base_to_proposed),
            },
            validation: CuratorPreviewValidation {
                scan_passed: preview.scan.passed,
                can_commit: preview.can_commit,
                pinned: false,
                trusted,
                conflict_count: preview.conflicts.len(),
                conflicts_complete: !preview.conflicts_truncated,
                safe_rule_ids: preview.scan.safe_rule_ids,
                rules_complete: !preview.scan.rule_ids_truncated,
            },
            witnesses,
        })
    }
}

fn validate_live_summary(
    binding: &CuratorPreviewBinding,
    summary: &crate::contexts::tooling::skills::api::OverlaySummary,
    scope: OverlayScope,
) -> Result<(), CuratorOverlayPreviewError> {
    if summary.base_instruction_hash != binding.base_instruction_hash
        || summary.base_package_hash != binding.base_package_hash
    {
        return Err(error(
            "preview.base-drift",
            Some(CuratorStalenessReason::BaseChanged),
        ));
    }
    if summary.effective_hash != binding.current_effective_hash
        || summary.effective_hash != binding.target_revision
        || summary
            .scopes
            .iter()
            .find(|entry| entry.scope == scope)
            .map(|entry| entry.revision)
            != binding.overlay_revision
    {
        return Err(error(
            "preview.overlay-drift",
            Some(CuratorStalenessReason::OverlayChanged),
        ));
    }
    let live = overlay_state_witnesses(summary, scope);
    if live.pin != binding.pin_witness || summary.pinned {
        return Err(error(
            "preview.pinned",
            Some(CuratorStalenessReason::PinChanged),
        ));
    }
    if live.trust != binding.trust_witness {
        return Err(error(
            "preview.trust-drift",
            Some(CuratorStalenessReason::TrustChanged),
        ));
    }
    if live.conflict != binding.conflict_witness || summary.needs_reconcile {
        return Err(error(
            "preview.conflict-drift",
            Some(CuratorStalenessReason::ConflictChanged),
        ));
    }
    Ok(())
}

fn project_diff(diff: &OverlayDiff) -> CuratorDiffProjection {
    let hunks = diff
        .hunks
        .iter()
        .map(|hunk| CuratorDiffHunk {
            label: hunk.label.clone(),
            before: project_text(
                &hunk.before.content,
                hunk.before.total_characters,
                hunk.before.truncated,
            ),
            after: project_text(
                &hunk.after.content,
                hunk.after.total_characters,
                hunk.after.truncated,
            ),
        })
        .collect::<Vec<_>>();
    let complete = !diff.hunks_truncated
        && hunks
            .iter()
            .all(|hunk| !hunk.before.truncated && !hunk.after.truncated);
    CuratorDiffProjection {
        from_hash: diff.base_hash.clone(),
        to_hash: diff.effective_hash.clone(),
        added_characters: diff.added_characters,
        removed_characters: diff.removed_characters,
        hunks,
        complete,
    }
}

fn project_text(content: &str, total_characters: usize, truncated: bool) -> CuratorDiffText {
    let projected = content
        .chars()
        .take(CURATOR_DIFF_TEXT_LIMIT)
        .collect::<String>();
    let projection_truncated = content.chars().count() > CURATOR_DIFF_TEXT_LIMIT;
    CuratorDiffText {
        content: projected,
        total_characters,
        truncated: truncated || projection_truncated,
    }
}

fn mutation(input: &CuratorDraftMutationInput) -> OverlayMutation {
    match input {
        CuratorDraftMutationInput::LearnedGuidance { guidance } => {
            OverlayMutation::LearnedGuidance {
                guidance: guidance.clone(),
            }
        }
        CuratorDraftMutationInput::ExactPatch {
            old_string,
            new_string,
            replace_all,
        } => OverlayMutation::ExactPatch {
            old_string: old_string.clone(),
            new_string: new_string.clone(),
            replace_all: *replace_all,
        },
    }
}

fn parse_scope(value: &str) -> Result<OverlayScope, CuratorOverlayPreviewError> {
    match value {
        "project" => Ok(OverlayScope::Project),
        "user" => Ok(OverlayScope::User),
        _ => Err(error("preview.invalid-scope", None)),
    }
}

fn map_skill_error(value: SkillError) -> CuratorOverlayPreviewError {
    match value {
        SkillError::Overlay(OverlayError::PinnedRefusal { .. }) => {
            error("preview.pinned", Some(CuratorStalenessReason::PinChanged))
        }
        SkillError::Overlay(OverlayError::TrustRequired { .. }) => error(
            "preview.trust-required",
            Some(CuratorStalenessReason::TrustChanged),
        ),
        SkillError::Overlay(OverlayError::NeedsReconciliation { .. }) => error(
            "preview.needs-reconciliation",
            Some(CuratorStalenessReason::ConflictChanged),
        ),
        SkillError::Overlay(OverlayError::StaleWitnesses { pin_changed, .. }) if pin_changed => {
            error(
                "preview.pin-drift",
                Some(CuratorStalenessReason::PinChanged),
            )
        }
        SkillError::Overlay(OverlayError::StaleWitnesses {
            base_changed: true, ..
        }) => error(
            "preview.base-drift",
            Some(CuratorStalenessReason::BaseChanged),
        ),
        SkillError::Overlay(OverlayError::StaleWitnesses { .. }) => error(
            "preview.overlay-drift",
            Some(CuratorStalenessReason::OverlayChanged),
        ),
        SkillError::Overlay(OverlayError::LimitExceeded { .. }) => {
            error("preview.size-limit", None)
        }
        SkillError::Overlay(OverlayError::InvalidRequest { .. }) => {
            error("preview.patch-ambiguity-or-invalid", None)
        }
        _ => error("preview.overlay-unavailable", None),
    }
}

fn error(
    reason_code: &str,
    staleness: Option<CuratorStalenessReason>,
) -> CuratorOverlayPreviewError {
    CuratorOverlayPreviewError {
        reason_code: reason_code.to_string(),
        staleness,
    }
}
