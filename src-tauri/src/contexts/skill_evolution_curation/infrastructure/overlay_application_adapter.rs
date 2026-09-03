use crate::contexts::skill_evolution_curation::{application::*, domain::*};
use crate::contexts::tooling::skills::api::{
    OverlayError, OverlayGovernedMutationRequest, OverlayKey, OverlayMutation,
    OverlayMutationRequest, OverlayScope, OverlayWitnesses, SkillApi, SkillError, SkillId,
};

pub(crate) struct SkillApiCuratorApplication<'a> {
    skills: &'a SkillApi,
    active_workspace: Option<&'a str>,
}

impl<'a> SkillApiCuratorApplication<'a> {
    pub(crate) fn new(skills: &'a SkillApi, active_workspace: Option<&'a str>) -> Self {
        Self {
            skills,
            active_workspace,
        }
    }
}

impl CuratorOverlayApplicationPort for SkillApiCuratorApplication<'_> {
    fn apply(
        &self,
        request: &CuratorOverlayApplicationRequest,
    ) -> Result<CuratorOverlayApplicationReceipt, CuratorApplicationFailure> {
        let governed = governed_request(request)?;
        let outcome = self
            .skills
            .governed_overlay_mutation(&governed, self.workspace(request)?)
            .map_err(map_error)?;
        Ok(CuratorOverlayApplicationReceipt {
            overlay_revision: outcome.committed_revision.to_string(),
            overlay_history_id: outcome.history_event_hash,
            effective_diff_hash: outcome.effective_diff_hash,
            duplicate: outcome.duplicate,
        })
    }

    fn find_committed(
        &self,
        request: &CuratorOverlayApplicationRequest,
    ) -> Result<Option<CuratorOverlayApplicationReceipt>, CuratorApplicationFailure> {
        let governed = governed_request(request)?;
        let key = OverlayKey {
            canonical_skill_id: governed.mutation.canonical_skill_id,
            scope: governed.mutation.scope,
            workspace_identity: governed.mutation.workspace_identity,
        };
        let entry = self
            .skills
            .overlay_history_by_application(&key, &request.application_id)
            .map_err(map_error)?;
        entry
            .map(|entry| {
                let effective_diff_hash = entry
                    .committed_effective_diff_hash
                    .ok_or(CuratorApplicationFailure::Integrity)?;
                if effective_diff_hash != request.witnesses.proposed_effective_hash {
                    return Err(CuratorApplicationFailure::Stale);
                }
                Ok(CuratorOverlayApplicationReceipt {
                    overlay_revision: entry.next_revision.to_string(),
                    overlay_history_id: entry.event_hash,
                    effective_diff_hash,
                    duplicate: true,
                })
            })
            .transpose()
    }
}

fn governed_request(
    request: &CuratorOverlayApplicationRequest,
) -> Result<OverlayGovernedMutationRequest, CuratorApplicationFailure> {
    let canonical_skill_id = SkillId::parse(&request.target_skill_id)
        .map_err(|_| CuratorApplicationFailure::Validation)?;
    let scope = parse_scope(&request.overlay_scope)?;
    Ok(OverlayGovernedMutationRequest {
        application_id: request.application_id.clone(),
        expected_effective_diff_hash: request.witnesses.proposed_effective_hash.clone(),
        mutation: OverlayMutationRequest {
            canonical_skill_id,
            scope,
            workspace_identity: (scope == OverlayScope::Project)
                .then(|| request.workspace_id.clone()),
            witnesses: OverlayWitnesses {
                expected_overlay_revision: request.witnesses.expected_overlay_revision,
                expected_base_instruction_hash: request.witnesses.base_instruction_hash.clone(),
                expected_base_package_hash: request.witnesses.base_package_hash.clone(),
                expected_payload_hash: None,
                expected_pinned: request.witnesses.expected_pinned,
            },
            mutation: match &request.mutation {
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
            },
        },
    })
}

fn parse_scope(value: &str) -> Result<OverlayScope, CuratorApplicationFailure> {
    match value {
        "user" => Ok(OverlayScope::User),
        "project" => Ok(OverlayScope::Project),
        _ => Err(CuratorApplicationFailure::Validation),
    }
}

impl SkillApiCuratorApplication<'_> {
    fn workspace<'a>(
        &'a self,
        request: &'a CuratorOverlayApplicationRequest,
    ) -> Result<Option<&'a str>, CuratorApplicationFailure> {
        match request.overlay_scope.as_str() {
            "project" if self.active_workspace == Some(request.workspace_id.as_str()) => {
                Ok(self.active_workspace)
            }
            "project" => Err(CuratorApplicationFailure::Stale),
            "user" => Ok(self.active_workspace),
            _ => Err(CuratorApplicationFailure::Validation),
        }
    }
}

fn map_error(error: SkillError) -> CuratorApplicationFailure {
    match error {
        SkillError::Overlay(OverlayError::PinnedRefusal { .. }) => {
            CuratorApplicationFailure::Pinned
        }
        SkillError::Overlay(OverlayError::StaleWitnesses { .. }) => {
            CuratorApplicationFailure::Stale
        }
        SkillError::Overlay(OverlayError::NeedsReconciliation { .. }) => {
            CuratorApplicationFailure::Conflict
        }
        SkillError::Conflict(_) => CuratorApplicationFailure::Conflict,
        SkillError::Overlay(OverlayError::Integrity { .. }) => CuratorApplicationFailure::Integrity,
        SkillError::Overlay(OverlayError::InvalidRequest { .. })
        | SkillError::Overlay(OverlayError::LimitExceeded { .. })
        | SkillError::Overlay(OverlayError::TrustRequired { .. }) => {
            CuratorApplicationFailure::Validation
        }
        SkillError::Filesystem(_) => CuratorApplicationFailure::Filesystem,
        _ => CuratorApplicationFailure::Unavailable,
    }
}
