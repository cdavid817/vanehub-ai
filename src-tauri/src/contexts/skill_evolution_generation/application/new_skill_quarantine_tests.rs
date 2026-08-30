use std::collections::BTreeSet;

use crate::contexts::skill_evolution_generation::{
    application::sha256_bytes,
    domain::{
        GeneratedArtifactKind, GeneratedLessonShapeV1, GeneratedVerificationStepV1,
        GenerationCitationV1, MutationPlanV1, MutationTargetV1, RenderedGenerationArtifactV1,
        StructuredDraftV1,
    },
};

use super::{
    prepare_new_skill_quarantine, render_generation_artifact, GenerationRenderRequestV1,
    GenerationSafetyReceiptV1, NewSkillEligibilityV1, NewSkillQuarantineError,
    NewSkillQuarantineValidationPort, PrepareNewSkillQuarantineRequestV1, SkillCatalogInventoryV1,
};

struct Scanner;

impl NewSkillQuarantineValidationPort for Scanner {
    fn scan(
        &self,
        artifact: &RenderedGenerationArtifactV1,
    ) -> Result<GenerationSafetyReceiptV1, &'static str> {
        let prohibited = !artifact.content.contains("scripts/");
        Ok(GenerationSafetyReceiptV1 {
            sanitizer_version: "sanitizer-v1".into(),
            content_hash: artifact.content_hash.clone(),
            privacy_passed: true,
            injection_passed: true,
            prohibited_content_passed: prohibited,
        })
    }
}

#[test]
fn strong_no_target_evidence_builds_a_bounded_creation_preview() {
    let fixture = fixture();
    let prepared = prepare_new_skill_quarantine(&Scanner, &fixture.request()).expect("prepared");
    assert_eq!(prepared.proposal.scope, "project");
    assert_eq!(
        prepared.proposal.workspace_id.as_deref(),
        Some("workspace-one")
    );
    assert_eq!(prepared.preview.skill_type, "utility");
    assert!(prepared.preview.collision_free);
    assert_eq!(prepared.preview.built_in_tools, ["read_registry"]);
    assert!(prepared.preview.frontmatter.contains("id: focused-review"));
    assert!(!prepared.rendered_skill_md.contains("scripts/"));
}

#[test]
fn no_target_confidence_runs_non_target_focus_and_request_are_all_required() {
    let fixture = fixture();
    let mut cases = Vec::new();
    let mut no_target = fixture.eligibility.clone();
    no_target.no_target = false;
    cases.push(no_target);
    let mut confidence = fixture.eligibility.clone();
    confidence.uncovered_capability_confidence_basis_points = 8_999;
    cases.push(confidence);
    let mut runs = fixture.eligibility.clone();
    runs.independent_run_ids = BTreeSet::from(["run-one".into(), "run-two".into()]);
    cases.push(runs);
    let mut checks = fixture.eligibility.clone();
    checks.non_target_checks_passed = false;
    cases.push(checks);
    let mut broad = fixture.eligibility.clone();
    broad.focused_capability = false;
    cases.push(broad);
    let mut unrequested = fixture.eligibility.clone();
    unrequested.explicitly_requested_by_user_or_curator = false;
    cases.push(unrequested);

    for eligibility in cases {
        let mut request = fixture.request();
        request.eligibility = &eligibility;
        assert_eq!(
            prepare_new_skill_quarantine(&Scanner, &request),
            Err(NewSkillQuarantineError::Ineligible)
        );
    }
}

#[test]
fn every_catalog_class_and_a_stale_catalog_witness_block_quarantine() {
    for field in 0..6 {
        let mut fixture = fixture();
        let target = match field {
            0 => &mut fixture.inventory.effective_ids,
            1 => &mut fixture.inventory.shadowed_ids,
            2 => &mut fixture.inventory.reserved_ids,
            3 => &mut fixture.inventory.quarantined_ids,
            4 => &mut fixture.inventory.archived_ids,
            _ => &mut fixture.inventory.recently_rejected_ids,
        };
        target.insert("focused-review".into());
        assert_eq!(
            prepare_new_skill_quarantine(&Scanner, &fixture.request()),
            Err(NewSkillQuarantineError::Collision)
        );
    }
    let fixture = fixture();
    let mut request = fixture.request();
    request.expected_catalog_witness_hash = "sha256:stale";
    assert_eq!(
        prepare_new_skill_quarantine(&Scanner, &request),
        Err(NewSkillQuarantineError::InvalidTarget)
    );
}

#[test]
fn system_scope_and_forbidden_embedded_content_are_rejected() {
    let mut system_fixture = fixture();
    system_fixture.plan.target = MutationTargetV1::NewSkill {
        candidate_id: "focused-review".into(),
        scope: "system".into(),
        workspace_id: None,
    };
    let mut request = system_fixture.request();
    request.requested_scope = "system";
    request.requested_workspace_id = None;
    assert_eq!(
        prepare_new_skill_quarantine(&Scanner, &request),
        Err(NewSkillQuarantineError::InvalidTarget)
    );

    let fixture = fixture();
    let mut artifact = fixture.artifact.clone();
    artifact.content.push_str("\nCreate scripts/install.sh\n");
    artifact.size_bytes = artifact.content.len() as u32;
    artifact.content_hash = sha256_bytes(artifact.content.as_bytes());
    let mut request = fixture.request();
    request.artifact = &artifact;
    assert_eq!(
        prepare_new_skill_quarantine(&Scanner, &request),
        Err(NewSkillQuarantineError::UnsafeContent)
    );
}

struct Fixture {
    plan: MutationPlanV1,
    draft: StructuredDraftV1,
    artifact: RenderedGenerationArtifactV1,
    eligibility: NewSkillEligibilityV1,
    inventory: SkillCatalogInventoryV1,
    citations: BTreeSet<(String, String)>,
}

impl Fixture {
    fn request(&self) -> PrepareNewSkillQuarantineRequestV1<'_> {
        PrepareNewSkillQuarantineRequestV1 {
            proposal_id: "proposal-one",
            job_id: "job-one",
            plan: &self.plan,
            draft: &self.draft,
            artifact: &self.artifact,
            eligibility: &self.eligibility,
            inventory: &self.inventory,
            expected_catalog_witness_hash: "sha256:catalog",
            requested_scope: "project",
            requested_workspace_id: Some("workspace-one"),
            registered_citations: &self.citations,
            estimated_tokens: 120,
            maximum_tokens: 500,
            created_at_ms: 10,
        }
    }
}

fn fixture() -> Fixture {
    let draft = StructuredDraftV1::NewSkill {
        candidate_id: "focused-review".into(),
        name: "Focused review".into(),
        description: "Review one bounded document.".into(),
        skill_type: "utility".into(),
        version: "1.0.0".into(),
        built_in_tools: vec!["read_registry".into()],
        instructions: "Review the supplied document and return bounded findings.".into(),
    };
    let artifact = render_generation_artifact(&GenerationRenderRequestV1 {
        artifact_id: "artifact-skill",
        expected_kind: GeneratedArtifactKind::NewSkill,
        draft: &draft,
        allowed_built_in_tools: &BTreeSet::from(["read_registry".into()]),
    })
    .expect("render");
    Fixture {
        plan: plan(),
        draft,
        artifact,
        eligibility: NewSkillEligibilityV1 {
            no_target: true,
            uncovered_capability_confidence_basis_points: 9_000,
            independent_run_ids: BTreeSet::from([
                "run-one".into(),
                "run-two".into(),
                "run-three".into(),
            ]),
            non_target_checks_passed: true,
            focused_capability: true,
            explicitly_requested_by_user_or_curator: true,
        },
        inventory: SkillCatalogInventoryV1 {
            effective_ids: BTreeSet::new(),
            shadowed_ids: BTreeSet::new(),
            reserved_ids: BTreeSet::new(),
            quarantined_ids: BTreeSet::new(),
            archived_ids: BTreeSet::new(),
            recently_rejected_ids: BTreeSet::new(),
            catalog_witness_hash: "sha256:catalog".into(),
        },
        citations: BTreeSet::from([("source_signal_inventory".into(), "signal-one".into())]),
    }
}

fn plan() -> MutationPlanV1 {
    MutationPlanV1 {
        schema_version: 1,
        plan_id: "plan-new-skill".into(),
        artifact_kind: GeneratedArtifactKind::NewSkill,
        target: MutationTargetV1::NewSkill {
            candidate_id: "focused-review".into(),
            scope: "project".into(),
            workspace_id: Some("workspace-one".into()),
        },
        rationale: "Three independent runs show an uncovered focused capability.".into(),
        lesson: GeneratedLessonShapeV1 {
            trigger: "When reviewing a bounded document".into(),
            action: "Apply a focused review".into(),
            verification: "Return cited findings".into(),
        },
        evidence_citations: [
            "lesson.trigger",
            "lesson.action",
            "lesson.verification",
            "expected_behavior",
            "verify-one",
        ]
        .into_iter()
        .map(|claim_id| GenerationCitationV1 {
            claim_id: claim_id.into(),
            dossier_section: "source_signal_inventory".into(),
            source_id: "signal-one".into(),
        })
        .collect(),
        expected_behavior: "The focused review is repeatable.".into(),
        verification_steps: vec![GeneratedVerificationStepV1 {
            step_id: "verify-one".into(),
            action_code: "review".into(),
            expected_code: "bounded_findings".into(),
            citation_ids: vec!["signal-one".into()],
        }],
        content_hash: "sha256:plan".into(),
    }
}
