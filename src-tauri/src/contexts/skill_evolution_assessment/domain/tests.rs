use super::target_catalog::project_target_catalog_entries;
use super::*;
use crate::contexts::tooling::api::{
    EffectiveSkillCatalogEntry, EffectiveSkillCatalogShadow, SkillAvailability, SkillLayer,
    SkillTrust, SkillType,
};
use serde_json::json;

fn input() -> SanitizedAssessmentInput {
    SanitizedAssessmentInput {
        schema_version: ASSESSMENT_SCHEMA_VERSION_V1,
        seed_id: "seed-1".to_string(),
        seed_revision: "revision-1".to_string(),
        seed_fingerprint: "fingerprint-1".to_string(),
        lineage_hash: "lineage-1".to_string(),
        workspace_id: Some("workspace:abcd".to_string()),
        sanitizer_version: "v1".to_string(),
        evidence_ids: vec!["evidence-b".to_string(), "evidence-a".to_string()],
        attribution: EvidenceAttribution::Verified,
    }
}

#[test]
fn assessment_wire_values_are_stable_and_reject_unknown_fields() {
    let encoded = serde_json::to_value(AssessmentRoute::NeedsHumanReview).expect("serialize route");
    assert_eq!(encoded, json!("needs_human_review"));
    let parsed = serde_json::from_value::<SanitizedAssessmentInput>(json!({
        "schemaVersion": 1, "seedId": "seed", "seedRevision": "revision", "seedFingerprint": "fingerprint",
        "lineageHash": "lineage", "workspaceId": null, "sanitizerVersion": "v1", "evidenceIds": [], "attribution": "verified", "unexpected": true
    })).expect_err("unknown fields must not become persisted input");
    let _ = parsed;
}

#[test]
fn witness_hash_is_order_independent_for_sets_and_changes_with_policy() {
    let target = EffectiveTargetWitness {
        skill_id: "review".to_string(),
        skill_type: "role".to_string(),
        revision_hash: "r1".to_string(),
        scope: TargetScope::Project,
        lifecycle: TargetLifecycle::Active,
        trust: TargetTrust::Trusted,
    };
    let first = AssessmentWitness {
        input: input(),
        targets: vec![target.clone()],
        selector_policy_version: "selector-v1".to_string(),
        lexical_policy_version: "lexical-v1".to_string(),
        gate_policy_version: "gates-v1".to_string(),
        routing_policy_version: "routing-v1".to_string(),
        confidence_policy_version: "confidence-v1".to_string(),
        consent_version: "consent-v1".to_string(),
        evaluator_configuration: None,
    };
    let mut reordered = first.clone();
    reordered.input.evidence_ids.reverse();
    assert_eq!(first.canonical_hash(), reordered.canonical_hash());
    reordered.gate_policy_version = "gates-v2".to_string();
    assert_ne!(first.canonical_hash(), reordered.canonical_hash());
}

#[test]
fn persisted_assessment_output_round_trips_all_nested_models() {
    let target = EffectiveTargetWitness {
        skill_id: "review".to_string(),
        skill_type: "role".to_string(),
        revision_hash: "r1".to_string(),
        scope: TargetScope::Project,
        lifecycle: TargetLifecycle::Active,
        trust: TargetTrust::Trusted,
    };
    let output = AssessmentOutput {
        schema_version: ASSESSMENT_SCHEMA_VERSION_V1,
        attempt_id: "attempt-1".to_string(),
        status: AssessmentAttemptStatus::Completed,
        classification: SelectionClassification::Selected,
        route: AssessmentRoute::Advance,
        confidence: AssessmentConfidence::High,
        risk: AssessmentRisk::Low,
        targets: vec![RankedTarget {
            witness: target,
            score: 80,
            attribution_score: 35,
            participation_score: 15,
            compatibility_score: 15,
            lexical_score: 10,
            locality_score: 5,
            matched_feature_classes: vec!["capability".to_string()],
            exclusions: vec![TargetExclusionReason::HistoricalOnly],
            attribution_uncertain: false,
        }],
        selection_threshold: SelectionThresholdWitness {
            leading_score: 80,
            runner_up_score: None,
            margin: 80,
            selected_minimum: 60,
            ambiguous_minimum: 45,
            required_margin: 15,
        },
        attribution_uncertain: false,
        lesson_shape: LessonShape {
            trigger: Some("verification_failure".to_string()),
            required_behavior: Some("inspect".to_string()),
            prohibited_behavior: None,
            verification: Some("test".to_string()),
            environment: Some("project".to_string()),
            content_kinds: vec!["guidance".to_string()],
        },
        checks: vec![QualityCheck {
            kind: QualityCheckKind::EvidenceSufficiency,
            result: QualityCheckResult::Pass,
            severity: AssessmentRisk::Low,
            reason_code: "verified_correction".to_string(),
            evidence_ids: vec!["evidence-a".to_string()],
            route_constraints: vec![AssessmentRoute::Advance],
        }],
        evaluator: EvaluatorResult {
            consulted: false,
            selected_target_id: None,
            confidence: None,
            recommended_route: None,
            cited_evidence_ids: Vec::new(),
            fallback_reason: Some(EvaluatorFallbackReason::DisabledConsent),
        },
    };
    let encoded = serde_json::to_string(&output).expect("serialize assessment output");
    let decoded: AssessmentOutput =
        serde_json::from_str(&encoded).expect("deserialize assessment output");
    assert_eq!(decoded, output);
}

#[test]
fn target_catalog_projects_complete_effective_metadata_and_participation() {
    let catalog = project_target_catalog_entries(
        &[
            catalog_source("system-skill", "system-r1", SkillLayer::System),
            catalog_source("project-skill", "project-r1", SkillLayer::Project),
        ],
        &[
            TargetParticipation {
                skill_id: "project-skill".to_string(),
                revision_hash: "project-r1".to_string(),
                verified_runs: 2,
                correlated_runs: 1,
            },
            TargetParticipation {
                skill_id: "project-skill".to_string(),
                revision_hash: "project-r1".to_string(),
                verified_runs: 3,
                correlated_runs: 4,
            },
            TargetParticipation {
                skill_id: "project-skill".to_string(),
                revision_hash: "historical-r0".to_string(),
                verified_runs: 100,
                correlated_runs: 100,
            },
        ],
    );

    assert_eq!(catalog.entries.len(), 2);
    let project = &catalog.entries[0];
    assert_eq!(project.witness.skill_id, "project-skill");
    assert_eq!(project.witness.revision_hash, "project-r1");
    assert_eq!(project.witness.scope, TargetScope::Project);
    assert_eq!(project.witness.lifecycle, TargetLifecycle::Active);
    assert_eq!(project.witness.trust, TargetTrust::Trusted);
    assert_eq!(project.name, "Project Skill");
    assert_eq!(project.skill_type, "utility");
    assert_eq!(project.category, "quality");
    assert_eq!(project.description, "Review project changes");
    assert_eq!(project.declared_tools, vec!["read", "search"]);
    assert_eq!(project.capabilities, vec!["code-review", "verification"]);
    assert_eq!(project.verified_participation, 5);
    assert_eq!(project.correlated_participation, 5);
    assert_eq!(
        catalog.entries[1].witness.lifecycle,
        TargetLifecycle::Pinned
    );
}

#[test]
fn target_catalog_records_every_noneligible_revision_as_an_exclusion() {
    let mut effective = catalog_source("review", "current-r2", SkillLayer::User);
    effective.shadowed = vec![
        EffectiveSkillCatalogShadow {
            skill_id: "review".to_string(),
            revision: "shadowed-r1".to_string(),
            availability: SkillAvailability::Available,
        },
        EffectiveSkillCatalogShadow {
            skill_id: "review".to_string(),
            revision: "malformed-r1".to_string(),
            availability: SkillAvailability::Invalid,
        },
    ];
    let mut malformed = catalog_source("broken", "broken-r1", SkillLayer::User);
    malformed.availability = SkillAvailability::Conflicting;
    let catalog = project_target_catalog_entries(
        &[effective, malformed],
        &[
            participation("review", "current-r2"),
            participation("review", "historical-r0"),
            participation("missing", "missing-r1"),
            participation("missing", "missing-r1"),
        ],
    );

    assert_eq!(catalog.entries.len(), 1);
    assert_eq!(
        catalog.exclusions,
        vec![
            exclusion("broken", "broken-r1", TargetExclusionReason::Malformed),
            exclusion("missing", "missing-r1", TargetExclusionReason::Missing),
            exclusion(
                "review",
                "historical-r0",
                TargetExclusionReason::HistoricalOnly,
            ),
            exclusion("review", "malformed-r1", TargetExclusionReason::Malformed,),
            exclusion("review", "shadowed-r1", TargetExclusionReason::Shadowed,),
        ]
    );
}

#[test]
fn target_catalog_preserves_scope_lifecycle_and_multi_skill_participation_boundaries() {
    let mut archived = catalog_source("archived", "archived-r1", SkillLayer::User);
    archived.availability = SkillAvailability::Disabled;
    let catalog = project_target_catalog_entries(
        &[
            catalog_source("system", "system-r1", SkillLayer::System),
            catalog_source("remote", "remote-r1", SkillLayer::Registry),
            archived,
            catalog_source("project", "project-r2", SkillLayer::Project),
        ],
        &[
            participation("project", "project-r2"),
            participation("remote", "remote-r1"),
            participation("project", "project-r1"),
        ],
    );

    assert_eq!(
        catalog
            .entries
            .iter()
            .map(|entry| (entry.witness.scope, entry.witness.lifecycle))
            .collect::<Vec<_>>(),
        vec![
            (TargetScope::Project, TargetLifecycle::Active),
            (TargetScope::User, TargetLifecycle::Archived),
            (TargetScope::Remote, TargetLifecycle::Pinned),
            (TargetScope::System, TargetLifecycle::Pinned),
        ]
    );
    assert_eq!(catalog.entries[0].verified_participation, 1);
    assert_eq!(catalog.entries[2].verified_participation, 1);
    assert!(catalog.exclusions.contains(&exclusion(
        "project",
        "project-r1",
        TargetExclusionReason::HistoricalOnly,
    )));
}

fn participation(skill_id: &str, revision_hash: &str) -> TargetParticipation {
    TargetParticipation {
        skill_id: skill_id.to_string(),
        revision_hash: revision_hash.to_string(),
        verified_runs: 1,
        correlated_runs: 0,
    }
}

fn exclusion(
    skill_id: &str,
    revision_hash: &str,
    reason: TargetExclusionReason,
) -> TargetExclusion {
    TargetExclusion {
        skill_id: skill_id.to_string(),
        revision_hash: revision_hash.to_string(),
        reason,
    }
}

fn catalog_source(skill_id: &str, revision: &str, layer: SkillLayer) -> EffectiveSkillCatalogEntry {
    EffectiveSkillCatalogEntry {
        skill_id: skill_id.to_string(),
        name: if layer == SkillLayer::Project {
            "Project Skill".to_string()
        } else {
            "System Skill".to_string()
        },
        description: "Review project changes".to_string(),
        category: "quality".to_string(),
        revision: revision.to_string(),
        layer,
        availability: SkillAvailability::Available,
        trust: SkillTrust::Trusted,
        skill_type: SkillType::Utility,
        capabilities: vec!["code-review".to_string(), "verification".to_string()],
        declared_tools: vec!["read".to_string(), "search".to_string()],
        shadowed: Vec::new(),
    }
}
