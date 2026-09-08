use crate::contexts::agent_runtime::application::{
    AgentChatConfiguration, ProviderInteractiveInvocationSpec, ProviderInvocationSpec,
    ProviderPromptDelivery,
};
use crate::contexts::permissions::api::PolicyTemplateName;
use crate::contexts::tooling::api::{
    CliLaunchScope, CliParameterSelection, CliParameterSelectionMap,
};
use std::fmt::{Display, Formatter};
/// Managed CLI agents whose chat and terminal launches receive a final policy projection.
///
/// Every built-in CLI is listed, including the ones whose terminal cannot express a template as a
/// flag: for those the projection is empty and `terminal_policy_enforceability` decides whether
/// the launch may proceed at all. Leaving an id out would make its launch fail with "no mapping"
/// instead of a truthful "this mode cannot enforce that policy".
pub(crate) const POLICY_TEMPLATE_GOVERNED_AGENT_IDS: [&str; 12] = [
    "claude-code",
    "codex-cli",
    "gemini-cli",
    "opencode",
    "antigravity-cli",
    "qwen-code",
    "kimi-cli",
    "qoder-cli",
    "codebuddy-code",
    "copilot-cli",
    "cursor-agent-cli",
    "iflow-cli",
];

/// Whether a PTY launch of `agent_id` can honour `template`.
///
/// The terminal is the CLI's own interactive surface: VaneHub can pass a reviewed flag, but it
/// cannot arbitrate the CLI's internal approvals the way the ACP bridge does. A template the CLI
/// cannot express as a flag is therefore either honoured by the CLI's own default (ask before
/// acting, which is what `standard` means) or not enforceable at all. `readonly` on a CLI with no
/// plan/read-only mode is the case that must refuse: launching anyway would hand a write-capable
/// terminal to a session whose policy forbids writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TerminalPolicyEnforceability {
    /// A reviewed flag or the CLI's own default expresses the template.
    HostProjected,
    /// The CLI asks before acting on its own; VaneHub passed no restriction and verified none.
    ProviderDelegated,
    /// No reviewed way to express the template on this transport. The launch must be refused.
    NotEnforceable { reason_code: &'static str },
}

/// Providers whose policy flags are rendered by the runtime (`direct_policy_arguments`) rather
/// than through catalog policy overrides, because the right flag depends on the transport.
pub(crate) const RUNTIME_POLICY_AGENT_IDS: [&str; 7] = [
    "qwen-code",
    "kimi-cli",
    "qoder-cli",
    "codebuddy-code",
    "copilot-cli",
    "cursor-agent-cli",
    "iflow-cli",
];

/// Policy flags rendered by the runtime itself for the seven expanded CLIs. The catalog renders
/// their user-editable parameters; the policy tokens come from here because they depend on the
/// transport. `None` for an id the runtime does not govern.
///
/// Every flag below was read from the installed program's own `--help` on 2026-09-06 (Qwen Code
/// 0.23.0, Kimi Code CLI 0.41.0, Qoder CLI 1.1.45, CodeBuddy Code 2.146.0, GitHub Copilot CLI
/// 1.0.83, Cursor Agent 2026.09.02, iFlow CLI 0.5.19); nothing here is inferred from another
/// vendor's grammar.
///
/// The transport matters. On a PTY the flag is the only policy the host can apply, so each
/// template is projected in full. Under ACP the agent asks the host through
/// `session/request_permission` and the host's policy answers per call; a permissive launch flag
/// would let the agent skip that question, so only the restrictive read-only posture is passed
/// and `standard`/`trusted`/`yolo` carry no flag at all.
pub(crate) fn direct_policy_arguments(
    agent_id: &str,
    template: PolicyTemplateName,
    scope: CliLaunchScope,
) -> Option<Vec<String>> {
    if !POLICY_TEMPLATE_GOVERNED_AGENT_IDS.contains(&agent_id) {
        return None;
    }
    let readonly = template == PolicyTemplateName::Readonly;
    let permissive = matches!(
        template,
        PolicyTemplateName::Trusted | PolicyTemplateName::Yolo
    );
    let terminal = scope == CliLaunchScope::Interactive;
    let flag = |tokens: &[&str]| tokens.iter().map(|token| (*token).to_string()).collect();
    Some(match agent_id {
        // `--approval-mode plan|default|auto-edit|auto|yolo`.
        "qwen-code" => match template {
            PolicyTemplateName::Readonly => flag(&["--approval-mode", "plan"]),
            PolicyTemplateName::Standard if terminal => flag(&["--approval-mode", "default"]),
            PolicyTemplateName::Trusted if terminal => flag(&["--approval-mode", "auto-edit"]),
            PolicyTemplateName::Yolo if terminal => flag(&["--approval-mode", "yolo"]),
            _ => Vec::new(),
        },
        // `--plan` (plan mode), `--yolo` (routine edits/commands run, risky actions still ask),
        // `--auto` (never ask). The default is ask-first.
        "kimi-cli" => match template {
            PolicyTemplateName::Readonly => flag(&["--plan"]),
            PolicyTemplateName::Trusted if terminal => flag(&["--yolo"]),
            PolicyTemplateName::Yolo if terminal => flag(&["--auto"]),
            _ => Vec::new(),
        },
        // `--permission-mode default|accept_edits|bypass_permissions|dont_ask|auto`. There is no
        // plan/read-only mode, so `readonly` stays unenforceable on the terminal.
        "qoder-cli" => match template {
            PolicyTemplateName::Standard if terminal => flag(&["--permission-mode", "default"]),
            _ if permissive && terminal => flag(&["--permission-mode", "accept_edits"]),
            _ => Vec::new(),
        },
        // `--permission-mode acceptEdits|bypassPermissions|default|plan|dontAsk|auto`.
        "codebuddy-code" => match template {
            PolicyTemplateName::Readonly => flag(&["--permission-mode", "plan"]),
            PolicyTemplateName::Standard if terminal => flag(&["--permission-mode", "default"]),
            _ if permissive && terminal => flag(&["--permission-mode", "acceptEdits"]),
            _ => Vec::new(),
        },
        // `--mode interactive|plan|autopilot`; `--allow-all-tools` runs tools without prompting
        // while path and URL checks stay on. The default is ask-first.
        "copilot-cli" => match template {
            PolicyTemplateName::Readonly => flag(&["--mode", "plan"]),
            _ if permissive && terminal => flag(&["--allow-all-tools"]),
            _ => Vec::new(),
        },
        // `--mode plan|ask`; `--force` allows commands unless explicitly denied. The default is
        // ask-first.
        "cursor-agent-cli" => match template {
            PolicyTemplateName::Readonly => flag(&["--mode", "plan"]),
            _ if permissive && terminal => flag(&["--force"]),
            _ => Vec::new(),
        },
        // `--plan`, `--default`, `--autoEdit`, `--yolo`. Terminal only: iFlow has no managed
        // conversation.
        "iflow-cli" => match template {
            PolicyTemplateName::Readonly => flag(&["--plan"]),
            PolicyTemplateName::Standard => flag(&["--default"]),
            PolicyTemplateName::Trusted => flag(&["--autoEdit"]),
            PolicyTemplateName::Yolo => flag(&["--yolo"]),
        },
        _ => {
            let _ = readonly;
            Vec::new()
        }
    })
}

pub(crate) fn terminal_policy_enforceability(
    agent_id: &str,
    template: PolicyTemplateName,
) -> TerminalPolicyEnforceability {
    use TerminalPolicyEnforceability::{HostProjected, NotEnforceable, ProviderDelegated};
    match (agent_id, template) {
        // The original five, Qwen, Kimi, CodeBuddy, and iFlow project every template through a
        // flag their own `--help` documents.
        (
            "claude-code" | "codex-cli" | "gemini-cli" | "opencode" | "antigravity-cli"
            | "qwen-code" | "kimi-cli" | "codebuddy-code" | "iflow-cli",
            _,
        ) => HostProjected,
        // Qoder has no plan/read-only launch mode; every other template has a `--permission-mode`.
        ("qoder-cli", PolicyTemplateName::Readonly) => NotEnforceable {
            reason_code: "terminal-readonly-unsupported",
        },
        ("qoder-cli", _) => HostProjected,
        // Copilot and Cursor: `--mode plan` for read-only, a pre-approval flag for trusted/yolo,
        // and their own ask-first default for standard.
        ("copilot-cli" | "cursor-agent-cli", PolicyTemplateName::Standard) => ProviderDelegated,
        ("copilot-cli" | "cursor-agent-cli", _) => HostProjected,
        _ => NotEnforceable {
            reason_code: "policy-mapping-missing",
        },
    }
}
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
        // Qwen Code's own `--resume <id>` / `--session-id <id>` pair (packages/cli/src/config/
        // config.ts). The shape mirrors Gemini's because Qwen's source defines the same two
        // options; it is verified against Qwen, not inherited.
        "qwen-code" => {
            if let Some(session_id) = existing_session_id {
                push_session_arg(&mut args, "--resume", session_id);
            } else {
                let session_id = uuid::Uuid::new_v4().to_string();
                push_session_arg(&mut args, "--session-id", &session_id);
                assigned_runtime_session_id = Some(session_id);
            }
        }
        // `kimi --session <id>` resumes a session by id (`-S`). No flag names a fresh session,
        // so a new launch lets the CLI mint its own id.
        "kimi-cli" => {
            if let Some(session_id) = existing_session_id {
                push_session_arg(&mut args, "--session", session_id);
            }
        }
        // Qoder 1.1.45 and CodeBuddy 2.146.0 both document `--resume <id>` and `--session-id <id>`,
        // so a fresh launch is named up front like Qwen's and a stored id resumes exactly.
        "qoder-cli" | "codebuddy-code" => {
            if let Some(session_id) = existing_session_id {
                push_session_arg(&mut args, "--resume", session_id);
            } else {
                let session_id = uuid::Uuid::new_v4().to_string();
                push_session_arg(&mut args, "--session-id", &session_id);
                assigned_runtime_session_id = Some(session_id);
            }
        }
        // Copilot 1.0.83: `-r, --resume[=value]`; the value must be attached with `=`.
        "copilot-cli" => {
            if let Some(session_id) = existing_session_id {
                args.push(format!("--resume={session_id}"));
            }
        }
        // Cursor 2026.09.02 `--resume [chatId]` and iFlow 0.5.19 `-r, --resume [id]` take the id
        // as the next token. Neither names a fresh session, so the CLI mints its own.
        "cursor-agent-cli" | "iflow-cli" => {
            if let Some(session_id) = existing_session_id {
                push_session_arg(&mut args, "--resume", session_id);
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
        // The seven expanded CLIs mirror `direct_policy_arguments` (terminal scope), keyed by the
        // parameter id a future catalog entry would carry. Values come from each program's own
        // `--help`, read on 2026-09-06.
        ("qwen-code", PolicyTemplateName::Readonly) => {
            overrides.insert(
                "approvalMode".to_string(),
                CliParameterSelection::text("plan"),
            );
        }
        ("qwen-code", PolicyTemplateName::Standard) => {
            overrides.insert(
                "approvalMode".to_string(),
                CliParameterSelection::text("default"),
            );
        }
        ("qwen-code", PolicyTemplateName::Trusted) => {
            overrides.insert(
                "approvalMode".to_string(),
                CliParameterSelection::text("auto-edit"),
            );
        }
        ("qwen-code", PolicyTemplateName::Yolo) => {
            overrides.insert(
                "approvalMode".to_string(),
                CliParameterSelection::text("yolo"),
            );
        }
        ("kimi-cli", PolicyTemplateName::Readonly) => {
            overrides.insert("plan".to_string(), CliParameterSelection::boolean(true));
        }
        ("kimi-cli", PolicyTemplateName::Standard) => {}
        ("kimi-cli", PolicyTemplateName::Trusted) => {
            overrides.insert("yolo".to_string(), CliParameterSelection::boolean(true));
        }
        ("kimi-cli", PolicyTemplateName::Yolo) => {
            overrides.insert("auto".to_string(), CliParameterSelection::boolean(true));
        }
        ("qoder-cli", PolicyTemplateName::Readonly) => {}
        ("qoder-cli", PolicyTemplateName::Standard) => {
            overrides.insert(
                "permissionMode".to_string(),
                CliParameterSelection::text("default"),
            );
        }
        ("qoder-cli", PolicyTemplateName::Trusted | PolicyTemplateName::Yolo) => {
            overrides.insert(
                "permissionMode".to_string(),
                CliParameterSelection::text("accept_edits"),
            );
        }
        ("codebuddy-code", PolicyTemplateName::Readonly) => {
            overrides.insert(
                "permissionMode".to_string(),
                CliParameterSelection::text("plan"),
            );
        }
        ("codebuddy-code", PolicyTemplateName::Standard) => {
            overrides.insert(
                "permissionMode".to_string(),
                CliParameterSelection::text("default"),
            );
        }
        ("codebuddy-code", PolicyTemplateName::Trusted | PolicyTemplateName::Yolo) => {
            overrides.insert(
                "permissionMode".to_string(),
                CliParameterSelection::text("acceptEdits"),
            );
        }
        ("copilot-cli", PolicyTemplateName::Readonly) => {
            overrides.insert("mode".to_string(), CliParameterSelection::text("plan"));
        }
        ("copilot-cli", PolicyTemplateName::Standard) => {}
        ("copilot-cli", PolicyTemplateName::Trusted | PolicyTemplateName::Yolo) => {
            overrides.insert(
                "allowAllTools".to_string(),
                CliParameterSelection::boolean(true),
            );
        }
        ("cursor-agent-cli", PolicyTemplateName::Readonly) => {
            overrides.insert("mode".to_string(), CliParameterSelection::text("plan"));
        }
        ("cursor-agent-cli", PolicyTemplateName::Standard) => {}
        ("cursor-agent-cli", PolicyTemplateName::Trusted | PolicyTemplateName::Yolo) => {
            overrides.insert("force".to_string(), CliParameterSelection::boolean(true));
        }
        ("iflow-cli", PolicyTemplateName::Readonly) => {
            overrides.insert("plan".to_string(), CliParameterSelection::boolean(true));
        }
        ("iflow-cli", PolicyTemplateName::Standard) => {
            overrides.insert("default".to_string(), CliParameterSelection::boolean(true));
        }
        ("iflow-cli", PolicyTemplateName::Trusted) => {
            overrides.insert("autoEdit".to_string(), CliParameterSelection::boolean(true));
        }
        ("iflow-cli", PolicyTemplateName::Yolo) => {
            overrides.insert("yolo".to_string(), CliParameterSelection::boolean(true));
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
