use crate::contexts::sessions::application::{
    ChatConfigurationValues, SessionChatProfilePort, SessionsApplicationError,
};
use crate::contexts::sessions::domain::{
    default_model_for_agent, model_id_from_cli, normalize_reasoning, provider_for_agent,
};
use crate::contexts::tooling::cli::application::NativeConfigPort;
use crate::contexts::tooling::cli_parameters::CliParametersApi;
use crate::platform::database::NativeDatabase;
use rusqlite::OptionalExtension;
use serde_json::Value;
use std::collections::BTreeMap;
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
                default_model_for_agent(agent_id)
                    .map(str::to_string)
                    .map_err(SessionsApplicationError::from)
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
            permission_mode: permission_from_cli(agent_id, &selections),
            provider_id: Some(
                provider_for_agent(agent_id)
                    .map_err(SessionsApplicationError::from)?
                    .to_string(),
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
            permission_mode: "default".to_string(),
            provider_id: Some("onepiece".to_string()),
            model_id: Some(model_id),
            reasoning_depth: None,
            streaming: true,
            thinking: true,
            long_context: true,
        })
    }
}

fn permission_from_cli(agent_id: &str, selections: &BTreeMap<String, Value>) -> String {
    match agent_id {
        "claude-code" => match selections.get("permissionMode").and_then(Value::as_str) {
            Some("plan") => "plan",
            Some("acceptEdits" | "dontAsk") => "agent",
            Some("auto") => "auto",
            _ => "default",
        },
        "codex-cli" => match (
            selections.get("sandbox").and_then(Value::as_str),
            selections.get("approvalPolicy").and_then(Value::as_str),
        ) {
            (Some("read-only"), _) => "plan",
            (Some("workspace-write"), _) => "agent",
            (_, Some("never")) => "auto",
            _ => "default",
        },
        "gemini-cli" => match selections.get("approvalMode").and_then(Value::as_str) {
            Some("plan") => "plan",
            Some("auto_edit") => "agent",
            _ => "default",
        },
        "opencode" if selections.get("autoApprove").and_then(Value::as_bool) == Some(true) => {
            "auto"
        }
        _ => "default",
    }
    .to_string()
}

fn profile_error(error: impl std::fmt::Display) -> SessionsApplicationError {
    SessionsApplicationError::Repository(error.to_string())
}
