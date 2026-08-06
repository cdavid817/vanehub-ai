use crate::contexts::agent_runtime::application::AgentChatConfiguration;
use crate::contexts::permissions::api::PolicyTemplateName;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

/// The three managed CLI agents whose interactive launches `apply_policy_template_overrides`
/// governs — `claude-code` is deliberately excluded, since its policy template is already
/// enforced dynamically through `claude-code-permission-hook`'s per-call hook, not a launch flag
/// (`add-cli-agent-permission-launch-flags` design.md).
pub(crate) const POLICY_TEMPLATE_GOVERNED_AGENT_IDS: [&str; 3] =
    ["codex-cli", "gemini-cli", "opencode"];

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
pub(crate) struct ProviderInteractiveInvocationSpec {
    pub(crate) executable: String,
    pub(crate) args: Vec<String>,
    pub(crate) assigned_runtime_session_id: Option<String>,
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
            args.extend(["--json".to_string(), "-".to_string()]);
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

pub(crate) fn build_interactive_invocation(
    agent_id: &str,
    executable: String,
    runtime_session_id: Option<&str>,
    managed_args: &[String],
) -> Result<ProviderInteractiveInvocationSpec, ProviderInvocationError> {
    let mut args = Vec::new();
    let existing_session_id = non_empty_session_id(runtime_session_id);
    let mut assigned_runtime_session_id = None;
    match agent_id {
        "claude-code" => {
            args.extend_from_slice(managed_args);
            if let Some(session_id) = existing_session_id {
                push_session_arg(&mut args, "--resume", session_id);
            } else {
                let session_id = uuid::Uuid::new_v4().to_string();
                push_session_arg(&mut args, "--session-id", &session_id);
                assigned_runtime_session_id = Some(session_id);
            }
        }
        "codex-cli" => {
            args.extend_from_slice(managed_args);
            if let Some(session_id) = existing_session_id {
                args.extend(["resume".to_string(), session_id.to_string()]);
            }
        }
        "gemini-cli" => {
            args.extend_from_slice(managed_args);
            if let Some(session_id) = existing_session_id {
                push_session_arg(&mut args, "--resume", session_id);
            } else {
                let session_id = uuid::Uuid::new_v4().to_string();
                push_session_arg(&mut args, "--session-id", &session_id);
                assigned_runtime_session_id = Some(session_id);
            }
        }
        "opencode" => {
            args.extend_from_slice(managed_args);
            if let Some(session_id) = existing_session_id {
                push_session_arg(&mut args, "--session", session_id);
            }
        }
        other => return Err(ProviderInvocationError::UnsupportedAgent(other.to_string())),
    };

    Ok(ProviderInteractiveInvocationSpec {
        executable,
        args,
        assigned_runtime_session_id,
    })
}

pub(crate) fn add_codex_output_capture_args(args: &mut Vec<String>, output_path: &str) {
    let insert_at = args
        .iter()
        .position(|argument| argument == "-")
        .unwrap_or(args.len());
    args.splice(
        insert_at..insert_at,
        ["-o".to_string(), output_path.to_string()],
    );
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

/// Projects an agent principal's assigned policy template onto `codex-cli`/`gemini-cli`/
/// `opencode`'s own native launch-time approval/sandbox parameters, taking precedence over
/// whatever the user separately saved in that agent's CLI Parameter profile for the specific
/// keys it governs (`add-cli-agent-permission-launch-flags` design.md's mapping table). Every
/// other selection is left untouched — this mirrors `apply_configuration_overrides`'s own
/// override-only-what-you-govern shape, just driven by the launch-time policy template instead
/// of the per-message chat `permission_mode`.
///
/// `trusted` and `yolo` deliberately resolve identically here, matching the established
/// `permissions-core` precedent that the two templates already resolve identically in
/// `evaluate()` — the difference between them is assignment-time confirmation friction, not
/// technical capability, so it would be inconsistent to carve out a tool-specific middle tier
/// (e.g. gemini-cli's `auto_edit`) for one of them here.
///
/// `opencode`'s `standard` deliberately makes no selections change: no existing `opencode`
/// catalog value means "ask before edits/bash, stay permissive for reads," so that template is
/// instead expressed via an injected `OPENCODE_PERMISSION` environment variable in the generated
/// terminal wrapper script (see `terminal_wrapper.rs`).
pub(crate) fn apply_policy_template_overrides(
    agent_id: &str,
    mut selections: BTreeMap<String, Value>,
    template: PolicyTemplateName,
) -> BTreeMap<String, Value> {
    match (agent_id, template) {
        ("codex-cli", PolicyTemplateName::Readonly) => {
            selections.insert(
                "sandbox".to_string(),
                Value::String("read-only".to_string()),
            );
            selections.insert(
                "approvalPolicy".to_string(),
                Value::String("never".to_string()),
            );
        }
        ("codex-cli", PolicyTemplateName::Standard) => {
            selections.insert(
                "sandbox".to_string(),
                Value::String("workspace-write".to_string()),
            );
            selections.insert(
                "approvalPolicy".to_string(),
                Value::String("on-request".to_string()),
            );
        }
        ("codex-cli", PolicyTemplateName::Trusted | PolicyTemplateName::Yolo) => {
            selections.insert(
                "sandbox".to_string(),
                Value::String("workspace-write".to_string()),
            );
            selections.insert(
                "approvalPolicy".to_string(),
                Value::String("never".to_string()),
            );
        }
        ("gemini-cli", PolicyTemplateName::Readonly) => {
            selections.insert(
                "approvalMode".to_string(),
                Value::String("plan".to_string()),
            );
        }
        ("gemini-cli", PolicyTemplateName::Standard) => {
            // `preview_args` omits any flag whose selected value is the literal string
            // "default" (its convention for "don't override, respect whatever's already
            // configured"). gemini-cli's own real ask-every-time mode happens to be spelled
            // "default" too, so recording it here is necessary for the effective-selections
            // view to be honest, but not sufficient on its own to guarantee the flag reaches
            // argv — `force_gemini_standard_approval_flag` does that after `preview_args` runs.
            selections.insert(
                "approvalMode".to_string(),
                Value::String("default".to_string()),
            );
        }
        ("gemini-cli", PolicyTemplateName::Trusted | PolicyTemplateName::Yolo) => {
            selections.insert(
                "approvalMode".to_string(),
                Value::String("yolo".to_string()),
            );
        }
        ("opencode", PolicyTemplateName::Readonly) => {
            selections.insert("agent".to_string(), Value::String("plan".to_string()));
        }
        ("opencode", PolicyTemplateName::Standard) => {}
        ("opencode", PolicyTemplateName::Trusted | PolicyTemplateName::Yolo) => {
            selections.insert("autoApprove".to_string(), Value::Bool(true));
        }
        _ => {}
    }
    selections
}

/// Forces `--approval-mode default` onto a gemini-cli `standard` launch's final argv, undoing
/// `preview_args`' general "a `default`-valued selection omits its flag" convention for this one
/// case. Without this, `standard` would silently fall through to the user's own
/// `~/.gemini/settings.json`, which may set anything (including `yolo`), breaking the guarantee
/// that `standard` reliably asks. Safe to call for any agent/template; it only acts on
/// `(gemini-cli, Standard)`.
pub(crate) fn force_gemini_standard_approval_flag(
    agent_id: &str,
    template: PolicyTemplateName,
    mut args: Vec<String>,
) -> Vec<String> {
    if agent_id != "gemini-cli" || template != PolicyTemplateName::Standard {
        return args;
    }
    if let Some(position) = args
        .iter()
        .position(|argument| argument == "--approval-mode")
    {
        let end = (position + 2).min(args.len());
        args.drain(position..end);
    }
    args.extend(["--approval-mode".to_string(), "default".to_string()]);
    args
}

/// Opencode's `standard` template has no expressible `cli_parameters` catalog value for "ask
/// before edits/bash, stay permissive for reads" (its `agent` enum is only `default`/`build`/
/// `plan`, none of which mean that) — this returns the `OPENCODE_PERMISSION` environment
/// variable to inject instead, so the generated terminal wrapper script can export it. `None`
/// for every other `(agent_id, template)` combination.
pub(crate) fn opencode_standard_permission_env_var(
    agent_id: &str,
    template: PolicyTemplateName,
) -> Option<(&'static str, &'static str)> {
    if agent_id == "opencode" && template == PolicyTemplateName::Standard {
        Some(("OPENCODE_PERMISSION", r#"{"edit":"ask","bash":"ask"}"#))
    } else {
        None
    }
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
        push_session_arg(args, flag, session_id);
    }
}

fn push_session_arg(args: &mut Vec<String>, flag: &str, session_id: &str) {
    args.extend([flag.to_string(), session_id.to_string()]);
}
