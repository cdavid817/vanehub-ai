use crate::contexts::sessions::application::{
    ChatConfigurationValues, SessionChatProfilePort, SessionsApplicationError,
};
use crate::contexts::sessions::domain::normalize_reasoning;
use crate::contexts::tooling::cli::application::native_config::NativeConfigPort;
use crate::contexts::tooling::cli_parameters::CliParametersApi;
use crate::platform::database::NativeDatabase;
use rusqlite::OptionalExtension;
use serde_json::Value;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SqliteSessionChatProfileAdapter {
    database: NativeDatabase,
    cli_parameters: CliParametersApi,
    native_config: Arc<dyn NativeConfigPort>,
}

impl SqliteSessionChatProfileAdapter {
    pub(crate) fn new(
        database: NativeDatabase,
        cli_parameters: CliParametersApi,
        native_config: Arc<dyn NativeConfigPort>,
    ) -> Self {
        Self {
            database,
            cli_parameters,
            native_config,
        }
    }
}

impl SessionChatProfilePort for SqliteSessionChatProfileAdapter {
    fn defaults_for(
        &self,
        agent_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<ChatConfigurationValues, SessionsApplicationError> {
        if agent_id == "onepiece" {
            return self.onepiece_defaults();
        }
        let selections = self
            .cli_parameters
            .load_selections(agent_id)
            .map_err(profile_error)?;
        let model_id = selections
            .get("model")
            .and_then(Value::as_str)
            .and_then(|model| model_id_from_cli(agent_id, model))
            .or_else(|| {
                self.native_config
                    .discover_model(agent_id, workspace_path)
                    .ok()
                    .flatten()
                    .and_then(|raw| model_id_from_cli(agent_id, &raw))
            })
            .map(Ok)
            .unwrap_or_else(|| {
                provider_defaults(agent_id)
                    .map(|(_, model)| model.to_string())
                    .ok_or_else(|| unsupported_agent(agent_id))
            })?;
        let reasoning_value = if agent_id == "codex-cli" {
            selections
                .get("reasoningEffort")
                .and_then(Value::as_str)
                .map(|value| if value == "xhigh" { "max" } else { value })
        } else {
            selections.get("effort").and_then(Value::as_str)
        };
        let reasoning_depth = normalize_reasoning(reasoning_value).or_else(|| {
            if agent_id == "opencode" {
                None
            } else {
                Some("high".to_string())
            }
        });
        Ok(ChatConfigurationValues {
            execution_mode: "inherit".to_string(),
            provider_id: Some(
                provider_defaults(agent_id)
                    .map(|(provider, _)| provider.to_string())
                    .ok_or_else(|| unsupported_agent(agent_id))?,
            ),
            model_id: Some(model_id),
            reasoning_depth,
            streaming: true,
            thinking: selections
                .get("thinking")
                .and_then(Value::as_bool)
                .unwrap_or(true),
            long_context: agent_id != "opencode",
        })
    }
}

fn provider_defaults(agent_id: &str) -> Option<(&'static str, &'static str)> {
    match agent_id {
        "claude-code" => Some(("anthropic", "claude-opus-4-8")),
        "codex-cli" => Some(("openai", "gpt-5-5")),
        "gemini-cli" => Some(("google", "gemini-2-5-pro")),
        "opencode" => Some(("opencode", "opencode-default")),
        "antigravity-cli" => Some(("google", "antigravity-default")),
        _ => None,
    }
}

fn model_id_from_cli(agent_id: &str, model: &str) -> Option<String> {
    match (agent_id, model) {
        ("claude-code", "opus") => Some("claude-opus-4-8".to_string()),
        ("claude-code", "sonnet") => Some("claude-sonnet-5".to_string()),
        ("claude-code", "haiku") => Some("claude-haiku-4-5".to_string()),
        ("codex-cli", "gpt-5.5") => Some("gpt-5-5".to_string()),
        ("codex-cli", "gpt-5.4") => Some("gpt-5-4".to_string()),
        ("codex-cli", "gpt-5.2-codex") => Some("gpt-5-2-codex".to_string()),
        ("codex-cli", "gpt-5.1-codex-max") => Some("gpt-5-1-codex-max".to_string()),
        ("gemini-cli", "gemini-2.5-pro") => Some("gemini-2-5-pro".to_string()),
        ("gemini-cli", "gemini-2.5-flash") => Some("gemini-2-5-flash".to_string()),
        (_, "") | (_, "default") => None,
        _ => Some(model.to_string()),
    }
}

fn unsupported_agent(agent_id: &str) -> SessionsApplicationError {
    SessionsApplicationError::Validation(format!("Unsupported chat Agent: {agent_id}"))
}

impl SqliteSessionChatProfileAdapter {
    fn onepiece_defaults(&self) -> Result<ChatConfigurationValues, SessionsApplicationError> {
        let connection = self.database.connection().map_err(profile_error)?;
        let model_id = connection
            .query_row(
                "SELECT model_id FROM agents WHERE id = 'onepiece' AND launch_kind = 'api' AND agent_origin = 'builtin'",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(profile_error)?
            .flatten()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                SessionsApplicationError::Validation(
                    "OnePiece does not have an active configured model.".to_string(),
                )
            })?;
        Ok(ChatConfigurationValues {
            execution_mode: "inherit".to_string(),
            provider_id: Some("onepiece".to_string()),
            model_id: Some(model_id),
            reasoning_depth: None,
            streaming: true,
            thinking: true,
            long_context: true,
        })
    }
}

fn profile_error(error: impl std::fmt::Display) -> SessionsApplicationError {
    SessionsApplicationError::Repository(error.to_string())
}
