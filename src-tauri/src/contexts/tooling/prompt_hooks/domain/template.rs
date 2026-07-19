use super::PromptHookDomainError;

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

    pub(crate) fn render(&self, agent_id: &str, sample_input: &str) -> String {
        render_prompt_template(&self.0, agent_id, sample_input)
    }
}

pub(crate) fn render_prompt_template(template: &str, agent_id: &str, sample_input: &str) -> String {
    template
        .replace("{{agentId}}", agent_id)
        .replace("{{sampleInput}}", sample_input)
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
    fn rendering_interpolates_only_the_two_declared_placeholders() {
        let template = PromptHookTemplate::new(
            "Agent {{agentId}} receives {{sampleInput}} and keeps {{unknown}}",
        )
        .expect("template");
        assert_eq!(
            template.render("codex-cli", "review"),
            "Agent codex-cli receives review and keeps {{unknown}}"
        );
    }

    #[test]
    fn rendering_treats_script_syntax_as_inert_literal_text() {
        let template = PromptHookTemplate::new(
            "$(Set-Content marker.txt executed); `printf executed`; {{agentId}}; {{sampleInput}}",
        )
        .expect("template");

        assert_eq!(
            template.render("codex-cli", "&& touch marker.txt | Out-File marker.txt"),
            "$(Set-Content marker.txt executed); `printf executed`; codex-cli; && touch marker.txt | Out-File marker.txt"
        );
    }
}
