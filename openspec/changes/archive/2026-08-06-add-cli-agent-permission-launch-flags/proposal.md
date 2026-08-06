## Why

Phase 1 (`permissions-core`) built a generic policy-template PDP (readonly/standard/trusted/yolo) that works for any agent id. Phase 2 (`claude-code-permission-hook`) wired Claude Code to it via dynamic per-call hooks. The three other managed CLI agents — `gemini-cli`, `codex-cli`, `opencode` — have no equivalent per-call interception surface, so today an assigned policy template has zero effect when VaneHub launches them interactively: the Agent Policies page doesn't even list them, and nothing projects their template into a launch argument. Three of the four managed CLI agents currently run with whatever the user happened to save in CLI Parameters, regardless of any policy template assigned to them.

## What Changes

- Add a template → launch-flag/env-var projection for `gemini-cli`, `codex-cli`, and `opencode`, applied once at Agent Terminal launch time (not per tool-call — these tools expose no per-call interception point). Each template maps to values already legal in that tool's existing `cli_parameters` catalog (`sandbox`/`approvalPolicy` for codex-cli, `approvalMode` for gemini-cli, `agent`/`autoApprove` for opencode), plus a new `OPENCODE_PERMISSION` environment variable injected via the generated terminal wrapper script so opencode's `standard` template can express "ask before edits/bash" (no existing catalog value covers that case).
- `standard` is designed to enable each tool's own native interactive approval prompting (not a degraded binary choice) — this works because VaneHub always launches these CLIs through a real interactive PTY, never headless, so the tool's own prompt renders and is answerable by the user. Per-call decisions made this way are not routed through VaneHub's `evaluate()` and are not recorded in the unified `approval_audit` trail — an explicitly accepted asymmetry with Claude Code's hook-based mechanism.
- Generalize the Agent Policies settings page from hand-special-casing the single `claude-code` CLI principal to listing all four managed CLI principals (`claude-code`, `codex-cli`, `gemini-cli`, `opencode`), reusing the existing trusted/yolo confirmation flow unchanged. The `claude-code`-only hook-install confirmation is unaffected and stays claude-code-specific.
- **BREAKING** (spec-level contract, not a user-facing break): narrows `cli-parameter-management`'s and `agent-terminal-runtime`'s existing guarantee that the saved CLI Parameter profile is the *single, unoverridden* argument source for interactive launches. A narrow, explicit exception is carved out: an assigned policy template's governed parameters now take precedence over the persisted CLI Parameter profile for interactive launches of these three tools. Every other parameter (model, effort, thinking, chrome, ephemeral, strictConfig, etc.) is completely unaffected and continues to resolve exactly as today.
- No changes to `cli_config`, no new persisted config files (`settings.json`/`opencode.json`/`config.toml` stay untouched), no hook bridge / wrapper binary / loopback server like Phase 2 — this reuses the existing `cli_parameters` selections → `preview_args` → `managed_args` pipeline plus one small wrapper-script addition.

## Capabilities

### New Capabilities
- `cli-agent-permission-launch-flags`: projects an agent principal's assigned policy template (readonly/standard/trusted/yolo) into `gemini-cli`/`codex-cli`/`opencode`'s own native launch-time approval and sandbox controls, reusing each tool's existing graduated modes rather than raw bypass flags, and takes precedence over the user's persisted CLI Parameter selections for the specific keys it governs.

### Modified Capabilities
- `cli-parameter-management`: the "Agent Terminal uses interactive profile only" and "Deterministic configuration precedence" requirements are narrowed for interactive launches of the three tools above — an assigned policy template's governed parameters now win over the persisted CLI Parameter profile; all other parameters are unaffected.
- `agent-terminal-runtime`: the "Interactive CLI profile injection" requirement gains the same narrow, template-driven exception, stated from the terminal runtime's side.
- `permissions-approval`: "Agent policy list surfaces every eligible agent's current template" broadens from listing only the `claude-code` CLI principal to listing all four managed CLI principals (`claude-code`, `codex-cli`, `gemini-cli`, `opencode`).

## Impact

- Desktop runtime only. The Web/mock runtime doesn't spawn real CLI processes; its Agent Policies list mock already generalizes for free once the frontend loop change lands.
- New Rust: `apply_policy_template_overrides` (pure function, `agent_runtime::infrastructure::providers`), a new `PolicyTemplateLookupPort` in `agent_runtime::application` plus an adapter over `PermissionsApi` wired in bootstrap (agent_runtime's first call *into* `contexts::permissions`; Phase 2's cross-context port went the other direction), a small addition to wrapper-script generation for opencode's `OPENCODE_PERMISSION` env var.
- Modified Rust: `RuntimeAgentCliProfileAdapter::load_interactive` (`src-tauri/src/contexts/agent_runtime/infrastructure/cli_profile.rs`).
- Modified frontend: `src/settings/pages/agent-policies-page.tsx` (generalized row rendering over `MANAGED_CLI_AGENT_IDS`), i18n additions across all five locale files, one static help-text line on the CLI Parameter Management page.
- No changes to `cli_config`, `cli-agent-config-management`, or any persisted provider config file.
