## Why

Session chat permission modes and durable Agent policy templates currently drive separate launch paths, so a read-only Agent can start a write-capable CLI chat process while its Agent Terminal remains read-only. The UI also labels session-local execution intent as a permission setting, which obscures the effective safety boundary.

## What Changes

- Introduce one effective session execution-policy resolution contract in which the Agent policy template is the maximum permission and a session can only inherit it or narrow it to planning/read-only behavior.
- **BREAKING** Replace session `permissionMode` / `permission_mode` and the values `default`, `plan`, `agent`, and `auto` with `executionMode` / `execution_mode` and the values `inherit`, `plan`, and `execute`; existing persisted chat-configuration snapshots are reset to `inherit` rather than migrated value-by-value.
- Apply the resolved effective policy to every managed CLI chat and Agent Terminal launch before provider arguments are built, with policy-governed security arguments taking precedence over saved CLI profiles.
- Remove policy-governed approval, sandbox, and execution controls from editable CLI parameter profiles so the Agent policy page remains their single source of authority.
- Rename the session control from permission mode to execution mode and show the Agent policy plus the effective behavior in both desktop and Web/mock surfaces.
- Add matrix regression coverage for every built-in CLI, native OnePiece execution, both launch scopes, and all Agent-policy/session-mode combinations.

## Capabilities

### New Capabilities

- `session-execution-policy`: Defines session execution intent, its composition with durable Agent policy templates, effective behavior reporting, and fail-closed resolution.

### Modified Capabilities

- `session-chat-configuration`: Replaces persisted permission mode with the breaking execution-mode contract and resets existing session snapshots to the new default.
- `cli-agent-permission-launch-flags`: Extends Agent-policy projection from Agent Terminal launches to managed CLI chat launches and defines the effective-policy mapping for both scopes.
- `cli-parameter-management`: Removes policy-governed security settings from editable profiles and gives resolved Agent/session policy final precedence at process launch.

## Impact

- Desktop runtime: Rust session configuration domain/DTO/repository code, database migrations, Agent policy lookup, CLI profile loading, provider invocation mapping, and native OnePiece plan-mode selection.
- Frontend and service boundary: chat configuration types, selectors, Tauri adapter, Web/mock adapter, effective-policy presentation, localization, component tests, and Playwright coverage.
- Persisted session chat-configuration data and request/response contracts change incompatibly; no compatibility parser or value migration is retained.
- Frontend/backend isolation remains intact: React obtains effective-policy information through `AgentService`, while policy resolution and provider argument construction stay in Rust/runtime adapters. Web/mock implements the same contract without launching local processes.
- No new third-party dependency is introduced.
