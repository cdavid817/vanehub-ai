use super::PromptHookDomainError;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PromptHookVariableDefinition {
    pub(crate) name: &'static str,
    pub(crate) description_key: &'static str,
    pub(crate) availability_key: &'static str,
    pub(crate) example: &'static str,
}

pub(crate) const PROMPT_HOOK_VARIABLES: [PromptHookVariableDefinition; 5] = [
    PromptHookVariableDefinition {
        name: "agent_id",
        description_key: "promptHooks.variables.agentId",
        availability_key: "promptHooks.variables.availability.agent",
        example: "codex-cli",
    },
    PromptHookVariableDefinition {
        name: "agent_name",
        description_key: "promptHooks.variables.agentName",
        availability_key: "promptHooks.variables.availability.agent",
        example: "Codex CLI",
    },
    PromptHookVariableDefinition {
        name: "current_time",
        description_key: "promptHooks.variables.currentTime",
        availability_key: "promptHooks.variables.availability.invocation",
        example: "2026-07-23T12:00:00Z",
    },
    PromptHookVariableDefinition {
        name: "sample_input",
        description_key: "promptHooks.variables.sampleInput",
        availability_key: "promptHooks.variables.availability.input",
        example: "Review the current change.",
    },
    PromptHookVariableDefinition {
        name: "session_id",
        description_key: "promptHooks.variables.sessionId",
        availability_key: "promptHooks.variables.availability.session",
        example: "session-preview",
    },
];

#[derive(Debug, Clone, Copy)]
pub(crate) struct PromptHookTemplateContext<'a> {
    pub(crate) agent_id: &'a str,
    pub(crate) agent_name: &'a str,
    pub(crate) current_time: &'a str,
    pub(crate) sample_input: &'a str,
    pub(crate) session_id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookTemplate(String);

impl PromptHookTemplate {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, PromptHookDomainError> {
        let value = value.into();
        if value
            .chars()
            .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
        {
            Err(PromptHookDomainError::UnsupportedControlCharacter)
        } else {
            Ok(Self(value))
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn unknown_variables(&self) -> Vec<String> {
        unknown_prompt_variables(&self.0)
    }

    pub(crate) fn validate_variables(&self) -> Result<(), PromptHookDomainError> {
        let unknown = self.unknown_variables();
        if unknown.is_empty() {
            Ok(())
        } else {
            Err(PromptHookDomainError::UnknownVariables(unknown))
        }
    }

    pub(crate) fn render(
        &self,
        context: PromptHookTemplateContext<'_>,
    ) -> Result<String, PromptHookDomainError> {
        self.validate_variables()?;
        Ok(render_prompt_template(&self.0, context))
    }
}

fn unknown_prompt_variables(template: &str) -> Vec<String> {
    let mut unknown = BTreeSet::new();
    let mut remainder = template;
    while let Some(start) = remainder.find("{{") {
        let after_start = &remainder[start + 2..];
        let Some(end) = after_start.find("}}") else {
            break;
        };
        let name = after_start[..end].trim();
        if !is_supported_variable(name) {
            unknown.insert(name.to_string());
        }
        remainder = &after_start[end + 2..];
    }
    unknown.into_iter().collect()
}

fn is_supported_variable(name: &str) -> bool {
    PROMPT_HOOK_VARIABLES
        .iter()
        .any(|definition| definition.name == name)
        || matches!(name, "agentId" | "sampleInput")
}

pub(crate) fn render_prompt_template(
    template: &str,
    context: PromptHookTemplateContext<'_>,
) -> String {
    [
        ("agent_id", context.agent_id),
        ("agent_name", context.agent_name),
        ("current_time", context.current_time),
        ("sample_input", context.sample_input),
        ("session_id", context.session_id),
        ("agentId", context.agent_id),
        ("sampleInput", context.sample_input),
    ]
    .into_iter()
    .fold(template.to_string(), |rendered, (name, value)| {
        rendered.replace(&format!("{{{{{name}}}}}"), value)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_rejects_unsupported_controls_but_preserves_layout_characters() {
        let template = PromptHookTemplate::new("line one\n\tline two").expect("template");
        assert_eq!(template.as_str(), "line one\n\tline two");
        assert_eq!(
            PromptHookTemplate::new("unsafe\u{0000}"),
            Err(PromptHookDomainError::UnsupportedControlCharacter)
        );
    }

    #[test]
    fn rendering_interpolates_canonical_variables_and_legacy_aliases() {
        let template = PromptHookTemplate::new(
            "{{agent_name}} ({{agent_id}}/{{agentId}}) at {{current_time}} receives \
             {{sample_input}}/{{sampleInput}} in {{session_id}}",
        )
        .expect("template");
        assert_eq!(
            template.render(PromptHookTemplateContext {
                agent_id: "codex-cli",
                agent_name: "Codex CLI",
                current_time: "2026-07-23T12:00:00Z",
                sample_input: "review",
                session_id: "session-1",
            }),
            Ok(
                "Codex CLI (codex-cli/codex-cli) at 2026-07-23T12:00:00Z receives \
                 review/review in session-1"
                    .to_string()
            )
        );
    }

    #[test]
    fn unknown_variables_are_sorted_and_rejected() {
        let template = PromptHookTemplate::new("{{z_unknown}} {{agent_name}} {{a_unknown}}")
            .expect("template");
        assert_eq!(
            template.unknown_variables(),
            ["a_unknown".to_string(), "z_unknown".to_string()]
        );
        assert_eq!(
            template.validate_variables(),
            Err(PromptHookDomainError::UnknownVariables(vec![
                "a_unknown".to_string(),
                "z_unknown".to_string()
            ]))
        );
    }

    #[test]
    fn rendering_treats_script_syntax_as_inert_literal_text() {
        let template = PromptHookTemplate::new(
            "$(Set-Content marker.txt executed); `printf executed`; {{agentId}}; {{sampleInput}}",
        )
        .expect("template");

        assert_eq!(
            template.render(PromptHookTemplateContext {
                agent_id: "codex-cli",
                agent_name: "Codex CLI",
                current_time: "2026-07-23T12:00:00Z",
                sample_input: "&& touch marker.txt | Out-File marker.txt",
                session_id: "session-1",
            }),
            Ok("$(Set-Content marker.txt executed); `printf executed`; codex-cli; && touch marker.txt | Out-File marker.txt".to_string())
        );
    }
}
