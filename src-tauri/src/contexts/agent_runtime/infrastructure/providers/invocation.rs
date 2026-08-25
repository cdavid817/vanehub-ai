use crate::contexts::agent_runtime::application::{
    AgentChatConfiguration, ProviderInteractiveInvocationSpec, ProviderInvocationSpec,
    ProviderPromptDelivery,
};
use crate::contexts::permissions::api::PolicyTemplateName;
use crate::contexts::tooling::api::{CliParameterSelection, CliParameterSelectionMap};
use std::fmt::{Display, Formatter};
/// Managed CLI agents whose chat and terminal launches receive a final policy projection.
pub(crate) const POLICY_TEMPLATE_GOVERNED_AGENT_IDS: [&str; 5] = [
    "claude-code",
    "codex-cli",
    "gemini-cli",
    "opencode",
    "antigravity-cli",
];
/// The two registry-declared placement slots, resolved by the Tooling CLI-parameter API. The
/// provider grammar decides where each lands; the builder never inspects a token's spelling.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ProviderLaunchSegments<'a> {
    pub(crate) global: &'a [String],
    pub(crate) invocation: &'a [String],
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
/// Builds an invocation, optionally placing a seat's role briefing in the CLI's own system-prompt
/// channel.
///
/// The briefing must not travel as ordinary prompt text: that channel is subject to context
/// compaction, so a long session would drop the role and the Agent would quietly stop playing it.
/// Agents with no such channel get no briefing here — the caller falls back to per-turn injection
/// and marks the seat as not compaction-immune, rather than this silently dropping it.
pub(crate) fn build_invocation_with_role(
    agent_id: &str,
    executable: String,
    prompt: &str,
    runtime_session_id: Option<&str>,
    segments: ProviderLaunchSegments<'_>,
    role_briefing: Option<&str>,
) -> Result<ProviderInvocationSpec, ProviderInvocationError> {
    let briefing = role_briefing.map(str::trim).filter(|text| !text.is_empty());
    let Some(briefing) = briefing else {
        return build_invocation(agent_id, executable, prompt, runtime_session_id, segments);
    };
    let extra: Vec<String> = match agent_id {
        "claude-code" => vec!["--append-system-prompt".to_string(), briefing.to_string()],
        "codex-cli" => vec![
            "-c".to_string(),
            format!("developer_instructions={briefing}"),
        ],
        // No native channel; the caller injects per turn instead.
        _ => Vec::new(),
    };
    // The briefing is a runtime-owned argument, not a registry parameter. It rides in the global
    // position so its placement is unchanged by the segment split.
    let mut global = segments.global.to_vec();
    global.extend(extra);
    build_invocation(
        agent_id,
        executable,
        prompt,
        runtime_session_id,
        ProviderLaunchSegments {
            global: &global,
            invocation: segments.invocation,
        },
    )
}
pub(crate) fn build_invocation(
    agent_id: &str,
    executable: String,
    prompt: &str,
    runtime_session_id: Option<&str>,
    segments: ProviderLaunchSegments<'_>,
) -> Result<ProviderInvocationSpec, ProviderInvocationError> {
    let mut args = Vec::new();
    let prompt_delivery = match agent_id {
        "claude-code" => {
            args.extend_from_slice(segments.global);
            args.extend_from_slice(segments.invocation);
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
        // The only provider whose two slots straddle a subcommand: options before `exec` are
        // global, options the `exec` grammar owns (such as `--ephemeral`) follow the resume pair.
        // Placement is registry-declared; no argument is matched by spelling here.
        "codex-cli" => {
            args.extend_from_slice(segments.global);
            args.push("exec".to_string());
            if let Some(session_id) = non_empty_session_id(runtime_session_id) {
                args.extend(["resume".to_string(), session_id.to_string()]);
            }
            args.extend_from_slice(segments.invocation);
            args.extend(["--json".to_string(), "-".to_string()]);
            ProviderPromptDelivery::Stdin
        }
        // Stdin, not `-p`, and this one is forced by the platform rather than chosen. On Windows
        // `gemini` is an npm batch shim with no `.exe` beside it, and since the BatBadBut
        // hardening Rust's `std::process::Command` refuses outright to pass a `.cmd` any argument
        // containing CR or LF — "batch file arguments are invalid", before `CreateProcess` is even
        // reached. A composed prompt always spans lines, so argv delivery could never spawn.
        // `cmd.exe`'s 8,191-character command line is the second wall behind it: past that the
        // spawn succeeds and the child receives empty argv, losing the prompt silently.
        //
        // The CLI documents `-p` as "Appended to input on stdin (if any)", and with no `-p` it
        // reads stdin as the prompt, so this is the same request through the channel that has
        // neither limit.
        "gemini-cli" => {
            args.extend_from_slice(segments.global);
            args.extend_from_slice(segments.invocation);
            push_resume_args(&mut args, runtime_session_id, "--resume");
            args.extend(["-o".to_string(), "stream-json".to_string()]);
            ProviderPromptDelivery::Stdin
        }
        "opencode" => {
            args.extend_from_slice(segments.global);
            args.push("run".to_string());
            args.extend_from_slice(segments.invocation);
            push_resume_args(&mut args, runtime_session_id, "--session");
            args.extend([
                "--format".to_string(),
                "json".to_string(),
                prompt.to_string(),
            ]);
            ProviderPromptDelivery::Argument
        }
        // `-p` takes the prompt as its value, so the prompt travels as an argument the way it does
        // for gemini-cli rather than through stdin. Flags verified against `agy --help` (v1.1.11).
        "antigravity-cli" => {
            args.extend_from_slice(segments.global);
            args.extend_from_slice(segments.invocation);
            push_resume_args(&mut args, runtime_session_id, "--conversation");
            args.extend([
                "-p".to_string(),
                prompt.to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
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
    segments: ProviderLaunchSegments<'_>,
) -> Result<ProviderInteractiveInvocationSpec, ProviderInvocationError> {
    let mut args = Vec::new();
    let existing_session_id = non_empty_session_id(runtime_session_id);
    let mut assigned_runtime_session_id = None;
    // No interactive grammar has a subcommand, so both segments precede the session arguments in
    // declared order.
    args.extend_from_slice(segments.global);
    args.extend_from_slice(segments.invocation);
    match agent_id {
        "claude-code" => {
            if let Some(session_id) = existing_session_id {
                push_session_arg(&mut args, "--resume", session_id);
            } else {
                let session_id = uuid::Uuid::new_v4().to_string();
                push_session_arg(&mut args, "--session-id", &session_id);
                assigned_runtime_session_id = Some(session_id);
            }
        }
        "codex-cli" => {
            if let Some(session_id) = existing_session_id {
                args.extend(["resume".to_string(), session_id.to_string()]);
            }
        }
        "gemini-cli" => {
            if let Some(session_id) = existing_session_id {
                push_session_arg(&mut args, "--resume", session_id);
            } else {
                let session_id = uuid::Uuid::new_v4().to_string();
                push_session_arg(&mut args, "--session-id", &session_id);
                assigned_runtime_session_id = Some(session_id);
            }
        }
        "opencode" => {
            if let Some(session_id) = existing_session_id {
                push_session_arg(&mut args, "--session", session_id);
            }
        }
        // No id can be assigned up front: `agy` has `--conversation <id>` to resume an existing
        // conversation but no documented flag to name a new one, so a fresh interactive launch
        // lets the CLI mint its own id and picks it up from the `init` event.
        "antigravity-cli" => {
            if let Some(session_id) = existing_session_id {
                push_session_arg(&mut args, "--conversation", session_id);
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
pub(crate) fn add_opencode_directory_args(args: &mut Vec<String>, directory: &str) {
    let insert_at = args.len().saturating_sub(1);
    args.splice(
        insert_at..insert_at,
        ["--dir".to_string(), directory.to_string()],
    );
}
/// Per-message overrides for ordinary parameters only. A message can govern model, reasoning
/// depth, and opencode's thinking display; every other parameter stays absent so the saved profile
/// decides. Policy-governed and runtime-reserved ids are never produced here.
pub(crate) fn message_override_selections(
    agent_id: &str,
    configuration: &AgentChatConfiguration,
) -> CliParameterSelectionMap {
    let mut overrides = CliParameterSelectionMap::new();
    if let Some(model) = configuration
        .model_id
        .as_deref()
        .and_then(|model_id| mapped_model(agent_id, model_id))
    {
        overrides.insert("model".to_string(), CliParameterSelection::text(model));
    }
    if let Some(reasoning_depth) = configuration.reasoning_depth.as_deref() {
        match agent_id {
            "claude-code" => {
                overrides.insert(
                    "effort".to_string(),
                    CliParameterSelection::text(reasoning_depth),
                );
            }
            "codex-cli" => {
                let effort = if reasoning_depth == "max" {
                    "xhigh"
                } else {
                    reasoning_depth
                };
                overrides.insert(
                    "reasoningEffort".to_string(),
                    CliParameterSelection::text(effort),
                );
            }
            // `agy --effort` accepts only low|medium|high, so anything above high clamps rather
            // than being passed through and rejected by the CLI.
            "antigravity-cli" => {
                let effort = if matches!(reasoning_depth, "max" | "xhigh") {
                    "high"
                } else {
                    reasoning_depth
                };
                overrides.insert("effort".to_string(), CliParameterSelection::text(effort));
            }
            _ => {}
        }
    }
    if agent_id == "opencode" {
        overrides.insert(
            "thinking".to_string(),
            CliParameterSelection::boolean(configuration.thinking),
        );
    }
    overrides
}
/// Projects an agent principal's assigned policy template onto the registry's policy-governed
/// parameters. These values never come from a saved profile or a message: the resolver accepts
/// them on a separate input and refuses a user-editable id on that path.
///
/// `trusted` and `yolo` deliberately resolve identically, matching the established
/// `permissions-core` precedent that the two templates already resolve identically in
/// `evaluate()` — the difference between them is assignment-time confirmation friction, not
/// technical capability.
///
/// `opencode`'s `standard` deliberately makes no selection: no opencode catalog value means "ask
/// before edits/bash, stay permissive for reads," so that template is expressed via an injected
/// `OPENCODE_PERMISSION` environment variable instead (see `terminal_wrapper.rs`).
///
/// An `Inherit` entry is an explicit "this template emits no token for that parameter" and is what
/// the pre-cutover code expressed by storing the sentinel string `default`.
pub(crate) fn policy_override_selections(
    agent_id: &str,
    template: PolicyTemplateName,
) -> CliParameterSelectionMap {
    let mut overrides = CliParameterSelectionMap::new();
    match (agent_id, template) {
        ("claude-code", PolicyTemplateName::Readonly) => {
            overrides.insert(
                "permissionMode".to_string(),
                CliParameterSelection::text("plan"),
            );
        }
        // Claude Code's own ask-before-acting default. The registry does not expose `default` as a
        // provider value here, so `standard` emits no flag, exactly as before the cutover.
        ("claude-code", PolicyTemplateName::Standard) => {
            overrides.insert("permissionMode".to_string(), CliParameterSelection::Inherit);
        }
        ("claude-code", PolicyTemplateName::Trusted | PolicyTemplateName::Yolo) => {
            overrides.insert(
                "permissionMode".to_string(),
                CliParameterSelection::text("acceptEdits"),
            );
        }
        ("codex-cli", PolicyTemplateName::Readonly) => {
            overrides.insert(
                "sandbox".to_string(),
                CliParameterSelection::text("read-only"),
            );
            overrides.insert(
                "approvalPolicy".to_string(),
                CliParameterSelection::text("never"),
            );
        }
        ("codex-cli", PolicyTemplateName::Standard) => {
            overrides.insert(
                "sandbox".to_string(),
                CliParameterSelection::text("workspace-write"),
            );
            overrides.insert(
                "approvalPolicy".to_string(),
                CliParameterSelection::text("on-request"),
            );
        }
        ("codex-cli", PolicyTemplateName::Trusted | PolicyTemplateName::Yolo) => {
            overrides.insert(
                "sandbox".to_string(),
                CliParameterSelection::text("workspace-write"),
            );
            overrides.insert(
                "approvalPolicy".to_string(),
                CliParameterSelection::text("never"),
            );
        }
        ("gemini-cli", PolicyTemplateName::Readonly) => {
            overrides.insert(
                "approvalMode".to_string(),
                CliParameterSelection::text("plan"),
            );
        }
        // `default` is gemini-cli's own real ask-every-time mode. The registry declares it as a
        // provider value rather than an inheritance sentinel, so it renders declaratively and the
        // post-render fixup this used to need is gone.
        ("gemini-cli", PolicyTemplateName::Standard) => {
            overrides.insert(
                "approvalMode".to_string(),
                CliParameterSelection::text("default"),
            );
        }
        ("gemini-cli", PolicyTemplateName::Trusted | PolicyTemplateName::Yolo) => {
            overrides.insert(
                "approvalMode".to_string(),
                CliParameterSelection::text("yolo"),
            );
        }
        ("opencode", PolicyTemplateName::Readonly) => {
            overrides.insert("agent".to_string(), CliParameterSelection::text("plan"));
        }
        ("opencode", PolicyTemplateName::Standard) => {}
        ("opencode", PolicyTemplateName::Trusted | PolicyTemplateName::Yolo) => {
            overrides.insert(
                "autoApprove".to_string(),
                CliParameterSelection::boolean(true),
            );
        }
        // `--mode` is Antigravity's own graduated execution mode, so the projection uses it rather
        // than the `--dangerously-skip-permissions` bypass flag the non-bypass rule forbids.
        ("antigravity-cli", PolicyTemplateName::Readonly) => {
            overrides.insert("mode".to_string(), CliParameterSelection::text("plan"));
            overrides.insert("sandbox".to_string(), CliParameterSelection::boolean(true));
        }
        ("antigravity-cli", PolicyTemplateName::Standard) => {
            // No mode override: the CLI's own `request-review` default is exactly the
            // ask-before-acting posture `standard` means.
            overrides.insert("mode".to_string(), CliParameterSelection::Inherit);
            overrides.insert("sandbox".to_string(), CliParameterSelection::boolean(false));
        }
        ("antigravity-cli", PolicyTemplateName::Trusted | PolicyTemplateName::Yolo) => {
            overrides.insert(
                "mode".to_string(),
                CliParameterSelection::text("accept-edits"),
            );
            overrides.insert("sandbox".to_string(), CliParameterSelection::boolean(false));
        }
        _ => {}
    }
    overrides
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
