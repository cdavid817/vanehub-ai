use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum CliParametersError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Repository(String),
}

impl From<rusqlite::Error> for CliParametersError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Repository(error.to_string())
    }
}

pub(crate) const MANAGED_CLI_AGENT_IDS: [&str; 4] =
    ["claude-code", "codex-cli", "gemini-cli", "opencode"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum CliParameterControl {
    Enum,
    Boolean,
    MultiEnum,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CliParameterRisk {
    Normal,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum CliParameterLaunchScope {
    Interactive,
    Chat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliParameterOption {
    pub(crate) value: String,
    pub(crate) label_key: String,
    pub(crate) description_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliParameterDefinition {
    pub(crate) id: String,
    pub(crate) agent_id: String,
    pub(crate) flag: String,
    pub(crate) control: CliParameterControl,
    pub(crate) label_key: String,
    pub(crate) description_key: String,
    pub(crate) options: Vec<CliParameterOption>,
    pub(crate) default_value: Value,
    pub(crate) launch_scopes: Vec<CliParameterLaunchScope>,
    pub(crate) risk: CliParameterRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliParameterProfile {
    pub(crate) agent_id: String,
    pub(crate) definitions: Vec<CliParameterDefinition>,
    pub(crate) selections: BTreeMap<String, Value>,
    pub(crate) preview_args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveCliParameterProfileInput {
    agent_id: String,
    selections: BTreeMap<String, Value>,
}

fn option(prefix: &str, value: &str) -> CliParameterOption {
    let common_reasoning_value = matches!(value, "low" | "medium" | "high" | "xhigh" | "max");
    let label_key = if value == "default" {
        "cliParameters.values.default.label".to_string()
    } else if common_reasoning_value {
        format!("cliParameters.common.values.{value}.label")
    } else {
        format!("{prefix}.values.{value}.label")
    };
    CliParameterOption {
        value: value.to_string(),
        label_key,
        description_key: if common_reasoning_value {
            format!("cliParameters.common.values.{value}.description")
        } else {
            format!("{prefix}.values.{value}.description")
        },
    }
}

fn enum_definition(
    agent_id: &str,
    id: &str,
    flag: &str,
    values: &[&str],
    default_value: &str,
    risk: CliParameterRisk,
) -> CliParameterDefinition {
    let prefix = format!("cliParameters.{agent_id}.{id}");
    CliParameterDefinition {
        id: id.to_string(),
        agent_id: agent_id.to_string(),
        flag: flag.to_string(),
        control: CliParameterControl::Enum,
        label_key: format!("{prefix}.label"),
        description_key: format!("{prefix}.description"),
        options: values.iter().map(|value| option(&prefix, value)).collect(),
        default_value: Value::String(default_value.to_string()),
        launch_scopes: vec![
            CliParameterLaunchScope::Interactive,
            CliParameterLaunchScope::Chat,
        ],
        risk,
    }
}

fn boolean_definition(
    agent_id: &str,
    id: &str,
    flag: &str,
    scopes: Vec<CliParameterLaunchScope>,
    risk: CliParameterRisk,
) -> CliParameterDefinition {
    let prefix = format!("cliParameters.{agent_id}.{id}");
    CliParameterDefinition {
        id: id.to_string(),
        agent_id: agent_id.to_string(),
        flag: flag.to_string(),
        control: CliParameterControl::Boolean,
        label_key: format!("{prefix}.label"),
        description_key: format!("{prefix}.description"),
        options: Vec::new(),
        default_value: Value::Bool(false),
        launch_scopes: scopes,
        risk,
    }
}

pub(crate) fn catalog_for(
    agent_id: &str,
) -> Result<Vec<CliParameterDefinition>, CliParametersError> {
    let normal = CliParameterRisk::Normal;
    let warning = CliParameterRisk::Warning;
    let both = || {
        vec![
            CliParameterLaunchScope::Interactive,
            CliParameterLaunchScope::Chat,
        ]
    };
    let definitions = match agent_id {
        "claude-code" => vec![
            enum_definition(
                agent_id,
                "model",
                "--model",
                &["default", "sonnet", "opus", "haiku"],
                "default",
                normal.clone(),
            ),
            enum_definition(
                agent_id,
                "effort",
                "--effort",
                &["default", "low", "medium", "high", "xhigh", "max"],
                "default",
                normal.clone(),
            ),
            enum_definition(
                agent_id,
                "permissionMode",
                "--permission-mode",
                &["default", "plan", "acceptEdits", "auto", "dontAsk"],
                "default",
                normal.clone(),
            ),
            boolean_definition(
                agent_id,
                "chrome",
                "--chrome",
                vec![CliParameterLaunchScope::Interactive],
                normal,
            ),
        ],
        "codex-cli" => vec![
            enum_definition(
                agent_id,
                "model",
                "--model",
                &[
                    "default",
                    "gpt-5.5",
                    "gpt-5.4",
                    "gpt-5.2-codex",
                    "gpt-5.1-codex-max",
                ],
                "default",
                normal.clone(),
            ),
            enum_definition(
                agent_id,
                "reasoningEffort",
                "--config",
                &["default", "low", "medium", "high", "xhigh", "max"],
                "default",
                normal.clone(),
            ),
            enum_definition(
                agent_id,
                "sandbox",
                "--sandbox",
                &["default", "read-only", "workspace-write"],
                "default",
                normal.clone(),
            ),
            enum_definition(
                agent_id,
                "approvalPolicy",
                "--ask-for-approval",
                &["default", "untrusted", "on-request", "never"],
                "default",
                normal.clone(),
            ),
            boolean_definition(
                agent_id,
                "ephemeral",
                "--ephemeral",
                vec![CliParameterLaunchScope::Chat],
                normal.clone(),
            ),
            boolean_definition(agent_id, "strictConfig", "--strict-config", both(), normal),
        ],
        "gemini-cli" => vec![
            enum_definition(
                agent_id,
                "model",
                "--model",
                &["default", "gemini-2.5-pro", "gemini-2.5-flash"],
                "default",
                normal.clone(),
            ),
            enum_definition(
                agent_id,
                "approvalMode",
                "--approval-mode",
                &["default", "auto_edit", "plan", "yolo"],
                "yolo",
                warning,
            ),
            boolean_definition(agent_id, "sandbox", "--sandbox", both(), normal),
        ],
        "opencode" => vec![
            enum_definition(
                agent_id,
                "agent",
                "--agent",
                &["default", "build", "plan"],
                "default",
                normal.clone(),
            ),
            boolean_definition(
                agent_id,
                "thinking",
                "--thinking",
                vec![CliParameterLaunchScope::Chat],
                normal,
            ),
            boolean_definition(agent_id, "autoApprove", "--auto", both(), warning),
        ],
        _ => {
            return Err(CliParametersError::Validation(format!(
                "unsupported CLI agent id: {agent_id}"
            )))
        }
    };
    Ok(definitions)
}

fn default_selections(definitions: &[CliParameterDefinition]) -> BTreeMap<String, Value> {
    definitions
        .iter()
        .map(|definition| (definition.id.clone(), definition.default_value.clone()))
        .collect()
}

fn has_control_char(value: &str) -> bool {
    value.chars().any(char::is_control)
}

fn validate_value(definition: &CliParameterDefinition, value: &Value) -> bool {
    match definition.control {
        CliParameterControl::Boolean => value.is_boolean(),
        CliParameterControl::Enum => value.as_str().is_some_and(|candidate| {
            !has_control_char(candidate)
                && definition
                    .options
                    .iter()
                    .any(|option| option.value == candidate)
        }),
        CliParameterControl::MultiEnum => value.as_array().is_some_and(|values| {
            values.iter().all(|entry| {
                entry.as_str().is_some_and(|candidate| {
                    !has_control_char(candidate)
                        && definition
                            .options
                            .iter()
                            .any(|option| option.value == candidate)
                })
            })
        }),
    }
}

fn normalized_value(definition: &CliParameterDefinition, value: Value) -> Value {
    if definition.control != CliParameterControl::MultiEnum {
        return value;
    }
    let selected = value
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<BTreeSet<_>>();
    Value::Array(
        definition
            .options
            .iter()
            .filter(|option| selected.contains(option.value.as_str()))
            .map(|option| Value::String(option.value.clone()))
            .collect(),
    )
}

pub(crate) fn normalize_selections(
    agent_id: &str,
    input: &BTreeMap<String, Value>,
) -> Result<BTreeMap<String, Value>, CliParametersError> {
    let definitions = catalog_for(agent_id)?;
    let definition_ids = definitions
        .iter()
        .map(|definition| definition.id.as_str())
        .collect::<BTreeSet<_>>();
    if let Some(unknown) = input
        .keys()
        .find(|parameter_id| !definition_ids.contains(parameter_id.as_str()))
    {
        return Err(CliParametersError::Validation(format!(
            "unknown CLI parameter '{unknown}' for {agent_id}"
        )));
    }
    definitions
        .into_iter()
        .map(|definition| {
            let value = input
                .get(&definition.id)
                .cloned()
                .unwrap_or_else(|| definition.default_value.clone());
            if !validate_value(&definition, &value) {
                return Err(CliParametersError::Validation(format!(
                    "invalid value for CLI parameter '{}'",
                    definition.id
                )));
            }
            let value = normalized_value(&definition, value);
            Ok((definition.id, value))
        })
        .collect()
}

fn scope_matches(definition: &CliParameterDefinition, scope: &CliParameterLaunchScope) -> bool {
    definition.launch_scopes.contains(scope)
}

pub(crate) fn preview_args(
    agent_id: &str,
    selections: &BTreeMap<String, Value>,
    scope: CliParameterLaunchScope,
) -> Result<Vec<String>, CliParametersError> {
    let normalized = normalize_selections(agent_id, selections)?;
    let mut args = Vec::new();
    for definition in catalog_for(agent_id)? {
        if !scope_matches(&definition, &scope) {
            continue;
        }
        let Some(value) = normalized.get(&definition.id) else {
            continue;
        };
        match definition.control {
            CliParameterControl::Boolean => {
                if value.as_bool() == Some(true) {
                    args.push(definition.flag);
                }
            }
            CliParameterControl::Enum => {
                if let Some(value) = value.as_str().filter(|value| *value != "default") {
                    let rendered_value = if definition.id == "reasoningEffort" {
                        format!("model_reasoning_effort=\"{value}\"")
                    } else {
                        value.to_string()
                    };
                    args.extend([definition.flag, rendered_value]);
                }
            }
            CliParameterControl::MultiEnum => {
                if let Some(values) = value.as_array() {
                    for value in values.iter().filter_map(Value::as_str) {
                        args.extend([definition.flag.clone(), value.to_string()]);
                    }
                }
            }
        }
    }
    Ok(args)
}

pub(crate) fn apply_schema(conn: &Connection) -> Result<(), CliParametersError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS cli_parameter_settings (
            agent_id TEXT NOT NULL,
            parameter_id TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            value_json TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (agent_id, parameter_id),
            FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
        );
        "#,
    )?;
    Ok(())
}

fn write_profile_event(
    logging: Option<&dyn DiagnosticLogPort>,
    severity: LogSeverity,
    agent_id: &str,
    parameter_id: &str,
    message: &str,
) {
    let Some(logging) = logging else {
        return;
    };
    let mut context = BTreeMap::new();
    context.insert("agentId".to_string(), agent_id.to_string());
    context.insert("parameterId".to_string(), parameter_id.to_string());
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity,
        category: "cli.parameter".to_string(),
        message: message.to_string(),
        context,
    });
}

fn write_profile_warning(
    logging: Option<&dyn DiagnosticLogPort>,
    agent_id: &str,
    parameter_id: &str,
    message: &str,
) {
    write_profile_event(logging, LogSeverity::Warn, agent_id, parameter_id, message);
}

pub(crate) fn load_selections(
    conn: &Connection,
    agent_id: &str,
) -> Result<BTreeMap<String, Value>, CliParametersError> {
    load_selections_with_logging(conn, agent_id, None)
}

fn load_selections_with_logging(
    conn: &Connection,
    agent_id: &str,
    logging: Option<&dyn DiagnosticLogPort>,
) -> Result<BTreeMap<String, Value>, CliParametersError> {
    let definitions = catalog_for(agent_id)?;
    let mut selections = default_selections(&definitions);
    let definitions_by_id = definitions
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect::<BTreeMap<_, _>>();
    let mut statement = conn.prepare(
        "SELECT parameter_id, value_json FROM cli_parameter_settings WHERE agent_id = ?1 AND enabled = 1",
    )?;
    let rows = statement.query_map(params![agent_id], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (parameter_id, raw_value) = row?;
        let Some(definition) = definitions_by_id.get(parameter_id.as_str()) else {
            write_profile_warning(
                logging,
                agent_id,
                &parameter_id,
                "ignored unknown saved CLI parameter",
            );
            continue;
        };
        let Ok(value) = serde_json::from_str::<Value>(&raw_value) else {
            write_profile_warning(
                logging,
                agent_id,
                &parameter_id,
                "ignored malformed saved CLI parameter",
            );
            continue;
        };
        if validate_value(definition, &value) {
            selections.insert(parameter_id, value);
        } else {
            write_profile_warning(
                logging,
                agent_id,
                &parameter_id,
                "ignored invalid saved CLI parameter",
            );
        }
    }
    Ok(selections)
}

pub(crate) fn load_profile(
    conn: &Connection,
    agent_id: &str,
) -> Result<CliParameterProfile, CliParametersError> {
    let definitions = catalog_for(agent_id)?;
    let selections = load_selections(conn, agent_id)?;
    let preview_args = preview_args(agent_id, &selections, CliParameterLaunchScope::Chat)?;
    Ok(CliParameterProfile {
        agent_id: agent_id.to_string(),
        definitions,
        selections,
        preview_args,
    })
}

fn load_profile_with_logging(
    conn: &Connection,
    agent_id: &str,
    logging: Option<&dyn DiagnosticLogPort>,
) -> Result<CliParameterProfile, CliParametersError> {
    let definitions = catalog_for(agent_id)?;
    let selections = load_selections_with_logging(conn, agent_id, logging)?;
    let preview_args = preview_args(agent_id, &selections, CliParameterLaunchScope::Chat)?;
    Ok(CliParameterProfile {
        agent_id: agent_id.to_string(),
        definitions,
        selections,
        preview_args,
    })
}

fn save_profile_to_conn(
    conn: &mut Connection,
    input: &SaveCliParameterProfileInput,
) -> Result<CliParameterProfile, CliParametersError> {
    let selections = normalize_selections(&input.agent_id, &input.selections)?;
    let now = Utc::now().to_rfc3339();
    let transaction = conn.transaction()?;
    transaction.execute(
        "DELETE FROM cli_parameter_settings WHERE agent_id = ?1",
        params![input.agent_id],
    )?;
    for (parameter_id, value) in &selections {
        let value_json = serde_json::to_string(value)
            .map_err(|error| CliParametersError::Repository(error.to_string()))?;
        transaction.execute(
            "INSERT INTO cli_parameter_settings (agent_id, parameter_id, enabled, value_json, updated_at) VALUES (?1, ?2, 1, ?3, ?4)",
            params![input.agent_id, parameter_id, value_json, now],
        )?;
    }
    transaction.commit()?;
    load_profile(conn, &input.agent_id)
}

fn reset_profile_in_conn(
    conn: &Connection,
    agent_id: &str,
) -> Result<CliParameterProfile, CliParametersError> {
    catalog_for(agent_id)?;
    conn.execute(
        "DELETE FROM cli_parameter_settings WHERE agent_id = ?1",
        params![agent_id],
    )?;
    load_profile(conn, agent_id)
}

#[derive(Clone)]
pub(crate) struct CliParametersApi {
    database: NativeDatabase,
    logging: Arc<dyn DiagnosticLogPort>,
}

impl CliParametersApi {
    pub(crate) fn new(database: NativeDatabase, logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self { database, logging }
    }

    pub(crate) fn list_profiles(&self) -> Result<Vec<CliParameterProfile>, CliParametersError> {
        let connection = self.connection()?;
        MANAGED_CLI_AGENT_IDS
            .iter()
            .map(|agent_id| {
                load_profile_with_logging(&connection, agent_id, Some(self.logging.as_ref()))
            })
            .collect()
    }

    pub(crate) fn save_profile(
        &self,
        input: &SaveCliParameterProfileInput,
    ) -> Result<CliParameterProfile, CliParametersError> {
        let mut connection = self.connection()?;
        let result = save_profile_to_conn(&mut connection, input);
        let (severity, message) = match &result {
            Ok(_) => (LogSeverity::Info, "saved CLI parameter profile".to_string()),
            Err(error) => (
                LogSeverity::Error,
                format!("failed to save CLI parameter profile: {error}"),
            ),
        };
        write_profile_event(
            Some(self.logging.as_ref()),
            severity,
            &input.agent_id,
            "profile",
            &message,
        );
        result
    }

    pub(crate) fn reset_profile(
        &self,
        agent_id: &str,
    ) -> Result<CliParameterProfile, CliParametersError> {
        let connection = self.connection()?;
        let result = reset_profile_in_conn(&connection, agent_id);
        let (severity, message) = match &result {
            Ok(_) => (LogSeverity::Info, "reset CLI parameter profile".to_string()),
            Err(error) => (
                LogSeverity::Error,
                format!("failed to reset CLI parameter profile: {error}"),
            ),
        };
        write_profile_event(
            Some(self.logging.as_ref()),
            severity,
            agent_id,
            "profile",
            &message,
        );
        result
    }

    pub(crate) fn load_selections(
        &self,
        agent_id: &str,
    ) -> Result<BTreeMap<String, Value>, CliParametersError> {
        load_selections_with_logging(&*self.connection()?, agent_id, Some(self.logging.as_ref()))
    }

    pub(crate) fn normalize_selections(
        &self,
        agent_id: &str,
        selections: &BTreeMap<String, Value>,
    ) -> Result<BTreeMap<String, Value>, CliParametersError> {
        normalize_selections(agent_id, selections)
    }

    pub(crate) fn preview_args(
        &self,
        agent_id: &str,
        selections: &BTreeMap<String, Value>,
        scope: CliParameterLaunchScope,
    ) -> Result<Vec<String>, CliParametersError> {
        preview_args(agent_id, selections, scope)
    }

    fn connection(&self) -> Result<PooledSqlite, CliParametersError> {
        self.database
            .connection()
            .map_err(|error| CliParametersError::Repository(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::OptionalExtension;

    fn connection() -> Connection {
        let conn = Connection::open_in_memory().expect("database");
        conn.execute_batch(
            "PRAGMA foreign_keys = ON; CREATE TABLE agents (id TEXT PRIMARY KEY); INSERT INTO agents VALUES ('claude-code'), ('codex-cli'), ('gemini-cli'), ('opencode');",
        )
        .expect("agents");
        apply_schema(&conn).expect("schema");
        conn
    }

    #[test]
    fn schema_and_profiles_round_trip_per_agent() {
        let mut conn = connection();
        let table: Option<String> = conn
            .query_row(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'cli_parameter_settings'",
                [],
                |row| row.get(0),
            )
            .optional()
            .expect("query");
        assert_eq!(table.as_deref(), Some("cli_parameter_settings"));

        let input = SaveCliParameterProfileInput {
            agent_id: "codex-cli".to_string(),
            selections: BTreeMap::from([
                (
                    "sandbox".to_string(),
                    Value::String("read-only".to_string()),
                ),
                ("strictConfig".to_string(), Value::Bool(true)),
            ]),
        };
        let saved = save_profile_to_conn(&mut conn, &input).expect("save");
        assert_eq!(saved.selections["sandbox"], "read-only");
        assert!(saved.preview_args.contains(&"--strict-config".to_string()));
        assert_eq!(
            load_profile(&conn, "claude-code")
                .expect("other")
                .selections["model"],
            "default"
        );

        let reset = reset_profile_in_conn(&conn, "codex-cli").expect("reset");
        assert_eq!(reset.selections["sandbox"], "default");
    }

    #[test]
    fn additive_schema_preserves_existing_tables() {
        let conn = Connection::open_in_memory().expect("database");
        conn.execute_batch(
            "CREATE TABLE agents (id TEXT PRIMARY KEY); CREATE TABLE legacy_data (value TEXT); INSERT INTO legacy_data VALUES ('kept');",
        )
        .expect("legacy schema");
        apply_schema(&conn).expect("schema");
        let value: String = conn
            .query_row("SELECT value FROM legacy_data", [], |row| row.get(0))
            .expect("legacy row");
        assert_eq!(value, "kept");
    }

    #[test]
    fn invalid_save_is_atomic() {
        let mut conn = connection();
        let valid = SaveCliParameterProfileInput {
            agent_id: "codex-cli".to_string(),
            selections: BTreeMap::from([(
                "sandbox".to_string(),
                Value::String("read-only".to_string()),
            )]),
        };
        save_profile_to_conn(&mut conn, &valid).expect("initial save");
        let invalid = SaveCliParameterProfileInput {
            agent_id: "codex-cli".to_string(),
            selections: BTreeMap::from([(
                "sandbox".to_string(),
                Value::String("danger-full-access".to_string()),
            )]),
        };
        assert!(save_profile_to_conn(&mut conn, &invalid).is_err());
        assert_eq!(
            load_profile(&conn, "codex-cli").expect("load").selections["sandbox"],
            "read-only"
        );
        assert!(normalize_selections("unknown-agent", &BTreeMap::new()).is_err());
        assert!(normalize_selections(
            "codex-cli",
            &BTreeMap::from([("sandbox".to_string(), Value::Bool(true))]),
        )
        .is_err());
        assert!(normalize_selections(
            "codex-cli",
            &BTreeMap::from([("unknown".to_string(), Value::String("value".to_string()))]),
        )
        .is_err());
        assert!(normalize_selections(
            "codex-cli",
            &BTreeMap::from([(
                "sandbox".to_string(),
                Value::String("read-only\n--json".to_string()),
            )]),
        )
        .is_err());
    }

    #[test]
    fn launch_scopes_and_codex_config_render_as_distinct_safe_tokens() {
        let selections = BTreeMap::from([
            ("ephemeral".to_string(), Value::Bool(true)),
            ("strictConfig".to_string(), Value::Bool(true)),
            (
                "reasoningEffort".to_string(),
                Value::String("high".to_string()),
            ),
        ]);
        let chat = preview_args("codex-cli", &selections, CliParameterLaunchScope::Chat)
            .expect("chat args");
        assert!(chat.contains(&"--ephemeral".to_string()));
        assert!(chat
            .windows(2)
            .any(|pair| { pair == ["--config", "model_reasoning_effort=\"high\""] }));
        let interactive = preview_args(
            "codex-cli",
            &selections,
            CliParameterLaunchScope::Interactive,
        )
        .expect("interactive args");
        assert!(!interactive.contains(&"--ephemeral".to_string()));
        assert!(interactive.contains(&"--strict-config".to_string()));
        assert!(chat.iter().all(|value| !value.contains("prompt")));
    }

    #[test]
    fn interactive_profile_is_reloaded_for_each_launch_snapshot() {
        let conn = connection();
        let before = preview_args(
            "claude-code",
            &load_selections(&conn, "claude-code").expect("before"),
            CliParameterLaunchScope::Interactive,
        )
        .expect("before args");
        assert!(!before.contains(&"--chrome".to_string()));
        conn.execute(
            "INSERT INTO cli_parameter_settings (agent_id, parameter_id, enabled, value_json, updated_at) VALUES ('claude-code', 'chrome', 1, 'true', ?1)",
            params![Utc::now().to_rfc3339()],
        )
        .expect("save");
        let after = preview_args(
            "claude-code",
            &load_selections(&conn, "claude-code").expect("after"),
            CliParameterLaunchScope::Interactive,
        )
        .expect("after args");
        assert!(after.contains(&"--chrome".to_string()));
    }

    #[test]
    fn multi_enum_values_normalize_to_catalog_order() {
        let definition = CliParameterDefinition {
            id: "feature".to_string(),
            agent_id: "codex-cli".to_string(),
            flag: "--feature".to_string(),
            control: CliParameterControl::MultiEnum,
            label_key: "feature.label".to_string(),
            description_key: "feature.description".to_string(),
            options: vec![option("feature", "alpha"), option("feature", "beta")],
            default_value: Value::Array(Vec::new()),
            launch_scopes: vec![CliParameterLaunchScope::Chat],
            risk: CliParameterRisk::Normal,
        };
        let normalized =
            normalized_value(&definition, serde_json::json!(["beta", "alpha", "beta"]));
        assert_eq!(normalized, serde_json::json!(["alpha", "beta"]));
    }

    #[test]
    fn catalog_excludes_reserved_and_dangerous_flags() {
        let reserved = [
            "--output-format",
            "--resume",
            "--session",
            "--json",
            "--format",
            "--prompt",
        ];
        for agent_id in MANAGED_CLI_AGENT_IDS {
            let definitions = catalog_for(agent_id).expect("catalog");
            let expected_ids: &[&str] = match agent_id {
                "claude-code" => &["model", "effort", "permissionMode", "chrome"],
                "codex-cli" => &[
                    "model",
                    "reasoningEffort",
                    "sandbox",
                    "approvalPolicy",
                    "ephemeral",
                    "strictConfig",
                ],
                "gemini-cli" => &["model", "approvalMode", "sandbox"],
                "opencode" => &["agent", "thinking", "autoApprove"],
                _ => unreachable!(),
            };
            assert_eq!(
                definitions
                    .iter()
                    .map(|definition| definition.id.as_str())
                    .collect::<Vec<_>>(),
                expected_ids
            );
            assert!(definitions
                .iter()
                .any(|entry| entry.control == CliParameterControl::Enum));
            assert!(definitions
                .iter()
                .any(|entry| entry.control == CliParameterControl::Boolean));
            assert!(definitions
                .iter()
                .all(|entry| !reserved.contains(&entry.flag.as_str())));
            assert!(definitions
                .iter()
                .all(|entry| !entry.flag.contains("dangerously")));
        }
    }

    #[test]
    fn diagnostics_redact_sensitive_tokens() {
        let redacted =
            crate::platform::logging::redact_text("parameter api_key=secret token=value");
        assert!(!redacted.contains("secret"));
        assert!(!redacted.contains("value"));
    }
}
