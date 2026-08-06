use crate::contexts::agent_runtime::application::{
    AgentPersonalizationPort, AgentRuntimeApplicationError, PersonalizationSettings,
};
use crate::contexts::desktop::api::DesktopSettingsApi;

/// Wraps `desktop`'s settings facade to satisfy `agent_runtime`'s own `AgentPersonalizationPort`
/// (`add-personalization-settings`) — mirrors `RuntimeAgentSkillAdapter`'s existing pattern for
/// depending on another context's API through an `agent_runtime`-owned port rather than that
/// context's types directly.
#[derive(Clone)]
pub(crate) struct RuntimeAgentPersonalizationAdapter {
    settings: DesktopSettingsApi,
}

impl RuntimeAgentPersonalizationAdapter {
    pub(crate) fn new(settings: DesktopSettingsApi) -> Self {
        Self { settings }
    }
}

impl AgentPersonalizationPort for RuntimeAgentPersonalizationAdapter {
    fn settings(&self) -> Result<PersonalizationSettings, AgentRuntimeApplicationError> {
        let view = self
            .settings
            .get_settings()
            .map_err(|error| AgentRuntimeApplicationError::Personalization(error.to_string()))?;
        let settings = &view.settings;
        Ok(PersonalizationSettings {
            custom_instructions_about_user: settings.custom_instructions_about_user().to_string(),
            custom_instructions_style_rules: settings.custom_instructions_style_rules().to_string(),
            custom_instructions_enabled: settings.custom_instructions_enabled(),
            memory_enabled: settings.memory_enabled(),
            memory_tool_assisted_chats_enabled: settings.memory_tool_assisted_chats_enabled(),
        })
    }
}
