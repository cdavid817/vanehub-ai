use crate::contexts::permissions::api::PolicyTemplateName;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SessionExecutionMode {
    Inherit,
    Plan,
    Execute,
}

impl SessionExecutionMode {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "inherit" => Some(Self::Inherit),
            "plan" => Some(Self::Plan),
            "execute" => Some(Self::Execute),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EffectiveExecutionPolicy {
    Readonly,
    Ask,
    Allow,
}

impl EffectiveExecutionPolicy {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Readonly => "readonly",
            Self::Ask => "ask",
            Self::Allow => "allow",
        }
    }

    pub(crate) fn launch_template(self) -> PolicyTemplateName {
        match self {
            Self::Readonly => PolicyTemplateName::Readonly,
            Self::Ask => PolicyTemplateName::Standard,
            Self::Allow => PolicyTemplateName::Trusted,
        }
    }
}

pub(crate) fn resolve_effective_execution_policy(
    template: PolicyTemplateName,
    mode: SessionExecutionMode,
) -> EffectiveExecutionPolicy {
    if mode == SessionExecutionMode::Plan || template == PolicyTemplateName::Readonly {
        return EffectiveExecutionPolicy::Readonly;
    }
    match template {
        PolicyTemplateName::Standard => EffectiveExecutionPolicy::Ask,
        PolicyTemplateName::Trusted | PolicyTemplateName::Yolo => EffectiveExecutionPolicy::Allow,
        PolicyTemplateName::Readonly => EffectiveExecutionPolicy::Readonly,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_matrix_never_widens_the_agent_template() {
        let cases = [
            (
                PolicyTemplateName::Readonly,
                SessionExecutionMode::Inherit,
                EffectiveExecutionPolicy::Readonly,
            ),
            (
                PolicyTemplateName::Readonly,
                SessionExecutionMode::Plan,
                EffectiveExecutionPolicy::Readonly,
            ),
            (
                PolicyTemplateName::Readonly,
                SessionExecutionMode::Execute,
                EffectiveExecutionPolicy::Readonly,
            ),
            (
                PolicyTemplateName::Standard,
                SessionExecutionMode::Inherit,
                EffectiveExecutionPolicy::Ask,
            ),
            (
                PolicyTemplateName::Standard,
                SessionExecutionMode::Plan,
                EffectiveExecutionPolicy::Readonly,
            ),
            (
                PolicyTemplateName::Standard,
                SessionExecutionMode::Execute,
                EffectiveExecutionPolicy::Ask,
            ),
            (
                PolicyTemplateName::Trusted,
                SessionExecutionMode::Inherit,
                EffectiveExecutionPolicy::Allow,
            ),
            (
                PolicyTemplateName::Trusted,
                SessionExecutionMode::Plan,
                EffectiveExecutionPolicy::Readonly,
            ),
            (
                PolicyTemplateName::Trusted,
                SessionExecutionMode::Execute,
                EffectiveExecutionPolicy::Allow,
            ),
            (
                PolicyTemplateName::Yolo,
                SessionExecutionMode::Inherit,
                EffectiveExecutionPolicy::Allow,
            ),
            (
                PolicyTemplateName::Yolo,
                SessionExecutionMode::Plan,
                EffectiveExecutionPolicy::Readonly,
            ),
            (
                PolicyTemplateName::Yolo,
                SessionExecutionMode::Execute,
                EffectiveExecutionPolicy::Allow,
            ),
        ];

        for (template, mode, expected) in cases {
            assert_eq!(resolve_effective_execution_policy(template, mode), expected);
        }
    }

    #[test]
    fn removed_execution_modes_are_rejected() {
        for value in ["default", "agent", "auto", ""] {
            assert_eq!(SessionExecutionMode::parse(value), None);
        }
    }
}
