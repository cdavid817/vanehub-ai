## Why

VaneHub currently sends chat prompts directly from the session composer to the selected CLI runtime, which leaves no structured way to shape agent behavior across Claude Code, Codex CLI, Gemini CLI, and OpenCode. A Prompt Hook system gives users fine-grained, auditable control over default context, behavior constraints, routing hints, and per-turn guidance while preserving the existing service boundary and multi-runtime architecture.

## What Changes

- Introduce a global Prompt Hook registry with VaneHub-defined default hooks organized by category: `bootstrap`, `callback`, `dynamic`, `law`, `navigation`, `routing`, and `static`.
- Add Prompt Hook manifests, rendered templates, resolver metadata, enabled state, CLI bindings, governance flags, and safe trace summaries.
- Add a service-backed Prompt Hooks settings page where users can inspect hooks, filter by category, enable or disable disableable hooks, bind hooks to supported CLI agents, manage user-created hooks, preview rendered content explicitly, and inspect trace summaries.
- Apply enabled Prompt Hooks before CLI chat invocation for `claude-code`, `codex-cli`, `gemini-cli`, and `opencode` through a shared pipeline rather than provider-specific duplicated logic.
- Preserve chat history semantics by storing and displaying the user's original message while sending the assembled effective prompt to the provider runtime.
- Persist Prompt Hook overrides and user-defined hooks through the Rust/SQLite layer in desktop mode, with deterministic Web/mock adapter parity.
- Record Prompt Hook injection trace metadata through unified logging with redaction; full rendered content is not shown or logged by default.

## Capabilities

### New Capabilities

- `prompt-hook-management`: Defines Prompt Hook manifests, registry behavior, default and user-created hooks, global enablement, CLI binding, preview, trace summaries, persistence, and Web/mock parity.
- `settings-prompt-hooks-ui`: Defines the service-backed Prompt Hooks settings page, visual consistency, filtering, management controls, preview behavior, trace display, and i18n requirements.

### Modified Capabilities

- `settings-center-ui`: Adds the Prompt Hooks settings navigation entry and preserves shared settings shell behavior, style, scrolling, and localization guarantees.
- `chat-experience`: Applies Prompt Hooks to CLI chat invocation while preserving original user-message persistence and frontend service boundary rules.
- `unified-log-management`: Requires Prompt Hook trace diagnostics to use the unified logging service with prompt-content redaction.
- `frontend-runtime-architecture`: Extends frontend service interfaces and runtime adapters for Prompt Hook management parity between Tauri and Web/mock runtimes.
- `native-runtime-architecture`: Adds native Prompt Hook registry, SQLite persistence, pipeline execution, and provider-agnostic integration before CLI invocation.

## Impact

- Frontend service API: `AgentService` gains Prompt Hook list, mutate, preview, and trace query methods implemented by both Tauri and Web/mock adapters.
- Frontend UI: `src/settings/settings-pages.ts`, settings i18n resources, and a new Prompt Hooks page and subcomponents under `src/settings/pages/`.
- Native runtime: Rust commands, SQLite migration/schema, Prompt Hook manifest/template loading, override storage, user hook storage, trace summaries, and prompt assembly before provider invocation.
- Chat runtime: `send_message` continues persisting the original user content but passes an assembled prompt into the existing provider-specific CLI invocation builders.
- Logging: Prompt Hook execution summaries are written through unified logging with hook ids, status, content hash, token estimate, and redacted diagnostics.
- Tests: frontend service and settings page tests, Web/mock parity tests, Rust registry/pipeline tests, chat invocation tests for four CLI agent ids, i18n parity, and OpenSpec strict validation.
