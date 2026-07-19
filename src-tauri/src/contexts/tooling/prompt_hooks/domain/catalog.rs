use super::{PromptHookCategory, PromptHookStage};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BuiltinPromptHookDefinition {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) category: PromptHookCategory,
    pub(crate) stage: PromptHookStage,
    pub(crate) order: i64,
    pub(crate) enabled: bool,
    pub(crate) disableable: bool,
    pub(crate) template_body: &'static str,
}

const BUILTIN_PROMPT_HOOKS: [BuiltinPromptHookDefinition; 7] = [
    BuiltinPromptHookDefinition {
        id: "bootstrap-session-context",
        name: "Session Context",
        description: "Adds session and workspace context to each CLI prompt.",
        category: PromptHookCategory::Bootstrap,
        stage: PromptHookStage::SessionInit,
        order: 100,
        enabled: true,
        disableable: true,
        template_body: "Session context: {{sampleInput}}",
    },
    BuiltinPromptHookDefinition {
        id: "law-runtime-boundary",
        name: "Runtime Boundary",
        description: "Keeps CLI behavior inside VaneHub runtime and permission boundaries.",
        category: PromptHookCategory::Law,
        stage: PromptHookStage::SessionInit,
        order: 200,
        enabled: true,
        disableable: false,
        template_body: "Respect the active VaneHub runtime, permissions, and project boundaries.",
    },
    BuiltinPromptHookDefinition {
        id: "static-response-format",
        name: "Response Format",
        description: "Sets a concise engineering response baseline.",
        category: PromptHookCategory::Static,
        stage: PromptHookStage::SessionInit,
        order: 300,
        enabled: true,
        disableable: true,
        template_body:
            "Use direct, actionable engineering responses with concise verification notes.",
    },
    BuiltinPromptHookDefinition {
        id: "dynamic-session-config",
        name: "Session Configuration",
        description: "Summarizes active session configuration for the selected CLI.",
        category: PromptHookCategory::Dynamic,
        stage: PromptHookStage::PerTurn,
        order: 400,
        enabled: true,
        disableable: true,
        template_body: "Active CLI: {{agentId}}. User request follows after the hook context.",
    },
    BuiltinPromptHookDefinition {
        id: "navigation-project-hints",
        name: "Project Navigation",
        description: "Encourages grounded project inspection before code changes.",
        category: PromptHookCategory::Navigation,
        stage: PromptHookStage::PerTurn,
        order: 500,
        enabled: true,
        disableable: true,
        template_body:
            "Inspect relevant project files and existing patterns before making changes.",
    },
    BuiltinPromptHookDefinition {
        id: "routing-cli-capabilities",
        name: "CLI Capability Routing",
        description: "Keeps behavior aligned with the selected CLI agent capabilities.",
        category: PromptHookCategory::Routing,
        stage: PromptHookStage::PerTurn,
        order: 600,
        enabled: true,
        disableable: true,
        template_body: "Route work through capabilities available to {{agentId}}.",
    },
    BuiltinPromptHookDefinition {
        id: "callback-future-channel",
        name: "Callback Channel Placeholder",
        description: "Reserved placeholder for future callback-aware workflows.",
        category: PromptHookCategory::Callback,
        stage: PromptHookStage::PerTurn,
        order: 700,
        enabled: false,
        disableable: true,
        template_body: "Callback channel support is not active in this runtime.",
    },
];

pub(crate) fn builtin_prompt_hooks() -> &'static [BuiltinPromptHookDefinition] {
    &BUILTIN_PROMPT_HOOKS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::prompt_hooks::domain::{
        PromptHookId, PromptHookOrder, PromptHookTemplate,
    };
    use std::collections::HashSet;

    #[test]
    fn catalog_contains_the_seven_stable_backend_owned_hooks() {
        let hooks = builtin_prompt_hooks();
        assert_eq!(hooks.len(), 7);
        assert_eq!(
            hooks.iter().map(|hook| hook.id).collect::<Vec<_>>(),
            [
                "bootstrap-session-context",
                "law-runtime-boundary",
                "static-response-format",
                "dynamic-session-config",
                "navigation-project-hints",
                "routing-cli-capabilities",
                "callback-future-channel"
            ]
        );
        assert_eq!(
            hooks
                .iter()
                .map(|hook| hook.category)
                .collect::<HashSet<_>>(),
            PromptHookCategory::ALL.into_iter().collect()
        );
        for hook in hooks {
            PromptHookId::parse(hook.id).expect("catalog id");
            PromptHookOrder::new(hook.order).expect("catalog order");
            PromptHookTemplate::new(hook.template_body).expect("catalog template");
        }
    }

    #[test]
    fn runtime_boundary_is_the_only_non_disableable_builtin() {
        let immutable = builtin_prompt_hooks()
            .iter()
            .filter(|hook| !hook.disableable)
            .collect::<Vec<_>>();
        assert_eq!(immutable.len(), 1);
        assert_eq!(immutable[0].id, "law-runtime-boundary");
        assert!(immutable[0].enabled);
    }
}
