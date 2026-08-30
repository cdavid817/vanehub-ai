use super::*;
use crate::contexts::skill_evolution_orchestration::domain::{
    EvolutionActorProvenance, EvolutionTriggerFamily, EVOLUTION_TRIGGER_FAMILIES_V1,
};

#[test]
fn authoritative_projectors_cover_exactly_the_closed_trigger_registry() {
    let mut projected = vec![
        EvolutionTriggerProjectorV1::startup_recovery(source("startup")).expect("startup"),
        EvolutionTriggerProjectorV1::periodic_maintenance(source("periodic")).expect("periodic"),
        EvolutionTriggerProjectorV1::application_idle_transition(source("idle")).expect("idle"),
        EvolutionTriggerProjectorV1::explicit_feedback_commit(source("feedback"))
            .expect("feedback"),
        EvolutionTriggerProjectorV1::relevant_mutation(
            RelevantMutationKindV1::Skill,
            source("skill"),
        )
        .expect("skill"),
        EvolutionTriggerProjectorV1::manual_run_request(source("manual")).expect("manual"),
    ];
    for (kind, id) in [
        (RuntimeCompletionKindV1::AgentRun, "run"),
        (RuntimeCompletionKindV1::Conversation, "conversation"),
        (RuntimeCompletionKindV1::Verification, "verification"),
        (RuntimeCompletionKindV1::DelegatedUtility, "utility"),
    ] {
        projected.push(
            EvolutionTriggerProjectorV1::runtime_completion(kind, source(id))
                .expect("runtime completion"),
        );
    }
    projected.sort_by_key(|trigger| trigger.family.as_str());
    let mut families: Vec<_> = EVOLUTION_TRIGGER_FAMILIES_V1
        .into_iter()
        .map(EvolutionTriggerFamily::as_str)
        .collect();
    families.sort_unstable();
    assert_eq!(
        projected
            .iter()
            .map(|trigger| trigger.family.as_str())
            .collect::<Vec<_>>(),
        families
    );
}

#[test]
fn projection_is_deterministic_and_keeps_actor_provenance() {
    let first = EvolutionTriggerProjectorV1::manual_run_request(source("manual")).expect("first");
    let second = EvolutionTriggerProjectorV1::manual_run_request(source("manual")).expect("second");
    assert_eq!(first, second);
    assert_eq!(first.actor, EvolutionActorProvenance::InteractiveUser);
    assert_eq!(first.safe_reason_codes, ["manual-run-requested"]);
}

#[test]
fn projection_rejects_unsafe_or_time_invalid_authoritative_sources() {
    let mut invalid = source("run");
    invalid.workspace_id = "unsafe workspace".into();
    assert_eq!(
        EvolutionTriggerProjectorV1::runtime_completion(RuntimeCompletionKindV1::AgentRun, invalid,),
        Err(TriggerProjectionError::InvalidSource)
    );
    let mut invalid = source("run");
    invalid.occurred_at_ms = -1;
    assert_eq!(
        EvolutionTriggerProjectorV1::runtime_completion(RuntimeCompletionKindV1::AgentRun, invalid,),
        Err(TriggerProjectionError::InvalidSource)
    );
}

#[test]
fn relevant_mutation_projects_skill_overlay_and_policy_without_new_families() {
    for (kind, expected_source_kind) in [
        (RelevantMutationKindV1::Skill, "skill-revision"),
        (RelevantMutationKindV1::Overlay, "overlay-revision"),
        (RelevantMutationKindV1::Policy, "policy-revision"),
    ] {
        let trigger = EvolutionTriggerProjectorV1::relevant_mutation(kind, source("revision"))
            .expect("mutation");
        assert_eq!(
            trigger.family,
            EvolutionTriggerFamily::RelevantPolicyOrSkillChange
        );
        assert_eq!(trigger.source_kind, expected_source_kind);
    }
}

fn source(id: &str) -> AuthoritativeTriggerSourceV1 {
    AuthoritativeTriggerSourceV1 {
        workspace_id: "workspace-one".into(),
        source_id: id.into(),
        source_revision: 1,
        occurred_at_ms: 100,
    }
}
