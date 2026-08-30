use super::overlay_witnesses::overlay_state_witnesses;
use crate::contexts::skill_evolution_curation::{application::*, domain::*};
use crate::contexts::tooling::skills::api::{
    OverlayMutation, OverlayMutationRequest, OverlayScope, OverlayWitnesses, SkillApi, SkillId,
};

pub(crate) struct SkillApiCuratorDraftValidator<'a> {
    skills: &'a SkillApi,
    active_workspace: Option<&'a str>,
}

impl<'a> SkillApiCuratorDraftValidator<'a> {
    pub(crate) fn new(skills: &'a SkillApi, active_workspace: Option<&'a str>) -> Self {
        Self {
            skills,
            active_workspace,
        }
    }
}

impl CuratorOverlayDraftValidationPort for SkillApiCuratorDraftValidator<'_> {
    fn dry_validate(
        &self,
        binding: &CuratorDraftCandidateBinding,
        mutation: &CuratorDraftMutationInput,
    ) -> Result<CuratorOverlayValidationReceipt, CuratorOverlayValidationError> {
        let skill_id = SkillId::parse(&binding.target_skill_id)
            .map_err(|_| invalid("draft.invalid-target"))?;
        let scope = match binding.overlay_scope.as_str() {
            "project" => OverlayScope::Project,
            "user" => OverlayScope::User,
            _ => return Err(invalid("draft.system-scope-escalation")),
        };
        if scope == OverlayScope::Project && self.active_workspace.is_none() {
            return Err(invalid("draft.active-workspace-required"));
        }
        let summary = self
            .skills
            .overlay_summary(&skill_id, self.active_workspace)
            .map_err(|_| invalid("draft.overlay-snapshot-unavailable"))?;
        if summary.effective_hash != binding.target_revision {
            return Err(invalid("draft.target-witness-stale"));
        }
        let overlay_revision = summary
            .scopes
            .iter()
            .find(|entry| entry.scope == scope)
            .map(|entry| entry.revision);
        let state_witnesses = overlay_state_witnesses(&summary, scope);
        let request = OverlayMutationRequest {
            canonical_skill_id: skill_id,
            scope,
            workspace_identity: (scope == OverlayScope::Project)
                .then(|| self.active_workspace.map(str::to_owned))
                .flatten(),
            witnesses: OverlayWitnesses {
                expected_overlay_revision: overlay_revision,
                expected_base_instruction_hash: summary.base_instruction_hash.clone(),
                expected_base_package_hash: summary.base_package_hash.clone(),
                expected_payload_hash: None,
                expected_pinned: summary.pinned,
            },
            mutation: overlay_mutation(mutation),
        };
        let preview = self
            .skills
            .overlay_preview(&request, self.active_workspace)
            .map_err(|_| invalid("draft.overlay-dry-validation-failed"))?;
        if !preview.can_commit {
            return Err(invalid("draft.exact-patch-mismatch-or-conflict"));
        }
        Ok(CuratorOverlayValidationReceipt {
            scanner_version: preview.scan.scanner_version,
            base_hash: summary.base_instruction_hash,
            base_package_hash: summary.base_package_hash,
            effective_hash: summary.effective_hash,
            overlay_revision,
            pin_witness: state_witnesses.pin,
            trust_witness: state_witnesses.trust,
            conflict_witness: state_witnesses.conflict,
        })
    }
}

fn overlay_mutation(input: &CuratorDraftMutationInput) -> OverlayMutation {
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

fn invalid(reason_code: &str) -> CuratorOverlayValidationError {
    CuratorOverlayValidationError {
        reason_code: reason_code.to_owned(),
        scanner_version: "overlay-text-v1".to_owned(),
    }
}
