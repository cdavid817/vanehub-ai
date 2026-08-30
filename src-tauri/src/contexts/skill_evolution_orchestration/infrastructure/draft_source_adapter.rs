use crate::contexts::{
    skill_evolution_evidence::api::SkillEvolutionEvidenceApi,
    skill_evolution_orchestration::application::{
        AuthorizedCorrectionSourcePort, AuthorizedCorrectionSourceV1, AutomaticDraftPipelineError,
    },
};

pub(crate) struct EvidenceAuthorizedCorrectionSource<'a> {
    evidence: &'a SkillEvolutionEvidenceApi,
}

impl<'a> EvidenceAuthorizedCorrectionSource<'a> {
    pub(crate) fn new(evidence: &'a SkillEvolutionEvidenceApi) -> Self {
        Self { evidence }
    }
}

impl AuthorizedCorrectionSourcePort for EvidenceAuthorizedCorrectionSource<'_> {
    fn resolve(
        &self,
        authorization_id: &str,
    ) -> Result<Option<AuthorizedCorrectionSourceV1>, AutomaticDraftPipelineError> {
        self.evidence
            .authorized_correction_guidance(authorization_id)
            .map(|source| {
                source.map(|source| AuthorizedCorrectionSourceV1 {
                    authorization_id: source.authorization_id,
                    sanitized_guidance: source.sanitized_guidance,
                    sanitizer_version: source.sanitizer_version,
                    authorization_witness_hash: source.authorization_witness_hash,
                })
            })
            .map_err(|_| AutomaticDraftPipelineError::SourceUnavailable)
    }
}
