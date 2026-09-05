use std::collections::BTreeMap;

use crate::contexts::skill_evolution_generation::domain::{
    DossierRecordV1, DossierSectionStatus, DossierSourceWitnessV1,
};

use super::{AuthoritativeDossierSnapshotV1, DossierEffectiveSkillSourceV1};

pub(super) fn section_records(
    snapshot: &AuthoritativeDossierSnapshotV1,
) -> [Vec<DossierRecordV1>; 13] {
    let identity = vec![
        DossierRecordV1::Identity {
            identity_kind: "seed_id".into(),
            value: snapshot.identity.seed_id.clone(),
        },
        DossierRecordV1::Identity {
            identity_kind: "assessment_attempt_id".into(),
            value: snapshot.identity.assessment_attempt_id.clone(),
        },
    ];
    let summary = summary_records(snapshot);
    let seed = seed_records(snapshot);
    let signals = snapshot
        .signals
        .iter()
        .map(|signal| DossierRecordV1::SourceReference {
            source_id: signal.signal_id.clone(),
            category: signal.category.clone(),
            occurred_at_ms: signal.occurred_at_ms,
        })
        .collect();
    let mut targets: Vec<_> = snapshot
        .targets
        .iter()
        .map(|target| DossierRecordV1::Target {
            skill_id: target.skill_id.clone(),
            revision: target.revision.clone(),
            score_bps: target.score_bps,
        })
        .collect();
    if targets.is_empty() {
        if let Some(reason) = &snapshot.no_target_reason_code {
            targets.push(DossierRecordV1::Summary {
                codes: vec!["no_target".into(), reason.clone()],
                metrics: BTreeMap::new(),
            });
        }
    }
    let quality = snapshot
        .quality_checks
        .iter()
        .map(|check| DossierRecordV1::QualityCheck {
            code: check.code.clone(),
            result: check.result.clone(),
            reason_code: check.reason_code.clone(),
        })
        .collect();
    let effective = snapshot
        .effective_skill
        .as_ref()
        .map(effective_skill_records)
        .unwrap_or_default();
    let guidance = guidance_records(snapshot);
    let timeline = timeline_records(snapshot);
    let privacy = snapshot
        .privacy_classes
        .iter()
        .map(|item| DossierRecordV1::PrivacyClass {
            class_code: item.class_code.clone(),
            count: item.redacted_count,
        })
        .collect();
    let rationale = snapshot
        .rationale
        .iter()
        .map(|claim| DossierRecordV1::LessonClaim {
            claim_id: claim.claim_id.clone(),
            claim_kind: claim.claim_kind.clone(),
            text: claim.safe_text.clone(),
            citation_ids: claim.citation_ids.clone(),
        })
        .collect();
    let verification = snapshot
        .verification
        .iter()
        .map(|step| DossierRecordV1::VerificationStep {
            step_id: step.step_id.clone(),
            action_code: step.action_code.clone(),
            citation_ids: step.citation_ids.clone(),
        })
        .collect();
    let lineage = snapshot.lineage.iter().map(witness_record).collect();
    [
        identity,
        summary,
        seed,
        signals,
        targets,
        quality,
        effective,
        guidance,
        timeline,
        privacy,
        rationale,
        verification,
        lineage,
    ]
}

fn summary_records(snapshot: &AuthoritativeDossierSnapshotV1) -> Vec<DossierRecordV1> {
    let metrics = BTreeMap::from([
        ("signal_count".into(), snapshot.signals.len() as i64),
        ("target_count".into(), snapshot.targets.len() as i64),
    ]);
    vec![DossierRecordV1::Summary {
        codes: vec![
            snapshot.seed.category.clone(),
            snapshot.seed.readiness.clone(),
        ],
        metrics,
    }]
}

fn seed_records(snapshot: &AuthoritativeDossierSnapshotV1) -> Vec<DossierRecordV1> {
    vec![DossierRecordV1::Summary {
        codes: vec![
            snapshot.seed.category.clone(),
            snapshot.seed.readiness.clone(),
            snapshot.seed.safe_summary.clone(),
        ],
        metrics: BTreeMap::from([(
            "independent_run_count".into(),
            i64::from(snapshot.seed.independent_run_count),
        )]),
    }]
}

fn effective_skill_records(skill: &DossierEffectiveSkillSourceV1) -> Vec<DossierRecordV1> {
    vec![
        DossierRecordV1::Summary {
            codes: vec![
                skill.skill_id.clone(),
                skill.skill_type.clone(),
                skill.scope.clone(),
                skill.overlay_state.clone(),
            ],
            metrics: BTreeMap::new(),
        },
        DossierRecordV1::Witness {
            witness_kind: "effective_skill".into(),
            revision: skill.effective_revision.clone(),
            content_hash: skill
                .witnesses
                .first()
                .map(|witness| witness.content_hash.clone())
                .unwrap_or_default(),
        },
    ]
}

fn guidance_records(snapshot: &AuthoritativeDossierSnapshotV1) -> Vec<DossierRecordV1> {
    let mut records: Vec<_> = snapshot
        .guidance
        .excerpts
        .iter()
        .map(|excerpt| DossierRecordV1::SkillExcerpt {
            excerpt_id: excerpt.excerpt_id.clone(),
            logical_location: excerpt.logical_location.clone(),
            text: excerpt.safe_text.clone(),
        })
        .collect();
    records.extend(snapshot.guidance.resources.iter().map(|resource| {
        DossierRecordV1::SourceReference {
            source_id: resource.resource_id.clone(),
            category: resource.resource_kind.clone(),
            occurred_at_ms: 0,
        }
    }));
    records
}

fn timeline_records(snapshot: &AuthoritativeDossierSnapshotV1) -> Vec<DossierRecordV1> {
    let mut buckets: BTreeMap<String, (u32, i64, i64)> = BTreeMap::new();
    for event in &snapshot.timeline {
        let bucket = buckets.entry(event.event_code.clone()).or_insert((
            0,
            event.occurred_at_ms,
            event.occurred_at_ms,
        ));
        bucket.0 += 1;
        bucket.1 = bucket.1.min(event.occurred_at_ms);
        bucket.2 = bucket.2.max(event.occurred_at_ms);
    }
    buckets
        .into_iter()
        .map(
            |(event_code, (count, first_at_ms, last_at_ms))| DossierRecordV1::TimelineBucket {
                event_code,
                count,
                first_at_ms,
                last_at_ms,
            },
        )
        .collect()
}

pub(super) fn section_witnesses(
    snapshot: &AuthoritativeDossierSnapshotV1,
) -> [Vec<DossierSourceWitnessV1>; 13] {
    let seed = vec![snapshot.seed.witness.clone()];
    let signals = snapshot
        .signals
        .iter()
        .map(|signal| signal.witness.clone())
        .collect();
    let effective = snapshot
        .effective_skill
        .as_ref()
        .map(|skill| skill.witnesses.clone())
        .unwrap_or_default();
    [
        snapshot.lineage.clone(),
        snapshot.lineage.clone(),
        seed,
        signals,
        snapshot.lineage.clone(),
        snapshot.lineage.clone(),
        effective.clone(),
        effective,
        snapshot.lineage.clone(),
        snapshot.lineage.clone(),
        snapshot.lineage.clone(),
        snapshot.lineage.clone(),
        snapshot.lineage.clone(),
    ]
}

pub(super) fn section_status(
    ordinal: usize,
    records: &[DossierRecordV1],
    snapshot: &AuthoritativeDossierSnapshotV1,
    truncation: &crate::contexts::skill_evolution_generation::domain::DossierTruncationV1,
) -> DossierSectionStatus {
    if !truncation.complete {
        return DossierSectionStatus::Partial;
    }
    if !records.is_empty() {
        return DossierSectionStatus::Complete;
    }
    if (ordinal == 6 && snapshot.identity.target_skill_id.is_none()) || matches!(ordinal, 7..=11) {
        DossierSectionStatus::NotApplicable
    } else {
        DossierSectionStatus::Unavailable
    }
}

pub(super) fn unavailable_reason(status: DossierSectionStatus, ordinal: usize) -> Option<String> {
    match status {
        DossierSectionStatus::NotApplicable => Some(format!("section_{ordinal}_not_applicable")),
        DossierSectionStatus::Unavailable => Some(format!("section_{ordinal}_source_unavailable")),
        _ => None,
    }
}

fn witness_record(witness: &DossierSourceWitnessV1) -> DossierRecordV1 {
    DossierRecordV1::Witness {
        witness_kind: witness.source_kind.clone(),
        revision: witness.revision.clone(),
        content_hash: witness.content_hash.clone(),
    }
}
