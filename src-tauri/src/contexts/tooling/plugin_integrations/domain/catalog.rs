use super::PluginIntegrationDomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum PluginIntegrationId {
    Github,
}

impl PluginIntegrationId {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Github => "github",
        }
    }

    pub(crate) fn parse(value: &str) -> Result<Self, PluginIntegrationDomainError> {
        match value {
            "github" => Ok(Self::Github),
            _ => Err(PluginIntegrationDomainError::UnknownIntegration(
                value.to_string(),
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PluginIntegrationSetupStep {
    pub(crate) id: &'static str,
    pub(crate) label_key: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PluginIntegrationDefinition {
    pub(crate) id: PluginIntegrationId,
    pub(crate) name_key: &'static str,
    pub(crate) description_key: &'static str,
    pub(crate) version: &'static str,
    pub(crate) provider: &'static str,
    pub(crate) icon: &'static str,
    pub(crate) docs_url: &'static str,
    pub(crate) setup_steps: &'static [PluginIntegrationSetupStep],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PluginIntegrationToolPlan {
    pub(crate) integration_id: PluginIntegrationId,
    pub(crate) executable: &'static str,
    pub(crate) arguments: &'static [&'static str],
    pub(crate) timeout_seconds: u64,
}

const GITHUB_SETUP_STEPS: [PluginIntegrationSetupStep; 2] = [
    PluginIntegrationSetupStep {
        id: "install",
        label_key: "plugins.github.setup.install",
    },
    PluginIntegrationSetupStep {
        id: "auth",
        label_key: "plugins.github.setup.auth",
    },
];

const DEFINITIONS: [PluginIntegrationDefinition; 1] = [PluginIntegrationDefinition {
    id: PluginIntegrationId::Github,
    name_key: "plugins.github.name",
    description_key: "plugins.github.description",
    version: "1.0.0",
    provider: "GitHub",
    icon: "github",
    docs_url: "https://cli.github.com/manual/gh_auth_login",
    setup_steps: &GITHUB_SETUP_STEPS,
}];

const GITHUB_READINESS_PLAN: PluginIntegrationToolPlan = PluginIntegrationToolPlan {
    integration_id: PluginIntegrationId::Github,
    executable: "gh",
    arguments: &["auth", "status"],
    timeout_seconds: 10,
};

pub(crate) fn definitions() -> &'static [PluginIntegrationDefinition] {
    &DEFINITIONS
}

pub(crate) fn readiness_plan(id: PluginIntegrationId) -> PluginIntegrationToolPlan {
    match id {
        PluginIntegrationId::Github => GITHUB_READINESS_PLAN,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_contains_only_the_stable_builtin_github_definition() {
        assert_eq!(definitions().len(), 1);
        let github = definitions()[0];
        assert_eq!(github.id.as_str(), "github");
        assert_eq!(github.name_key, "plugins.github.name");
        assert_eq!(github.description_key, "plugins.github.description");
        assert_eq!(github.version, "1.0.0");
        assert_eq!(github.provider, "GitHub");
        assert_eq!(github.icon, "github");
        assert_eq!(
            github.docs_url,
            "https://cli.github.com/manual/gh_auth_login"
        );
        assert_eq!(
            github
                .setup_steps
                .iter()
                .map(|step| (step.id, step.label_key))
                .collect::<Vec<_>>(),
            vec![
                ("install", "plugins.github.setup.install"),
                ("auth", "plugins.github.setup.auth")
            ]
        );
    }

    #[test]
    fn unknown_ids_are_rejected_and_readiness_uses_a_fixed_allowlisted_plan() {
        assert_eq!(
            PluginIntegrationId::parse("github"),
            Ok(PluginIntegrationId::Github)
        );
        assert!(matches!(
            PluginIntegrationId::parse("gitlab"),
            Err(PluginIntegrationDomainError::UnknownIntegration(value)) if value == "gitlab"
        ));

        let plan = readiness_plan(PluginIntegrationId::Github);
        assert_eq!(plan.executable, "gh");
        assert_eq!(plan.arguments, &["auth", "status"]);
        assert_eq!(plan.timeout_seconds, 10);
    }
}
