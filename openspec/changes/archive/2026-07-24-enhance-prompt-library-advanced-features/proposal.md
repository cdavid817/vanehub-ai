## Why

Prompt Hooks currently support only two fixed placeholders, overwrite the active user Hook in place, and expose trace events without version-level outcome metrics. Users need safer iteration: richer runtime variables, a draft/publish/rollback lifecycle, and evidence about which published version performs reliably and quickly.

## What Changes

- Add a backend-owned, allowlisted variable registry for Prompt Hook templates, including snake-case variables such as `{{agent_name}}` and `{{current_time}}`, with deterministic preview values and explicit handling of unknown variables.
- Add draft, publish, version-history, and rollback operations for user-created Prompt Hooks. Published versions are immutable; rollback republishes selected historical content as a new version so audit history is preserved.
- Ensure only the currently published version participates in live prompt assembly; draft edits never affect running Agent sessions until published.
- Record safe per-execution outcome metadata for each fired Prompt Hook version, including success/failure and elapsed milliseconds, without storing rendered Prompt content.
- Aggregate version-level execution count, success rate, failure count, and latency statistics for comparison in the Prompt Hooks settings page.
- Extend both Tauri and Web/mock adapters with equivalent version-management, variable-catalog, and evaluation contracts.
- Add localized settings UI for variable discovery, draft status/actions, version history, rollback, and version performance summaries.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `prompt-hook-management`: Add allowlisted dynamic variables, published-version selection, draft/publish/rollback semantics, and safe version-level execution evaluation.
- `settings-prompt-hooks-ui`: Add variable guidance, lifecycle controls, history/rollback UI, and per-version effectiveness summaries.
- `frontend-runtime-architecture`: Extend the runtime-neutral Prompt Hook service contract and preserve Tauri/Web adapter parity.
- `native-runtime-architecture`: Persist immutable Prompt Hook versions, drafts, and execution outcomes through the tooling context without storing Prompt bodies in metrics.
- `chat-experience`: Attribute each live Prompt Hook execution outcome and duration to the exact published version used for an Agent invocation.

## Impact

- Frontend models and the `AgentService` Prompt Hook boundary gain variable, lifecycle, history, and evaluation operations.
- `tauri-agent-client.ts` maps the new service methods to bounded native commands; `web-agent-client.ts` supplies deterministic equivalent behavior.
- The Rust `tooling::prompt_hooks` domain, application ports/services, SQLite repository, API facade, commands, and bootstrap assembly gain version and evaluation behavior.
- The `agent_runtime` integration reports terminal invocation outcomes to the published Prompt Hook versions included in that invocation.
- SQLite receives additive migrations for draft/version records and safe execution observations. Existing user Hooks are migrated as their first published version.
- The settings page and synchronized Simplified Chinese/English resources gain new controls and summaries.
- No new package, UI framework, state manager, direct React-to-Tauri call, or feature-local log file is introduced.
