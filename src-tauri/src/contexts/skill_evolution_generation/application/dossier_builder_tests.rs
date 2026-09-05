use crate::contexts::skill_evolution_generation::{
    application::canonical_json,
    domain::{
        DossierRecordV1, DossierSectionStatus, DossierSourceWitnessV1, FrozenGenerationInputV1,
        GenerationEvidenceWitnessV1, GENERATION_SCHEMA_VERSION_V1,
    },
};

use super::*;

#[test]
fn identical_witnesses_produce_byte_identical_dossiers() {
    let input = input();
    let source = snapshot();
    let first = build(&input, &source).expect("first dossier");
    let mut reordered = source.clone();
    reordered.signals.reverse();
    reordered.targets.reverse();
    reordered.lineage.reverse();
    let second = build(&input, &reordered).expect("second dossier");
    assert_eq!(first.content_hash, second.content_hash);
    assert_eq!(canonical_json(&first), canonical_json(&second));
    assert!(first.has_exact_section_shape());
}

#[test]
fn version_drift_and_purged_lineage_fail_closed() {
    let input = input();
    let mut source = snapshot();
    source.sanitizer_version = "unknown-v2".into();
    assert_eq!(
        build(&input, &source),
        Err(DossierBuildError::IncompatibleVersion)
    );
    let mut source = snapshot();
    source.lineage[0].schema_version = 2;
    assert_eq!(
        build(&input, &source),
        Err(DossierBuildError::IncompatibleVersion)
    );
    let mut source = snapshot();
    source.lineage_complete = false;
    assert_eq!(
        build(&input, &source),
        Err(DossierBuildError::IncompatibleVersion)
    );
}

#[test]
fn absent_optional_records_remain_explicit_and_no_target_is_complete() {
    let input = input();
    let mut source = snapshot();
    source.effective_skill = None;
    source.identity.target_skill_id = None;
    source.targets.clear();
    source.no_target_reason_code = Some("strong_no_target".into());
    source.guidance = DossierGuidanceSourceV1::default();
    let dossier = build(&input, &source).expect("dossier");
    assert_eq!(dossier.sections[4].status, DossierSectionStatus::Complete);
    assert_eq!(
        dossier.sections[6].status,
        DossierSectionStatus::NotApplicable
    );
    assert_eq!(
        dossier.sections[7].status,
        DossierSectionStatus::NotApplicable
    );
    assert!(dossier.sections[6].unavailable_reason_code.is_some());
}

#[test]
fn record_and_excerpt_limits_are_explicitly_reported() {
    let input = input();
    let mut source = snapshot();
    source.signals = (0..105)
        .map(|index| DossierSignalSourceV1 {
            signal_id: format!("signal-{index:03}"),
            category: "failure".into(),
            occurred_at_ms: index,
            witness: witness("signal", &format!("signal-{index:03}")),
        })
        .collect();
    source.targets = (0..36)
        .map(|index| DossierTargetSourceV1 {
            skill_id: format!("skill-{index:03}"),
            revision: "r1".into(),
            score_bps: 5000,
        })
        .collect();
    source.guidance.excerpts = vec![
        DossierExcerptSourceV1 {
            excerpt_id: "excerpt-a".into(),
            logical_location: "a".into(),
            safe_text: "a".repeat(8_000),
        },
        DossierExcerptSourceV1 {
            excerpt_id: "excerpt-b".into(),
            logical_location: "b".into(),
            safe_text: "b".repeat(500),
        },
    ];
    source.timeline = (0..1_010)
        .map(|index| DossierTimelineSourceV1 {
            event_code: "failure".into(),
            occurred_at_ms: index,
        })
        .collect();
    let dossier = build(&input, &source).expect("bounded dossier");
    assert_truncation(&dossier, 3, 100, 105);
    assert_truncation(&dossier, 4, 32, 36);
    assert_truncation(&dossier, 7, 1, 2);
    assert_truncation(&dossier, 8, 1_000, 1_010);
    assert!(dossier.canonical_size_bytes <= 128 * 1024);
}

#[test]
fn redaction_metadata_and_multi_skill_targets_stay_sanitized_and_sorted() {
    let input = input();
    let mut source = snapshot();
    source.targets = vec![
        DossierTargetSourceV1 {
            skill_id: "z-skill".into(),
            revision: "r1".into(),
            score_bps: 6000,
        },
        DossierTargetSourceV1 {
            skill_id: "a-skill".into(),
            revision: "r2".into(),
            score_bps: 7000,
        },
    ];
    source.privacy_classes = vec![DossierPrivacySourceV1 {
        class_code: "credential".into(),
        redacted_count: 3,
    }];
    source.seed.safe_summary = "failure at [REDACTED:PATH]".into();
    let dossier = build(&input, &source).expect("dossier");
    let serialized = canonical_json(&dossier).expect("json");
    assert!(serialized.contains("[REDACTED:PATH]"));
    assert!(!serialized.contains("/home/private"));
    match &dossier.sections[4].records[0] {
        DossierRecordV1::Target { skill_id, .. } => assert_eq!(skill_id, "a-skill"),
        record => panic!("unexpected target record: {record:?}"),
    }
}

fn assert_truncation(
    dossier: &crate::contexts::skill_evolution_generation::domain::EvidenceDossierV1,
    ordinal: usize,
    retained: u32,
    total: u32,
) {
    let section = &dossier.sections[ordinal];
    assert_eq!(section.status, DossierSectionStatus::Partial);
    assert_eq!(
        (
            section.truncation.retained_count,
            section.truncation.total_count
        ),
        (retained, total)
    );
}

pub(crate) fn build(
    input: &FrozenGenerationInputV1,
    snapshot: &AuthoritativeDossierSnapshotV1,
) -> Result<crate::contexts::skill_evolution_generation::domain::EvidenceDossierV1, DossierBuildError>
{
    build_dossier(&DossierBuildRequestV1 {
        dossier_id: "dossier-one",
        revision: 1,
        builder_version: "builder-v1",
        input,
        snapshot,
        supersedes_dossier_id: None,
        created_at_ms: 1,
    })
}

pub(crate) fn witness(kind: &str, id: &str) -> DossierSourceWitnessV1 {
    DossierSourceWitnessV1 {
        schema_version: 1,
        source_kind: kind.into(),
        source_id: id.into(),
        revision: "r1".into(),
        content_hash: format!("sha256:{kind}-{id}"),
    }
}

pub(crate) fn input() -> FrozenGenerationInputV1 {
    FrozenGenerationInputV1 {
        schema_version: GENERATION_SCHEMA_VERSION_V1,
        request_id: "request-one".into(),
        workspace_id: Some("workspace-one".into()),
        seed_id: "seed-one".into(),
        seed_revision: "r1".into(),
        assessment_attempt_id: "assessment-one".into(),
        assessment_revision: "r1".into(),
        assessment_route: "advance".into(),
        target: None,
        evidence: GenerationEvidenceWitnessV1 {
            lineage_hash: "sha256:lineage".into(),
            sanitizer_version: "1".into(),
            evidence_ids: vec!["signal-one".into()],
            source_revision_hash: "sha256:sources".into(),
        },
        effective_skill: None,
        curator: None,
        policy_revision: 1,
        policy_hash: "sha256:policy".into(),
        consent_revision: 1,
        consent_hash: "sha256:consent".into(),
        model_configuration_hash: "sha256:model".into(),
        dossier_builder_version: "builder-v1".into(),
        renderer_version: "renderer-v1".into(),
        validator_version: "validator-v1".into(),
        frozen_at_ms: 1,
    }
}

pub(crate) fn snapshot() -> AuthoritativeDossierSnapshotV1 {
    AuthoritativeDossierSnapshotV1 {
        sanitizer_version: "1".into(),
        lineage_complete: true,
        identity: DossierIdentitySourceV1 {
            workspace_id: Some("workspace-one".into()),
            seed_id: "seed-one".into(),
            assessment_attempt_id: "assessment-one".into(),
            target_skill_id: Some("skill-one".into()),
        },
        seed: DossierSeedSourceV1 {
            category: "failure".into(),
            readiness: "ready".into(),
            safe_summary: "bounded summary".into(),
            independent_run_count: 3,
            witness: witness("seed", "seed-one"),
        },
        signals: vec![DossierSignalSourceV1 {
            signal_id: "signal-one".into(),
            category: "failure".into(),
            occurred_at_ms: 1,
            witness: witness("signal", "signal-one"),
        }],
        targets: vec![DossierTargetSourceV1 {
            skill_id: "skill-one".into(),
            revision: "r1".into(),
            score_bps: 9000,
        }],
        no_target_reason_code: None,
        quality_checks: vec![DossierQualitySourceV1 {
            code: "evidence_sufficiency".into(),
            result: "pass".into(),
            reason_code: "sufficient".into(),
        }],
        effective_skill: Some(DossierEffectiveSkillSourceV1 {
            skill_id: "skill-one".into(),
            skill_type: "personal".into(),
            scope: "project".into(),
            effective_revision: "r1".into(),
            overlay_state: "none".into(),
            metadata_codes: vec!["active".into()],
            witnesses: vec![witness("skill", "skill-one")],
        }),
        guidance: DossierGuidanceSourceV1 {
            excerpts: vec![DossierExcerptSourceV1 {
                excerpt_id: "excerpt-one".into(),
                logical_location: "instructions/1".into(),
                safe_text: "verify before completion".into(),
            }],
            resources: vec![DossierResourceSourceV1 {
                resource_id: "reference-one".into(),
                resource_kind: "reference".into(),
                revision: "r1".into(),
            }],
        },
        timeline: vec![
            DossierTimelineSourceV1 {
                event_code: "failure".into(),
                occurred_at_ms: 1,
            },
            DossierTimelineSourceV1 {
                event_code: "recovery".into(),
                occurred_at_ms: 2,
            },
        ],
        privacy_classes: vec![DossierPrivacySourceV1 {
            class_code: "path".into(),
            redacted_count: 1,
        }],
        rationale: vec![DossierClaimSourceV1 {
            claim_id: "claim-one".into(),
            claim_kind: "action".into(),
            safe_text: "add verification".into(),
            citation_ids: vec!["signal-one".into()],
        }],
        verification: vec![DossierVerificationSourceV1 {
            step_id: "step-one".into(),
            action_code: "run_check".into(),
            citation_ids: vec!["signal-one".into()],
        }],
        lineage: vec![
            witness("assessment", "assessment-one"),
            witness("evidence", "signal-one"),
        ],
    }
}
