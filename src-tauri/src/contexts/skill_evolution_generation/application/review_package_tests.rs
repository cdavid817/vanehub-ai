use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

use crate::contexts::skill_evolution_generation::{
    application::{canonical_hash, sha256_bytes},
    domain::{
        GeneratedArtifactKind, GeneratedLessonShapeV1, GeneratedReviewPackageV1,
        GeneratedVerificationStepV1, GenerationCitationV1, GenerationHandoffStatus,
        GenerationModelCallRecordV1, GenerationModelOutcome, GenerationQuarantineStatus,
        GenerationValidationStatus, GenerationValidationV1, MutationPlanV1, MutationTargetV1,
        QuarantinedSkillProposalV1, StructuredDraftV1,
    },
};

use super::{
    build_review_package, commit_approved_generated_skill, derive_user_edited_review_package,
    handoff_generation_package, render_generation_artifact, BuildReviewPackageRequestV1,
    GeneratedSkillCommitError, GeneratedSkillCommitRequestV1, GeneratedSkillCommitResultV1,
    GeneratedSkillCreationTransactionV1, GenerationCuratorHandoffError,
    GenerationCuratorHandoffPort, GenerationRenderRequestV1, NormalSkillCreationTransactionPort,
};

#[derive(Default)]
struct IdempotentCurator {
    candidates: RefCell<BTreeMap<String, String>>,
}

impl GenerationCuratorHandoffPort for IdempotentCurator {
    fn attach_existing_draft(
        &self,
        _package: &crate::contexts::skill_evolution_generation::domain::PreparedGenerationReviewPackageV1,
        idempotency_key: &str,
    ) -> Result<String, GenerationCuratorHandoffError> {
        Ok(self.candidate(idempotency_key, "draft"))
    }

    fn attach_creation_candidate(
        &self,
        _package: &crate::contexts::skill_evolution_generation::domain::PreparedGenerationReviewPackageV1,
        idempotency_key: &str,
    ) -> Result<String, GenerationCuratorHandoffError> {
        Ok(self.candidate(idempotency_key, "creation"))
    }
}

impl IdempotentCurator {
    fn candidate(&self, key: &str, prefix: &str) -> String {
        self.candidates
            .borrow_mut()
            .entry(key.into())
            .or_insert_with(|| format!("{prefix}-candidate"))
            .clone()
    }
}

#[test]
fn immutable_packages_handoff_idempotently_to_the_right_curator_queue() {
    let curator = IdempotentCurator::default();
    let existing = package(false);
    let first = handoff_generation_package(&curator, &existing).expect("existing handoff");
    let second = handoff_generation_package(&curator, &existing).expect("duplicate handoff");
    assert_eq!(first, second);
    assert!(!first.creation_candidate);
    assert_eq!(curator.candidates.borrow().len(), 1);

    let creation = package(true);
    let result = handoff_generation_package(&curator, &creation).expect("creation handoff");
    assert!(result.creation_candidate);
    assert_eq!(result.curator_candidate_id, "creation-candidate");
    assert_eq!(curator.candidates.borrow().len(), 2);
}

#[test]
fn handoff_rejects_any_package_that_loses_permanent_auto_exclusion() {
    let curator = IdempotentCurator::default();
    let mut package = package(false);
    package.payload.auto_apply_excluded = false;
    package.package_hash = canonical_hash(&package.payload).expect("tampered hash");
    assert_eq!(
        handoff_generation_package(&curator, &package),
        Err(GenerationCuratorHandoffError::AutoMutationForbidden)
    );
    assert!(curator.candidates.borrow().is_empty());
}

#[test]
fn user_edited_derivative_keeps_model_provenance_and_permanent_auto_exclusion() {
    let parent = package(false);
    let draft = StructuredDraftV1::OverlayLearnBlock {
        guidance: "Validate the bounded result and record the invariant.".into(),
    };
    let artifact = render_generation_artifact(&GenerationRenderRequestV1 {
        artifact_id: "artifact-edited",
        expected_kind: GeneratedArtifactKind::OverlayLearnBlock,
        draft: &draft,
        allowed_built_in_tools: &BTreeSet::new(),
    })
    .expect("render edit");
    let validation = validation(&artifact.artifact_id, "validation-edited");
    let derived = derive_user_edited_review_package(
        &parent,
        "package-edited",
        draft,
        artifact,
        validation,
        20,
    )
    .expect("derive");
    assert_eq!(
        derived.payload.parent_package_id.as_deref(),
        Some("package-existing")
    );
    assert!(derived.payload.user_edited);
    assert!(derived.payload.auto_apply_excluded);
    assert!(derived.payload.package.permanently_manual);
    assert_eq!(derived.payload.model_calls, parent.payload.model_calls);
    assert_eq!(
        derived.payload.package.model_provenance_hash,
        parent.payload.package.model_provenance_hash
    );
}

struct CreationPort {
    current_preview: String,
    collision: bool,
    fail_once: RefCell<bool>,
    calls: RefCell<u8>,
}

impl NormalSkillCreationTransactionPort for CreationPort {
    fn commit(
        &self,
        transaction: &GeneratedSkillCreationTransactionV1,
    ) -> Result<GeneratedSkillCommitResultV1, GeneratedSkillCommitError> {
        *self.calls.borrow_mut() += 1;
        if transaction.expected_preview_witness_hash != self.current_preview {
            return Err(GeneratedSkillCommitError::StaleWitness);
        }
        if self.collision {
            return Err(GeneratedSkillCommitError::Collision);
        }
        if *self.fail_once.borrow() {
            *self.fail_once.borrow_mut() = false;
            return Err(GeneratedSkillCommitError::Storage);
        }
        Ok(GeneratedSkillCommitResultV1 {
            skill_id: transaction.expected_candidate_id.clone(),
            revision_hash: sha256_bytes(transaction.rendered_skill_md.as_bytes()),
        })
    }
}

#[test]
fn approved_creation_uses_witnessed_transaction_and_recovers_after_storage_failure() {
    let package = package(true);
    let request = GeneratedSkillCommitRequestV1 {
        package: &package,
        rendered_skill_md: &package.payload.rendered_artifact.content,
        interactive_approved: true,
    };
    let port = CreationPort {
        current_preview: "sha256:preview".into(),
        collision: false,
        fail_once: RefCell::new(true),
        calls: RefCell::new(0),
    };
    assert_eq!(
        commit_approved_generated_skill(&port, &request),
        Err(GeneratedSkillCommitError::Storage)
    );
    let committed = commit_approved_generated_skill(&port, &request).expect("recovered commit");
    assert_eq!(committed.skill_id, "focused-review");
    assert_eq!(*port.calls.borrow(), 2);
}

#[test]
fn creation_requires_interactive_approval_and_current_collision_free_preview() {
    let package = package(true);
    let denied = GeneratedSkillCommitRequestV1 {
        package: &package,
        rendered_skill_md: &package.payload.rendered_artifact.content,
        interactive_approved: false,
    };
    let port = CreationPort {
        current_preview: "sha256:preview".into(),
        collision: false,
        fail_once: RefCell::new(false),
        calls: RefCell::new(0),
    };
    assert_eq!(
        commit_approved_generated_skill(&port, &denied),
        Err(GeneratedSkillCommitError::NotApproved)
    );
    assert_eq!(*port.calls.borrow(), 0);

    let approved = GeneratedSkillCommitRequestV1 {
        interactive_approved: true,
        ..denied
    };
    let stale = CreationPort {
        current_preview: "sha256:changed".into(),
        collision: false,
        fail_once: RefCell::new(false),
        calls: RefCell::new(0),
    };
    assert_eq!(
        commit_approved_generated_skill(&stale, &approved),
        Err(GeneratedSkillCommitError::StaleWitness)
    );
    let collision = CreationPort {
        current_preview: "sha256:preview".into(),
        collision: true,
        fail_once: RefCell::new(false),
        calls: RefCell::new(0),
    };
    assert_eq!(
        commit_approved_generated_skill(&collision, &approved),
        Err(GeneratedSkillCommitError::Collision)
    );
}

fn package(
    new_skill: bool,
) -> crate::contexts::skill_evolution_generation::domain::PreparedGenerationReviewPackageV1 {
    let (kind, target, draft, quarantine, package_id) = if new_skill {
        let draft = StructuredDraftV1::NewSkill {
            candidate_id: "focused-review".into(),
            name: "Focused review".into(),
            description: "Review one bounded document.".into(),
            skill_type: "utility".into(),
            version: "1.0.0".into(),
            built_in_tools: Vec::new(),
            instructions: "Review the document and return bounded findings.".into(),
        };
        (
            GeneratedArtifactKind::NewSkill,
            MutationTargetV1::NewSkill {
                candidate_id: "focused-review".into(),
                scope: "project".into(),
                workspace_id: Some("workspace-one".into()),
            },
            draft,
            true,
            "package-creation",
        )
    } else {
        (
            GeneratedArtifactKind::OverlayLearnBlock,
            MutationTargetV1::ExistingSkill {
                skill_id: "review-skill".into(),
                effective_revision: "revision-one".into(),
                overlay_scope: "project".into(),
            },
            StructuredDraftV1::OverlayLearnBlock {
                guidance: "Validate the bounded result.".into(),
            },
            false,
            "package-existing",
        )
    };
    let plan = plan(kind, target);
    let artifact = render_generation_artifact(&GenerationRenderRequestV1 {
        artifact_id: if new_skill {
            "artifact-creation"
        } else {
            "artifact-existing"
        },
        expected_kind: kind,
        draft: &draft,
        allowed_built_in_tools: &BTreeSet::new(),
    })
    .expect("render package");
    let validation = validation(&artifact.artifact_id, "validation-one");
    let calls = vec![model_call()];
    let metadata = GeneratedReviewPackageV1 {
        package_id: package_id.into(),
        job_id: "job-one".into(),
        attempt_id: "attempt-one".into(),
        dossier_id: "dossier-one".into(),
        plan_hash: canonical_hash(&plan).expect("plan hash"),
        artifact_id: artifact.artifact_id.clone(),
        validation_id: validation.validation_id.clone(),
        preview_witness_hash: Some("sha256:preview".into()),
        policy_hash: "sha256:policy".into(),
        consent_hash: "sha256:consent".into(),
        model_provenance_hash: canonical_hash(&calls).expect("calls hash"),
        permanently_manual: true,
        handoff_status: GenerationHandoffStatus::Pending,
        curator_candidate_id: None,
        created_at_ms: 10,
    };
    let quarantine = quarantine.then(|| QuarantinedSkillProposalV1 {
        proposal_id: "proposal-one".into(),
        job_id: "job-one".into(),
        status: GenerationQuarantineStatus::Quarantined,
        candidate_id: "focused-review".into(),
        scope: "project".into(),
        workspace_id: Some("workspace-one".into()),
        artifact_hash: artifact.content_hash.clone(),
        catalog_witness_hash: "sha256:catalog".into(),
        revision: 1,
    });
    build_review_package(BuildReviewPackageRequestV1 {
        package: metadata,
        dossier_revision: 1,
        dossier_hash: "sha256:dossier",
        plan,
        structured_draft: draft,
        rendered_artifact: artifact,
        validation,
        model_calls: calls,
        quarantine,
    })
    .expect("package")
}

fn validation(artifact_id: &str, validation_id: &str) -> GenerationValidationV1 {
    GenerationValidationV1 {
        validation_id: validation_id.into(),
        artifact_id: artifact_id.into(),
        validator_version: "validator-v1".into(),
        status: GenerationValidationStatus::Passed,
        checks: Vec::new(),
        preview_witness_hash: Some("sha256:preview".into()),
        report_hash: "sha256:validation".into(),
    }
}

fn model_call() -> GenerationModelCallRecordV1 {
    GenerationModelCallRecordV1 {
        model_call_id: "model-call".into(),
        stage_attempt_id: "attempt-one".into(),
        purpose: "skill_evolution_generation".into(),
        provider_protocol: Some("openai-compatible".into()),
        provider_profile_id: Some("profile".into()),
        model_id: Some("model".into()),
        prompt_template_version: "prompt-v1".into(),
        response_schema_version: "response-v1".into(),
        outcome: GenerationModelOutcome::Valid,
        input_tokens: 10,
        output_tokens: 5,
        latency_ms: 2,
        structured_response_hash: Some("sha256:response".into()),
        safe_failure_code: None,
        created_at_ms: 2,
    }
}

fn plan(kind: GeneratedArtifactKind, target: MutationTargetV1) -> MutationPlanV1 {
    MutationPlanV1 {
        schema_version: 1,
        plan_id: "plan-one".into(),
        artifact_kind: kind,
        target,
        rationale: "Bounded evidence supports review.".into(),
        lesson: GeneratedLessonShapeV1 {
            trigger: "When reviewing".into(),
            action: "Validate".into(),
            verification: "Confirm".into(),
        },
        evidence_citations: vec![GenerationCitationV1 {
            claim_id: "lesson.trigger".into(),
            dossier_section: "source_signal_inventory".into(),
            source_id: "signal-one".into(),
        }],
        expected_behavior: "Validated result".into(),
        verification_steps: vec![GeneratedVerificationStepV1 {
            step_id: "verify".into(),
            action_code: "validate".into(),
            expected_code: "pass".into(),
            citation_ids: vec!["signal-one".into()],
        }],
        content_hash: "sha256:plan-content".into(),
    }
}
