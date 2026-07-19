use super::SkillDomainError;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillBindingPlan {
    pub(crate) desired_agent_ids: Vec<String>,
    pub(crate) bind: Vec<String>,
    pub(crate) unbind: Vec<String>,
    pub(crate) mount: Vec<String>,
    pub(crate) unmount: Vec<String>,
}

pub(crate) fn plan_binding_change(
    current_agent_ids: &[String],
    requested_agent_ids: &[String],
    registered_agent_ids: &BTreeSet<String>,
    enabled: bool,
) -> Result<SkillBindingPlan, SkillDomainError> {
    let desired = requested_agent_ids
        .iter()
        .map(|agent_id| agent_id.trim().to_string())
        .collect::<BTreeSet<_>>();
    if let Some(agent_id) = desired
        .iter()
        .find(|agent_id| !registered_agent_ids.contains(*agent_id))
    {
        return Err(SkillDomainError::UnknownAgent(agent_id.clone()));
    }
    let current = current_agent_ids.iter().cloned().collect::<BTreeSet<_>>();
    Ok(SkillBindingPlan {
        desired_agent_ids: desired.iter().cloned().collect(),
        bind: desired.difference(&current).cloned().collect(),
        unbind: current.difference(&desired).cloned().collect(),
        mount: if enabled {
            desired.iter().cloned().collect()
        } else {
            Vec::new()
        },
        unmount: current.difference(&desired).cloned().collect(),
    })
}

pub(crate) fn plan_enablement(bound_agent_ids: &[String], enabled: bool) -> SkillBindingPlan {
    let bound = bound_agent_ids.iter().cloned().collect::<BTreeSet<_>>();
    SkillBindingPlan {
        desired_agent_ids: bound.iter().cloned().collect(),
        bind: Vec::new(),
        unbind: Vec::new(),
        mount: if enabled {
            bound.iter().cloned().collect()
        } else {
            Vec::new()
        },
        unmount: if enabled {
            Vec::new()
        } else {
            bound.iter().cloned().collect()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registered() -> BTreeSet<String> {
        ["claude-code", "codex-cli", "gemini-cli", "opencode"]
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    #[test]
    fn binding_plan_deduplicates_sorts_and_repairs_all_desired_enabled_mounts() {
        let plan = plan_binding_change(
            &["claude-code".to_string(), "gemini-cli".to_string()],
            &[
                "codex-cli".to_string(),
                "claude-code".to_string(),
                "codex-cli".to_string(),
            ],
            &registered(),
            true,
        )
        .expect("plan");
        assert_eq!(plan.desired_agent_ids, vec!["claude-code", "codex-cli"]);
        assert_eq!(plan.bind, vec!["codex-cli"]);
        assert_eq!(plan.unbind, vec!["gemini-cli"]);
        assert_eq!(plan.mount, vec!["claude-code", "codex-cli"]);
        assert_eq!(plan.unmount, vec!["gemini-cli"]);
    }

    #[test]
    fn disabled_binding_plan_does_not_mount_and_unknown_agents_are_rejected() {
        let plan = plan_binding_change(&[], &["codex-cli".to_string()], &registered(), false)
            .expect("plan");
        assert!(plan.mount.is_empty());
        assert_eq!(plan.bind, vec!["codex-cli"]);

        assert_eq!(
            plan_binding_change(&[], &["unknown".to_string()], &registered(), true),
            Err(SkillDomainError::UnknownAgent("unknown".to_string()))
        );
    }

    #[test]
    fn enablement_mounts_or_unmounts_every_existing_binding() {
        let agents = vec!["codex-cli".to_string(), "claude-code".to_string()];
        assert_eq!(
            plan_enablement(&agents, true).mount,
            vec!["claude-code", "codex-cli"]
        );
        assert_eq!(
            plan_enablement(&agents, false).unmount,
            vec!["claude-code", "codex-cli"]
        );
    }
}
