use crate::contexts::skill_evolution_evidence::domain::EvidenceSourceEnvelope;

use super::PipelineFailureCategory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessingReport {
    pub(crate) persisted_signals: usize,
}

pub(crate) trait EvidenceEnvelopeProcessor: Send + Sync {
    fn process(
        &self,
        envelope: &EvidenceSourceEnvelope,
    ) -> Result<ProcessingReport, PipelineFailureCategory>;
}
