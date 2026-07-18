## Context

The current chat path trims the user's draft, persists it as a user message, and passes the same content into the Rust CLI runtime. Provider-specific invocation builders then map the selected stable agent id to the supported headless command shape for Claude Code, Codex CLI, Gemini CLI, or OpenCode. There is no structured pre-invocation layer for default behavior, context shaping, routing hints, or prompt governance.

Clowder-AI demonstrates a useful architecture: hook manifests, category-like id families, templates, resolvers, pipeline execution, and trace summaries. VaneHub will not copy Clowder-AI hook content. It will define a smaller, product-neutral default set and keep the architecture extensible for user-created hooks and future CLI agents.

## Goals / Non-Goals

**Goals:**

- Add a global Prompt Hook system with VaneHub-defined default hooks categorized as `bootstrap`, `callback`, `dynamic`, `law`, `navigation`, `routing`, and `static`.
- Support first-version global enablement and CLI binding for `claude-code`, `codex-cli`, `gemini-cli`, and `opencode`.
- Allow built-in hooks to be enabled or disabled when `disableable=true`, and bound to CLI agents, while keeping built-in hook content immutable from the UI.
- Allow user-created hooks to be created, edited, deleted, enabled, disabled, bound to CLI agents, previewed, and used by the same pipeline.
- Apply hooks before CLI chat invocation through one shared runtime pipeline and preserve original user-message history.
- Provide trace summaries by default and require explicit user action before rendering full hook content previews.
- Keep React components behind service interfaces and keep SQLite, filesystem, and process launch behavior in Rust.

**Non-Goals:**

- Workspace-scoped, project-scoped, or session-scoped hook overrides.
- Importing Clowder-AI's specific 40+ hook texts into VaneHub.
- Remote prompt pack marketplaces, signed prompt packs, or cloud synchronization.
- Hook execution that runs arbitrary user scripts or external commands.
- Provider-specific duplicate prompt-hook implementations.
- Prompt Hook support for browser interaction mode or non-CLI session execution in v1.

## Decisions

1. Use manifest-backed hooks with runtime overrides.

   Built-in hooks are defined by backend-owned manifests and templates. Mutable state such as enabled overrides and CLI bindings is persisted separately in SQLite. User-created hooks are persisted in SQLite with equivalent normalized fields and template body content.

   Alternative considered: store all hooks as editable frontend JSON. This would weaken desktop/runtime ownership, make Web/mock parity dishonest, and invite React components to own native persistence details.

2. Execute hooks in Rust before provider invocation.

   Prompt Hook assembly runs after `send_message` validates the active session and configuration, and before `spawn_cli_generation` builds the provider command. The pipeline returns the effective prompt, fired patches, and trace summaries. Provider builders receive only the final assembled prompt and remain responsible for CLI-specific argument/stdin placement.

   Alternative considered: assemble hooks in React before `sendMessage`. That would duplicate security-sensitive prompt handling in the browser, make native IM or future non-React callers inconsistent, and risk persisting hook-expanded text as the user message.

3. Preserve original user-message persistence.

   The user message stored in chat history remains the trimmed user input. The effective prompt is transient runtime input and may be represented only through redacted diagnostics and explicit preview APIs.

   Alternative considered: store assembled prompt as the user message. This would make histories noisy, leak system guidance into user-visible transcripts, and complicate future audit/redaction.

4. Keep v1 scope global plus CLI bindings.

   First version state is global and optionally bound to stable CLI agent ids. Scope layering for workspace or session can be added later without changing the manifest or pipeline core.

   Alternative considered: support global, workspace, session, and agent scope immediately. That increases precedence complexity before the product has proven the management model.

5. Trace summaries are safe by default.

   Trace rows include hook id, category, stage, status, version, content hash, token estimate, skip reason, and timestamp. Full rendered content is only returned by explicit preview commands and is not written to unified logs by default.

   Alternative considered: always persist rendered hook content for debugging. This conflicts with prompt privacy and existing unified logging redaction requirements.

6. Default hooks are product-neutral and minimal.

   VaneHub will define a first default set with one or two hooks in each category: session/workspace context, response-format baseline, safety/permission boundaries, session configuration, enabled-skill summary, project navigation hints, CLI capability hints, and a disabled callback placeholder.

   Alternative considered: port Clowder-AI's complete hook library. The content is domain-specific and would create governance and localization debt before VaneHub has its own default behavior model.

## Risks / Trade-offs

- Prompt expansion can increase token usage -> Show token estimates in trace summaries and previews, and keep default hooks small.
- Hook injection can make agent behavior harder to debug -> Store safe trace summaries and provide explicit preview of effective hook content.
- A non-disableable built-in hook may frustrate users -> Limit immutable hooks to safety and product invariants; make normal behavior hooks disableable.
- User hooks may contain sensitive or harmful content -> Treat user hook content as prompt text only, reject control characters and unsupported manifest fields, never execute scripts, and redact logs.
- Provider CLIs differ in system prompt support -> Assemble a single effective prompt envelope first; provider builders decide whether it is sent through stdin, arguments, or supported system/developer instruction flags.
- Web/mock mode cannot launch CLIs -> Web adapter still supports deterministic management, preview, and mock trace behavior without claiming native execution.

## Migration Plan

1. Add SQLite migrations for prompt hook overrides, user hooks, CLI bindings, and recent trace summaries without modifying existing chat messages.
2. Seed built-in hook definitions from backend-owned catalog data when listing hooks; do not duplicate seed rows into SQLite unless an override exists.
3. Extend AgentService and both adapters with prompt hook management and preview methods.
4. Add the Prompt Hooks settings navigation entry and page using existing settings primitives and i18n resources.
5. Integrate the Rust pipeline into CLI chat execution before provider invocation.
6. Add frontend, Web/mock, Rust, and OpenSpec validation coverage.

Rollback is additive: disabling all hooks or removing their CLI bindings restores current prompt behavior while preserving stored user hooks and overrides for future re-enable.

## Open Questions

- Whether the full effective prompt preview should be available from the chat composer later, or only from the Prompt Hooks settings page in v1.
- Whether future workspace/session scope should use override precedence of session > workspace > global, or separate named hook profiles.
