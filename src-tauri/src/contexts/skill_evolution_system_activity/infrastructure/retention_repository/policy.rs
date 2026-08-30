use super::EvolutionSourceDomain;

pub(super) fn source_identity_kind(domain: EvolutionSourceDomain) -> Option<&'static str> {
    match domain {
        EvolutionSourceDomain::Orchestration => Some("run"),
        EvolutionSourceDomain::Evidence => Some("evidence"),
        EvolutionSourceDomain::Assessment => Some("assessment"),
        EvolutionSourceDomain::Generation => Some("generation_job"),
        EvolutionSourceDomain::Curator => Some("curator_candidate"),
        EvolutionSourceDomain::Probation => Some("probation"),
        EvolutionSourceDomain::Breaker => Some("breaker"),
        EvolutionSourceDomain::SkillCreation => Some("skill"),
        EvolutionSourceDomain::Overlay
        | EvolutionSourceDomain::AutomaticApplication
        | EvolutionSourceDomain::Recovery
        | EvolutionSourceDomain::Retention => None,
    }
}

pub(super) fn preserves_committed_outcome(event_code: &str) -> bool {
    matches!(
        event_code,
        "overlay_applied" | "automatic_applied" | "skill_created" | "source_purged"
    )
}
