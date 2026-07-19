use crate::contexts::agent_runtime::application::AgentChatConfiguration;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderPromptDelivery {
    Stdin,
    Argument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderInvocationSpec {
    pub(crate) executable: String,
    pub(crate) args: Vec<String>,
    pub(crate) prompt_delivery: ProviderPromptDelivery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProviderInvocationError {
    UnsupportedAgent(String),
}

impl Display for ProviderInvocationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedAgent(agent_id) => write!(
                formatter,
                "{agent_id} is not supported by the CLI chat runtime."
            ),
        }
    }
}

impl std::error::Error for ProviderInvocationError {}

pub(crate) fn build_invocation(
    agent_id: &str,
    executable: String,
    prompt: &str,
    runtime_session_id: Option<&str>,
    managed_args: &[String],
) -> Result<ProviderInvocationSpec, ProviderInvocationError> {
    let mut args = Vec::new();
    let prompt_delivery = match agent_id {
        "claude-code" => {
            args.extend_from_slice(managed_args);
            args.extend([
                "-p".to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--include-partial-messages".to_string(),
                "--verbose".to_string(),
            ]);
            push_resume_args(&mut args, runtime_session_id, "--resume");
            ProviderPromptDelivery::Stdin
        }
        "codex-cli" => {
            args.extend(
                managed_args
                    .iter()
                    .filter(|argument| argument.as_str() != "--ephemeral")
                    .cloned(),
            );
            args.push("exec".to_string());
            if let Some(session_id) = non_empty_session_id(runtime_session_id) {
                args.extend(["resume".to_string(), session_id.to_string()]);
            }
            if managed_args
                .iter()
                .any(|argument| argument == "--ephemeral")
            {
                args.push("--ephemeral".to_string());
            }
            args.extend(["--json".to_string(), "--".to_string(), "-".to_string()]);
            ProviderPromptDelivery::Stdin
        }
        "gemini-cli" => {
            args.extend_from_slice(managed_args);
            push_resume_args(&mut args, runtime_session_id, "--resume");
            args.extend([
                "-p".to_string(),
                prompt.to_string(),
                "-o".to_string(),
                "stream-json".to_string(),
            ]);
            ProviderPromptDelivery::Argument
        }
        "opencode" => {
            args.push("run".to_string());
            args.extend_from_slice(managed_args);
            push_resume_args(&mut args, runtime_session_id, "--session");
            args.extend([
                "--format".to_string(),
                "json".to_string(),
                prompt.to_string(),
            ]);
            ProviderPromptDelivery::Argument
        }
        other => return Err(ProviderInvocationError::UnsupportedAgent(other.to_string())),
    };

    Ok(ProviderInvocationSpec {
        executable,
        args,
        prompt_delivery,
    })
}

pub(crate) fn apply_configuration_overrides(
    agent_id: &str,
    mut selections: BTreeMap<String, Value>,
    configuration: &AgentChatConfiguration,
) -> BTreeMap<String, Value> {
    if let Some(model) = configuration
        .model_id
        .as_deref()
        .and_then(|model_id| mapped_model(agent_id, model_id))
    {
        selections.insert("model".to_string(), Value::String(model.to_string()));
    }

    if let Some(reasoning_depth) = configuration.reasoning_depth.as_deref() {
        match agent_id {
            "claude-code" => {
                selections.insert(
                    "effort".to_string(),
                    Value::String(reasoning_depth.to_string()),
                );
            }
            "codex-cli" => {
                let effort = if reasoning_depth == "max" {
                    "xhigh"
                } else {
                    reasoning_depth
                };
                selections.insert(
                    "reasoningEffort".to_string(),
                    Value::String(effort.to_string()),
                );
            }
            _ => {}
        }
    }

    match (agent_id, configuration.permission_mode.as_str()) {
        ("claude-code", "plan") => {
            selections.insert(
                "permissionMode".to_string(),
                Value::String("plan".to_string()),
            );
        }
        ("claude-code", "agent" | "auto") => {
            selections.insert(
                "permissionMode".to_string(),
                Value::String("acceptEdits".to_string()),
            );
        }
        ("codex-cli", "plan") => {
            selections.insert(
                "sandbox".to_string(),
                Value::String("read-only".to_string()),
            );
            selections.insert(
                "approvalPolicy".to_string(),
                Value::String("on-request".to_string()),
            );
        }
        ("codex-cli", "agent" | "auto") => {
            selections.insert(
                "sandbox".to_string(),
                Value::String("workspace-write".to_string()),
            );
            selections.insert(
                "approvalPolicy".to_string(),
                Value::String("on-request".to_string()),
            );
        }
        ("gemini-cli", "plan") => {
            selections.insert(
                "approvalMode".to_string(),
                Value::String("plan".to_string()),
            );
        }
        ("gemini-cli", "agent" | "auto") => {
            selections.insert(
                "approvalMode".to_string(),
                Value::String("auto_edit".to_string()),
            );
        }
        ("opencode", "plan") => {
            selections.insert("agent".to_string(), Value::String("plan".to_string()));
        }
        ("opencode", "agent") => {
            selections.insert("agent".to_string(), Value::String("build".to_string()));
        }
        ("opencode", "auto") => {
            selections.insert("autoApprove".to_string(), Value::Bool(true));
        }
        _ => {}
    }

    if agent_id == "opencode" {
        selections.insert("thinking".to_string(), Value::Bool(configuration.thinking));
    }

    selections
}

fn mapped_model(agent_id: &str, model_id: &str) -> Option<&'static str> {
    match (agent_id, model_id) {
        ("claude-code", "claude-opus-4-8") => Some("opus"),
        ("claude-code", "claude-sonnet-5" | "claude-sonnet-4-6") => Some("sonnet"),
        ("claude-code", "claude-haiku-4-5") => Some("haiku"),
        ("codex-cli", "gpt-5-5") => Some("gpt-5.5"),
        ("codex-cli", "gpt-5-4") => Some("gpt-5.4"),
        ("codex-cli", "gpt-5-2-codex") => Some("gpt-5.2-codex"),
        ("codex-cli", "gpt-5-1-codex-max") => Some("gpt-5.1-codex-max"),
        ("gemini-cli", "gemini-2-5-pro") => Some("gemini-2.5-pro"),
        ("gemini-cli", "gemini-2-5-flash") => Some("gemini-2.5-flash"),
        _ => None,
    }
}

fn non_empty_session_id(runtime_session_id: Option<&str>) -> Option<&str> {
    runtime_session_id.filter(|value| !value.trim().is_empty())
}

fn push_resume_args(args: &mut Vec<String>, runtime_session_id: Option<&str>, flag: &str) {
    if let Some(session_id) = non_empty_session_id(runtime_session_id) {
        args.extend([flag.to_string(), session_id.to_string()]);
    }
}
