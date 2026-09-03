use crate::contexts::{
    skill_evolution_orchestration::{
        application::{
            AutomaticCorrectionDraftRequestV1, AutomaticDraftPipelineError,
            AutomaticDraftSafetyPort, DraftSafetyReceiptV1,
        },
        domain::{canonical_hash, ProducedCorrectionDraftV1},
    },
    tooling::skills::api::{
        scan_overlay_guidance, OverlayMutation, OverlayMutationRequest, OverlayScope,
        OverlayWitnesses, SkillApi, SkillId,
    },
};

pub(crate) struct OverlayAutomaticDraftSafety<'a> {
    skills: &'a SkillApi,
}

impl<'a> OverlayAutomaticDraftSafety<'a> {
    pub(crate) fn new(skills: &'a SkillApi) -> Self {
        Self { skills }
    }
}

impl AutomaticDraftSafetyPort for OverlayAutomaticDraftSafety<'_> {
    fn validate(
        &self,
        request: &AutomaticCorrectionDraftRequestV1,
        draft: &ProducedCorrectionDraftV1,
    ) -> Result<DraftSafetyReceiptV1, AutomaticDraftPipelineError> {
        let scan = scan_overlay_guidance(&draft.content);
        if !scan.passed {
            return Err(AutomaticDraftPipelineError::UnsafeContent);
        }
        let skill_id = SkillId::parse(&request.target_skill_id)
            .map_err(|_| AutomaticDraftPipelineError::OverlayRejected)?;
        let scope = parse_scope(&request.overlay_scope)?;
        let workspace = (scope == OverlayScope::Project).then_some(request.workspace_id.as_str());
        let summary = self
            .skills
            .overlay_summary(&skill_id, workspace)
            .map_err(|_| AutomaticDraftPipelineError::OverlayRejected)?;
        if summary.effective_hash != request.target_revision || summary.pinned {
            return Err(AutomaticDraftPipelineError::OverlayRejected);
        }
        let overlay_revision = summary
            .scopes
            .iter()
            .find(|entry| entry.scope == scope)
            .map(|entry| entry.revision);
        let preview = self
            .skills
            .overlay_preview(
                &OverlayMutationRequest {
                    canonical_skill_id: skill_id,
                    scope,
                    workspace_identity: workspace.map(str::to_string),
                    witnesses: OverlayWitnesses {
                        expected_overlay_revision: overlay_revision,
                        expected_base_instruction_hash: summary.base_instruction_hash,
                        expected_base_package_hash: summary.base_package_hash,
                        expected_payload_hash: None,
                        expected_pinned: false,
                    },
                    mutation: OverlayMutation::LearnedGuidance {
                        guidance: draft.content.clone(),
                    },
                },
                workspace,
            )
            .map_err(|_| AutomaticDraftPipelineError::OverlayRejected)?;
        if !preview.can_commit || !preview.scan.passed {
            return Err(AutomaticDraftPipelineError::OverlayRejected);
        }
        let overlay_preview_hash = canonical_hash(&(
            &draft.record.content_hash,
            &preview.current_to_proposed.base_hash,
            &preview.current_to_proposed.effective_hash,
            preview.current_to_proposed.added_characters,
            preview.current_to_proposed.removed_characters,
            preview.tentative_revision,
        ))
        .map_err(|_| AutomaticDraftPipelineError::OverlayRejected)?;
        Ok(DraftSafetyReceiptV1 {
            scanner_version: scan.scanner_version,
            overlay_preview_hash,
        })
    }
}

fn parse_scope(value: &str) -> Result<OverlayScope, AutomaticDraftPipelineError> {
    match value {
        "user" => Ok(OverlayScope::User),
        "project" => Ok(OverlayScope::Project),
        _ => Err(AutomaticDraftPipelineError::OverlayRejected),
    }
}
