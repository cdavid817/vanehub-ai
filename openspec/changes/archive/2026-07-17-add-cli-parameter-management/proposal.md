## Why

VaneHub currently hard-codes provider CLI invocation arguments, so users cannot define persistent launch defaults for Claude Code, Codex CLI, Gemini CLI, and OpenCode. A typed CLI parameter management capability is needed to expose safe, documented choices without allowing saved values to break provider-required session, streaming, or prompt arguments.

## What Changes

- Add a dedicated CLI Parameter Management page to the settings center for all four managed CLIs.
- Provide a curated, typed parameter catalog with translated parameter descriptions, translated value descriptions, enum selectors, repeatable-value selectors where explicitly supported, and boolean switches.
- Persist validated selections in SQLite for the desktop runtime and in Web/mock storage behind the same frontend service contract.
- Inject saved selections into new interactive CLI launches and new chat-runtime process invocations through provider-specific argument builders.
- Define precedence as per-message chat configuration over persisted CLI defaults over provider defaults, while keeping VaneHub-owned protocol, output, resume, and prompt arguments reserved.
- Add explicit per-CLI save and restore-default actions, dirty state, validation errors, and safe effective-argument previews.
- Keep dangerous permission-bypass flags out of the first-version catalog and prohibit arbitrary raw argument entry.
- Preserve equivalent behavior and visual quality in the `futuristic` and `minimal` themes and provide complete `zh-CN` and `en` resources.
- Document deferred optimization paths for catalog versioning, runtime capability discovery, advanced custom arguments, richer value sources, profile/scope support, and future command-builder extraction.

## Capabilities

### New Capabilities

- `cli-parameter-management`: Typed parameter discovery, selection, persistence, validation, reset behavior, effective previews, and provider-specific launch injection for the four managed CLIs.

### Modified Capabilities

- `settings-center-ui`: Adds the CLI Parameter Management navigation entry and its settings-page interaction states.
- `frontend-runtime-architecture`: Extends the service and runtime adapter contract for CLI parameter profiles in both Tauri and Web/mock runtimes.
- `native-runtime-architecture`: Adds native persistence, validation, Tauri commands, and provider-owned argument composition for saved CLI parameters.
- `chat-experience`: Applies persisted defaults and per-message overrides to new provider CLI chat invocations without changing an already running process.

## Impact

- Frontend: settings page registry, new page/components, typed contracts, agent service methods, Tauri/Web adapters, local mock persistence, i18n resources, semantic theme styling, and component tests.
- Native runtime: a new SQLite migration and repository, CLI parameter commands, typed catalog validation, provider-specific argument construction, interactive launch routing, chat invocation composition, and redacted diagnostics.
- Contracts: new CLI parameter definition/profile/selection types and precedence rules across the frontend service boundary.
- Testing: frontend unit/component tests, adapter contract tests, Rust persistence and argument-builder tests, Playwright visual/interaction checks in both themes and both languages, plus the full project verification suite.
- Dependencies: no new state-management, UI-library, database, or package-manager dependency is introduced.
