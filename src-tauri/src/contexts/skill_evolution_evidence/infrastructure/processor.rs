use crate::contexts::skill_evolution_evidence::application::{
    EvidenceEnvelopeProcessor, PipelineFailureCategory, ProcessingReport,
};
use crate::contexts::skill_evolution_evidence::domain::{
    extract_registered_signals, EvidenceSanitizer, EvidenceSourceEnvelope, TaskFingerprintBuilder,
};

use super::SqliteEvolutionEvidenceRepository;

pub(crate) struct EvidenceIngestionProcessor {
    repository: SqliteEvolutionEvidenceRepository,
    sanitizer: EvidenceSanitizer,
    fingerprints: TaskFingerprintBuilder,
}

impl EvidenceIngestionProcessor {
    pub(crate) fn new(
        repository: SqliteEvolutionEvidenceRepository,
        installation_key: &[u8],
    ) -> Result<Self, PipelineFailureCategory> {
        Ok(Self {
            repository,
            sanitizer: EvidenceSanitizer::new(installation_key)
                .map_err(|_| PipelineFailureCategory::Sanitization)?,
            fingerprints: TaskFingerprintBuilder::new(installation_key)
                .map_err(|_| PipelineFailureCategory::Storage)?,
        })
    }
}

impl EvidenceEnvelopeProcessor for EvidenceIngestionProcessor {
    fn process(
        &self,
        envelope: &EvidenceSourceEnvelope,
    ) -> Result<ProcessingReport, PipelineFailureCategory> {
        envelope
            .validate()
            .map_err(|_| PipelineFailureCategory::InvalidEnvelope)?;
        let sanitized = envelope
            .sanitized_registered_text(&self.sanitizer)
            .map_err(|_| PipelineFailureCategory::Sanitization)?;
        let signals = extract_registered_signals(envelope, sanitized.as_ref());
        for signal in &signals {
            self.repository
                .persist_signal(signal, &self.fingerprints)
                .map_err(|_| PipelineFailureCategory::Storage)?;
        }
        if !signals.is_empty() {
            self.repository
                .rebuild_dirty_seeds()
                .map_err(|_| PipelineFailureCategory::Storage)?;
        }
        Ok(ProcessingReport {
            persisted_signals: signals.len(),
        })
    }
}

#[cfg(test)]
mod tests;
