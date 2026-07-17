use crate::{cli_parameters, current_timestamp, load_session, AppError, ChatConfig, Session};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct SessionChatPreferences {
    permission_mode: String,
    provider_id: String,
    model_id: String,
    reasoning_depth: Option<String>,
    streaming: bool,
    thinking: bool,
    long_context: bool,
}

pub(crate) fn apply_schema(conn: &Connection) -> Result<(), AppError> {
    if !crate::table_has_column(conn, "sessions", "chat_preferences")? {
        conn.execute("ALTER TABLE sessions ADD COLUMN chat_preferences TEXT", [])?;
    }
    Ok(())
}

fn provider_for_agent(agent_id: &str) -> Result<&'static str, AppError> {
    match agent_id {
        "claude-code" => Ok("anthropic"),
        "codex-cli" => Ok("openai"),
        "gemini-cli" => Ok("google"),
        "opencode" => Ok("opencode"),
        _ => Err(AppError::Validation(format!(
            "Unsupported chat agent: {agent_id}."
        ))),
    }
}

fn default_model_for_agent(agent_id: &str) -> Result<&'static str, AppError> {
    match agent_id {
        "claude-code" => Ok("claude-opus-4-8"),
        "codex-cli" => Ok("gpt-5-5"),
        "gemini-cli" => Ok("gemini-2-5-pro"),
        "opencode" => Ok("opencode-default"),
        _ => Err(AppError::Validation(format!(
            "Unsupported chat agent: {agent_id}."
        ))),
    }
}

fn model_id_from_cli(agent_id: &str, model: &str) -> Option<&'static str> {
    match (agent_id, model) {
        ("claude-code", "opus") => Some("claude-opus-4-8"),
        ("claude-code", "sonnet") => Some("claude-sonnet-5"),
        ("claude-code", "haiku") => Some("claude-haiku-4-5"),
        ("codex-cli", "gpt-5.5") => Some("gpt-5-5"),
        ("codex-cli", "gpt-5.4") => Some("gpt-5-4"),
        ("codex-cli", "gpt-5.2-codex") => Some("gpt-5-2-codex"),
        ("codex-cli", "gpt-5.1-codex-max") => Some("gpt-5-1-codex-max"),
        ("gemini-cli", "gemini-2.5-pro") => Some("gemini-2-5-pro"),
        ("gemini-cli", "gemini-2.5-flash") => Some("gemini-2-5-flash"),
        _ => None,
    }
}

fn supported_model(agent_id: &str, model_id: &str) -> bool {
    matches!(
        (agent_id, model_id),
        (
            "claude-code",
            "claude-opus-4-8" | "claude-sonnet-5" | "claude-sonnet-4-6" | "claude-haiku-4-5"
        ) | (
            "codex-cli",
            "gpt-5-5" | "gpt-5-4" | "gpt-5-2-codex" | "gpt-5-1-codex-max"
        ) | ("gemini-cli", "gemini-2-5-pro" | "gemini-2-5-flash")
            | ("opencode", "opencode-default")
    )
}

fn normalize_reasoning(value: Option<&str>) -> Option<String> {
    value
        .filter(|depth| matches!(*depth, "low" | "medium" | "high" | "max"))
        .map(str::to_string)
}

fn reasoning_rank(value: &str) -> usize {
    match value {
        "low" => 0,
        "medium" => 1,
        "high" => 2,
        "max" => 3,
        _ => 0,
    }
}

fn max_reasoning_for_model(model_id: &str) -> Option<&'static str> {
    match model_id {
        "claude-opus-4-8" | "claude-sonnet-5" | "gpt-5-5" | "gpt-5-1-codex-max" => Some("max"),
        "claude-sonnet-4-6" | "gpt-5-4" | "gpt-5-2-codex" | "gemini-2-5-pro" => Some("high"),
        "gemini-2-5-flash" => Some("medium"),
        _ => None,
    }
}

fn clamp_reasoning_for_model(model_id: &str, value: Option<&str>) -> Option<String> {
    let normalized = normalize_reasoning(value)?;
    let maximum = max_reasoning_for_model(model_id)?;
    if reasoning_rank(&normalized) > reasoning_rank(maximum) {
        Some(maximum.to_string())
    } else {
        Some(normalized)
    }
}

fn permission_from_cli(
    agent_id: &str,
    selections: &std::collections::BTreeMap<String, serde_json::Value>,
) -> String {
    match agent_id {
        "claude-code" => match selections
            .get("permissionMode")
            .and_then(serde_json::Value::as_str)
        {
            Some("plan") => "plan",
            Some("acceptEdits" | "dontAsk") => "agent",
            Some("auto") => "auto",
            _ => "default",
        },
        "codex-cli" => match (
            selections
                .get("sandbox")
                .and_then(serde_json::Value::as_str),
            selections
                .get("approvalPolicy")
                .and_then(serde_json::Value::as_str),
        ) {
            (Some("read-only"), _) => "plan",
            (Some("workspace-write"), _) => "agent",
            (_, Some("never")) => "auto",
            _ => "default",
        },
        "gemini-cli" => match selections
            .get("approvalMode")
            .and_then(serde_json::Value::as_str)
        {
            Some("plan") => "plan",
            Some("auto_edit") => "agent",
            _ => "default",
        },
        "opencode"
            if selections
                .get("autoApprove")
                .and_then(serde_json::Value::as_bool)
                == Some(true) =>
        {
            "auto"
        }
        _ => "default",
    }
    .to_string()
}

fn defaults_from_profile(
    conn: &Connection,
    session: &Session,
) -> Result<SessionChatPreferences, AppError> {
    let selections = cli_parameters::load_selections(conn, &session.agent_id)?;
    let model_id = selections
        .get("model")
        .and_then(serde_json::Value::as_str)
        .and_then(|model| model_id_from_cli(&session.agent_id, model))
        .unwrap_or(default_model_for_agent(&session.agent_id)?)
        .to_string();
    let reasoning_value = if session.agent_id == "codex-cli" {
        selections
            .get("reasoningEffort")
            .and_then(serde_json::Value::as_str)
            .map(|value| if value == "xhigh" { "max" } else { value })
    } else {
        selections.get("effort").and_then(serde_json::Value::as_str)
    };
    let reasoning_candidate = normalize_reasoning(reasoning_value).or_else(|| {
        if session.agent_id == "opencode" {
            None
        } else {
            Some("high".to_string())
        }
    });
    let reasoning_depth = clamp_reasoning_for_model(&model_id, reasoning_candidate.as_deref());
    let thinking = selections
        .get("thinking")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    Ok(SessionChatPreferences {
        permission_mode: permission_from_cli(&session.agent_id, &selections),
        provider_id: provider_for_agent(&session.agent_id)?.to_string(),
        model_id,
        reasoning_depth,
        streaming: true,
        thinking,
        long_context: session.agent_id != "opencode",
    })
}

fn normalize_preferences(
    session: &Session,
    config: &ChatConfig,
) -> Result<SessionChatPreferences, AppError> {
    if !matches!(
        config.permission_mode.as_str(),
        "default" | "plan" | "agent" | "auto"
    ) {
        return Err(AppError::Validation(
            "Unsupported permission mode.".to_string(),
        ));
    }
    let expected_provider = provider_for_agent(&session.agent_id)?;
    let provider_id = config.provider_id.as_deref().unwrap_or(expected_provider);
    if provider_id != expected_provider {
        return Err(AppError::Validation(format!(
            "Provider '{provider_id}' does not match session agent '{}'.",
            session.agent_id
        )));
    }
    let model_id = config
        .model_id
        .as_deref()
        .unwrap_or(default_model_for_agent(&session.agent_id)?);
    if !supported_model(&session.agent_id, model_id) {
        return Err(AppError::Validation(format!(
            "Model '{model_id}' is unsupported for session agent '{}'.",
            session.agent_id
        )));
    }
    let reasoning_depth = match config.reasoning_depth.as_deref() {
        Some(value) if normalize_reasoning(Some(value)).is_none() => {
            return Err(AppError::Validation(
                "Unsupported reasoning depth.".to_string(),
            ));
        }
        value => clamp_reasoning_for_model(model_id, value),
    };
    Ok(SessionChatPreferences {
        permission_mode: config.permission_mode.clone(),
        provider_id: expected_provider.to_string(),
        model_id: model_id.to_string(),
        reasoning_depth,
        streaming: config.streaming,
        thinking: config.thinking,
        long_context: config.long_context,
    })
}

fn compose(session: &Session, preferences: SessionChatPreferences) -> ChatConfig {
    ChatConfig {
        agent_id: session.agent_id.clone(),
        interaction_mode: session.interaction_mode.clone(),
        permission_mode: preferences.permission_mode,
        provider_id: Some(preferences.provider_id),
        model_id: Some(preferences.model_id),
        reasoning_depth: preferences.reasoning_depth,
        streaming: preferences.streaming,
        thinking: preferences.thinking,
        long_context: preferences.long_context,
    }
}

pub(crate) fn validate_for_session(
    conn: &Connection,
    session_id: &str,
    config: &ChatConfig,
) -> Result<ChatConfig, AppError> {
    let session = load_session(conn, session_id)?;
    let preferences = normalize_preferences(&session, config)?;
    Ok(compose(&session, preferences))
}

pub(crate) fn load_from_conn(conn: &Connection, session_id: &str) -> Result<ChatConfig, AppError> {
    let session = load_session(conn, session_id)?;
    let raw = conn
        .query_row(
            "SELECT chat_preferences FROM sessions WHERE id = ?1",
            params![session_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();
    let preferences = raw
        .as_deref()
        .and_then(|value| serde_json::from_str::<SessionChatPreferences>(value).ok())
        .filter(|value| {
            value.provider_id == provider_for_agent(&session.agent_id).unwrap_or_default()
                && supported_model(&session.agent_id, &value.model_id)
                && matches!(
                    value.permission_mode.as_str(),
                    "default" | "plan" | "agent" | "auto"
                )
                && value
                    .reasoning_depth
                    .as_deref()
                    .is_none_or(|depth| normalize_reasoning(Some(depth)).is_some())
        })
        .map(Ok)
        .unwrap_or_else(|| defaults_from_profile(conn, &session))?;
    Ok(compose(&session, preferences))
}

pub(crate) fn save_to_conn(
    conn: &Connection,
    session_id: &str,
    config: &ChatConfig,
) -> Result<ChatConfig, AppError> {
    let session = load_session(conn, session_id)?;
    let preferences = normalize_preferences(&session, config)?;
    let raw = serde_json::to_string(&preferences)
        .map_err(|error| AppError::Storage(error.to_string()))?;
    conn.execute(
        "UPDATE sessions SET chat_preferences = ?1, updated_at = ?2 WHERE id = ?3",
        params![raw, current_timestamp(), session_id],
    )?;
    Ok(compose(&session, preferences))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{insert_test_session, test_conn};
    use crate::InteractionMode;

    #[test]
    fn existing_session_derives_valid_defaults() {
        let conn = test_conn();
        insert_test_session(&conn, "session-config-default");

        let config = load_from_conn(&conn, "session-config-default").expect("default config");

        assert_eq!(config.agent_id, "gemini-cli");
        assert_eq!(config.interaction_mode, InteractionMode::Browser);
        assert_eq!(config.provider_id.as_deref(), Some("google"));
        assert_eq!(config.model_id.as_deref(), Some("gemini-2-5-pro"));
    }

    #[test]
    fn configurations_are_isolated_and_deleted_with_sessions() {
        let conn = test_conn();
        insert_test_session(&conn, "session-config-a");
        insert_test_session(&conn, "session-config-b");
        let mut first = load_from_conn(&conn, "session-config-a").expect("first defaults");
        first.model_id = Some("gemini-2-5-flash".to_string());
        save_to_conn(&conn, "session-config-a", &first).expect("save first");

        assert_eq!(
            load_from_conn(&conn, "session-config-a")
                .expect("load first")
                .model_id
                .as_deref(),
            Some("gemini-2-5-flash")
        );
        assert_eq!(
            load_from_conn(&conn, "session-config-b")
                .expect("load second")
                .model_id
                .as_deref(),
            Some("gemini-2-5-pro")
        );

        conn.execute(
            "DELETE FROM sessions WHERE id = ?1",
            params!["session-config-a"],
        )
        .expect("delete session");
        assert!(load_from_conn(&conn, "session-config-a").is_err());
    }

    #[test]
    fn invalid_snapshot_and_save_values_are_safe() {
        let conn = test_conn();
        insert_test_session(&conn, "session-config-invalid");
        conn.execute(
            "UPDATE sessions SET chat_preferences = ?1 WHERE id = ?2",
            params!["{not-json", "session-config-invalid"],
        )
        .expect("seed invalid snapshot");
        let defaults = load_from_conn(&conn, "session-config-invalid").expect("fallback defaults");
        assert_eq!(defaults.model_id.as_deref(), Some("gemini-2-5-pro"));

        let mut invalid = defaults;
        invalid.model_id = Some("unknown-model".to_string());
        assert!(save_to_conn(&conn, "session-config-invalid", &invalid).is_err());
    }

    #[test]
    fn send_validation_composes_authoritative_identity_before_launch() {
        let conn = test_conn();
        insert_test_session(&conn, "session-config-send");
        let mut requested = load_from_conn(&conn, "session-config-send").expect("defaults");
        requested.agent_id = "claude-code".to_string();
        requested.interaction_mode = InteractionMode::Cli;

        let validated =
            validate_for_session(&conn, "session-config-send", &requested).expect("validated");

        assert_eq!(validated.agent_id, "gemini-cli");
        assert_eq!(validated.interaction_mode, InteractionMode::Browser);
        assert_eq!(validated.provider_id.as_deref(), Some("google"));
    }

    #[test]
    fn send_validation_rejects_provider_model_and_permission_mismatches() {
        let conn = test_conn();
        insert_test_session(&conn, "session-config-reject");
        let defaults = load_from_conn(&conn, "session-config-reject").expect("defaults");

        let mut invalid_provider = defaults.clone();
        invalid_provider.provider_id = Some("openai".to_string());
        assert!(validate_for_session(&conn, "session-config-reject", &invalid_provider).is_err());

        let mut invalid_model = defaults.clone();
        invalid_model.model_id = Some("gpt-5-5".to_string());
        assert!(validate_for_session(&conn, "session-config-reject", &invalid_model).is_err());

        let mut invalid_permission = defaults;
        invalid_permission.permission_mode = "unrestricted".to_string();
        assert!(validate_for_session(&conn, "session-config-reject", &invalid_permission).is_err());
    }

    #[test]
    fn send_validation_clamps_reasoning_to_the_selected_model() {
        let conn = test_conn();
        insert_test_session(&conn, "session-config-reasoning");
        let mut requested = load_from_conn(&conn, "session-config-reasoning").expect("defaults");
        requested.reasoning_depth = Some("max".to_string());

        let validated =
            validate_for_session(&conn, "session-config-reasoning", &requested).expect("validated");

        assert_eq!(validated.reasoning_depth.as_deref(), Some("high"));
    }
}
