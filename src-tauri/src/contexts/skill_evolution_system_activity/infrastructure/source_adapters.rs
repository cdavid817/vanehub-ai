use std::sync::Arc;

use crate::contexts::skill_evolution_system_activity::domain::{
    EvolutionActivityEnvelopeV1, EvolutionProjectionSource, EvolutionSourceDomain,
    OpaqueDomainCursor, ProjectionScanLimit, ProjectionSourceError, ProjectionSourcePage,
    VerifiedProjectionEvent,
};

#[derive(Debug, Clone)]
pub(crate) struct CommittedProjectionRecord {
    pub(crate) committed: bool,
    pub(crate) cursor: String,
    pub(crate) source_sequence: u64,
    pub(crate) source_integrity_hash: String,
    pub(crate) envelope: EvolutionActivityEnvelopeV1,
}

#[derive(Debug, Clone)]
pub(crate) struct CommittedProjectionBatch {
    pub(crate) records: Vec<CommittedProjectionRecord>,
    pub(crate) retention_floor: Option<String>,
    pub(crate) has_more: bool,
}

pub(crate) trait CommittedEvolutionRecordReader: Send + Sync {
    fn scan(
        &self,
        domain: EvolutionSourceDomain,
        after: Option<&str>,
        limit: u16,
    ) -> Result<CommittedProjectionBatch, ProjectionSourceError>;
}

struct DomainProjectionSource {
    domain: EvolutionSourceDomain,
    reader: Arc<dyn CommittedEvolutionRecordReader>,
}

impl DomainProjectionSource {
    fn new(domain: EvolutionSourceDomain, reader: Arc<dyn CommittedEvolutionRecordReader>) -> Self {
        Self { domain, reader }
    }

    fn scan_committed(
        &self,
        after: Option<&OpaqueDomainCursor>,
        limit: ProjectionScanLimit,
    ) -> Result<ProjectionSourcePage, ProjectionSourceError> {
        let batch = self.reader.scan(
            self.domain,
            after.map(OpaqueDomainCursor::expose),
            limit.get(),
        )?;
        if batch.records.len() > usize::from(limit.get()) {
            return Err(ProjectionSourceError::UnboundedPage);
        }

        let mut events = Vec::with_capacity(batch.records.len());
        let mut next_cursor = None;
        for record in batch.records {
            if !record.committed
                || record.envelope.source_domain != self.domain.as_str()
                || record.envelope.source_sequence != record.source_sequence
            {
                return Err(ProjectionSourceError::IntegrityFailed);
            }
            let source_cursor = OpaqueDomainCursor::parse(record.cursor)?;
            next_cursor = Some(source_cursor.clone());
            events.push(VerifiedProjectionEvent {
                source_cursor,
                source_sequence: record.source_sequence,
                source_integrity_hash: record.source_integrity_hash,
                envelope: record.envelope,
            });
        }

        let page = ProjectionSourcePage {
            source_domain: self.domain,
            events,
            next_cursor,
            retention_floor: batch
                .retention_floor
                .map(OpaqueDomainCursor::parse)
                .transpose()?,
            has_more: batch.has_more,
        };
        page.validate(limit)?;
        Ok(page)
    }
}

macro_rules! projection_source {
    ($name:ident, $domain:expr) => {
        pub(crate) struct $name(DomainProjectionSource);

        impl $name {
            pub(crate) fn new(reader: Arc<dyn CommittedEvolutionRecordReader>) -> Self {
                Self(DomainProjectionSource::new($domain, reader))
            }
        }

        impl EvolutionProjectionSource for $name {
            fn domain(&self) -> EvolutionSourceDomain {
                $domain
            }

            fn scan_committed(
                &self,
                after: Option<&OpaqueDomainCursor>,
                limit: ProjectionScanLimit,
            ) -> Result<ProjectionSourcePage, ProjectionSourceError> {
                self.0.scan_committed(after, limit)
            }
        }
    };
}

projection_source!(
    OrchestrationProjectionSource,
    EvolutionSourceDomain::Orchestration
);
projection_source!(EvidenceProjectionSource, EvolutionSourceDomain::Evidence);
projection_source!(
    AssessmentProjectionSource,
    EvolutionSourceDomain::Assessment
);
projection_source!(
    GenerationProjectionSource,
    EvolutionSourceDomain::Generation
);
projection_source!(CuratorProjectionSource, EvolutionSourceDomain::Curator);
projection_source!(OverlayProjectionSource, EvolutionSourceDomain::Overlay);
projection_source!(
    AutomaticApplicationProjectionSource,
    EvolutionSourceDomain::AutomaticApplication
);
projection_source!(ProbationProjectionSource, EvolutionSourceDomain::Probation);
projection_source!(BreakerProjectionSource, EvolutionSourceDomain::Breaker);
projection_source!(
    SkillCreationProjectionSource,
    EvolutionSourceDomain::SkillCreation
);
projection_source!(RecoveryProjectionSource, EvolutionSourceDomain::Recovery);
projection_source!(RetentionProjectionSource, EvolutionSourceDomain::Retention);
