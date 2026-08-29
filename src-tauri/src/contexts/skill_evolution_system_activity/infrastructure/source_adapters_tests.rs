use std::{collections::BTreeMap, sync::Arc};

use super::*;
use crate::contexts::skill_evolution_system_activity::domain::*;

struct Reader {
    record: CommittedProjectionRecord,
}

struct BatchReader {
    batch: CommittedProjectionBatch,
}

impl CommittedEvolutionRecordReader for Reader {
    fn scan(
        &self,
        _domain: EvolutionSourceDomain,
        _after: Option<&str>,
        _limit: u16,
    ) -> Result<CommittedProjectionBatch, ProjectionSourceError> {
        Ok(CommittedProjectionBatch {
            records: vec![self.record.clone()],
            retention_floor: Some("retention:1".into()),
            has_more: false,
        })
    }
}

impl CommittedEvolutionRecordReader for BatchReader {
    fn scan(
        &self,
        _domain: EvolutionSourceDomain,
        _after: Option<&str>,
        _limit: u16,
    ) -> Result<CommittedProjectionBatch, ProjectionSourceError> {
        Ok(self.batch.clone())
    }
}

#[test]
fn all_adapters_expose_only_their_fixed_source_domains() {
    let sources: Vec<Box<dyn EvolutionProjectionSource>> = vec![
        Box::new(OrchestrationProjectionSource::new(reader(
            EvolutionSourceDomain::Orchestration,
        ))),
        Box::new(EvidenceProjectionSource::new(reader(
            EvolutionSourceDomain::Evidence,
        ))),
        Box::new(AssessmentProjectionSource::new(reader(
            EvolutionSourceDomain::Assessment,
        ))),
        Box::new(GenerationProjectionSource::new(reader(
            EvolutionSourceDomain::Generation,
        ))),
        Box::new(CuratorProjectionSource::new(reader(
            EvolutionSourceDomain::Curator,
        ))),
        Box::new(OverlayProjectionSource::new(reader(
            EvolutionSourceDomain::Overlay,
        ))),
        Box::new(AutomaticApplicationProjectionSource::new(reader(
            EvolutionSourceDomain::AutomaticApplication,
        ))),
        Box::new(ProbationProjectionSource::new(reader(
            EvolutionSourceDomain::Probation,
        ))),
        Box::new(BreakerProjectionSource::new(reader(
            EvolutionSourceDomain::Breaker,
        ))),
        Box::new(SkillCreationProjectionSource::new(reader(
            EvolutionSourceDomain::SkillCreation,
        ))),
        Box::new(RecoveryProjectionSource::new(reader(
            EvolutionSourceDomain::Recovery,
        ))),
        Box::new(RetentionProjectionSource::new(reader(
            EvolutionSourceDomain::Retention,
        ))),
    ];
    let limit = ProjectionScanLimit::new(10).expect("limit");
    for source in sources {
        let page = source.scan_committed(None, limit).expect("committed page");
        assert_eq!(page.source_domain, source.domain());
        assert_eq!(page.events.len(), 1);
        assert_eq!(
            page.events[0].envelope.source_domain,
            source.domain().as_str()
        );
        assert_eq!(page.next_cursor.expect("cursor").expose(), "cursor:1");
        assert_eq!(
            page.retention_floor.expect("retention floor").expose(),
            "retention:1"
        );
    }
}

#[test]
fn rolled_back_or_cross_domain_records_fail_before_projection() {
    let mut rolled_back = reader(EvolutionSourceDomain::Evidence).record.clone();
    rolled_back.committed = false;
    let source = EvidenceProjectionSource::new(Arc::new(Reader {
        record: rolled_back,
    }));
    assert_eq!(
        source.scan_committed(None, ProjectionScanLimit::new(1).expect("limit")),
        Err(ProjectionSourceError::IntegrityFailed)
    );

    let source = EvidenceProjectionSource::new(reader(EvolutionSourceDomain::Assessment));
    assert_eq!(
        source.scan_committed(None, ProjectionScanLimit::new(1).expect("limit")),
        Err(ProjectionSourceError::IntegrityFailed)
    );
}

#[test]
fn corrupt_integrity_and_unsupported_envelopes_fail_closed() {
    let mut corrupt = reader(EvolutionSourceDomain::Generation).record.clone();
    corrupt.source_integrity_hash.clear();
    let source = GenerationProjectionSource::new(Arc::new(Reader { record: corrupt }));
    assert_eq!(scan(&source), Err(ProjectionSourceError::IntegrityFailed));

    let mut unsupported = reader(EvolutionSourceDomain::Generation).record.clone();
    unsupported.envelope.schema_version = 2;
    assert_eq!(
        scan(&GenerationProjectionSource::new(Arc::new(Reader {
            record: unsupported,
        }))),
        Err(ProjectionSourceError::InvalidEnvelope(
            ActivityEnvelopeError::UnsupportedSchemaVersion(2)
        ))
    );
}

#[test]
fn missing_or_replayed_sequences_and_unbounded_pages_are_rejected() {
    let mut missing = reader(EvolutionSourceDomain::Assessment).record.clone();
    missing.source_sequence = 0;
    missing.envelope.source_sequence = 0;
    missing.envelope = missing.envelope.seal().expect("zero-sequence envelope");
    assert_eq!(
        scan(&AssessmentProjectionSource::new(Arc::new(Reader {
            record: missing,
        }))),
        Err(ProjectionSourceError::InvalidSequence)
    );

    let repeated = reader(EvolutionSourceDomain::Assessment).record.clone();
    let source = AssessmentProjectionSource::new(Arc::new(BatchReader {
        batch: CommittedProjectionBatch {
            records: vec![repeated.clone(), repeated],
            retention_floor: None,
            has_more: false,
        },
    }));
    assert_eq!(scan(&source), Err(ProjectionSourceError::UnboundedPage));
}

#[test]
fn retention_floor_is_opaque_and_committed_replay_is_deterministic() {
    let source = EvidenceProjectionSource::new(reader(EvolutionSourceDomain::Evidence));
    let first = scan(&source).expect("first source scan");
    let replay = scan(&source).expect("replayed source scan");
    assert_eq!(first, replay);
    assert_eq!(
        first.retention_floor.expect("retention floor").expose(),
        "retention:1"
    );

    let invalid_floor = EvidenceProjectionSource::new(Arc::new(BatchReader {
        batch: CommittedProjectionBatch {
            records: vec![reader(EvolutionSourceDomain::Evidence).record.clone()],
            retention_floor: Some(String::new()),
            has_more: false,
        },
    }));
    assert_eq!(
        scan(&invalid_floor),
        Err(ProjectionSourceError::InvalidCursor)
    );
}

fn scan(
    source: &dyn EvolutionProjectionSource,
) -> Result<ProjectionSourcePage, ProjectionSourceError> {
    source.scan_committed(None, ProjectionScanLimit::new(1).expect("limit"))
}

fn reader(domain: EvolutionSourceDomain) -> Arc<Reader> {
    Arc::new(Reader {
        record: CommittedProjectionRecord {
            committed: true,
            cursor: "cursor:1".into(),
            source_sequence: 1,
            source_integrity_hash: "sha256:source".into(),
            envelope: envelope(domain),
        },
    })
}

fn envelope(domain: EvolutionSourceDomain) -> EvolutionActivityEnvelopeV1 {
    EvolutionActivityEnvelopeV1 {
        schema_version: ACTIVITY_SCHEMA_VERSION_V1,
        event_id: format!("event-{}", domain.as_str()),
        event_code: ActivityEventCode::RunCompleted,
        source_domain: domain.as_str().into(),
        source_id: "source-one".into(),
        source_revision: "revision-one".into(),
        source_sequence: 1,
        scope_kind: ActivityScopeKind::Workspace,
        canonical_scope_id: "workspace-one".into(),
        occurred_at_ms: 1,
        committed_at_ms: 2,
        severity: ActivitySeverity::Info,
        status: ActivityStatus::Succeeded,
        attention_kind: ActivityAttentionKind::None,
        safe_actor_kind: ActivityActorKind::System,
        safe_identities: Vec::new(),
        metrics: BTreeMap::new(),
        reason_codes: Vec::new(),
        navigation: None,
        supersedes_event_id: None,
        payload: None,
        projection_policy_version: 1,
        content_hash: String::new(),
    }
    .seal()
    .expect("sealed envelope")
}
