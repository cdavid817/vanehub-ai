## Overview

The CLI management page uses a cached-read plus asynchronous-refresh model:

```text
CLI 管理 page
  ├─ initial load: read last persisted CLI status
  ├─ first startup with no persisted status: start one async refresh
  ├─ refresh: start async detection operation
  └─ install / upgrade / downgrade: start async npm operation
```

This keeps the settings center responsive. External commands and npm network calls must not run during initial render or in a blocking Tauri command path.

## CLI Catalog

The supported CLI catalog is backend-owned and ordered:

| Agent id | Display name | Executable | npm package |
| --- | --- | --- | --- |
| `claude-code` | Anthropic Claude Code CLI | `claude` | `@anthropic-ai/claude-code` |
| `codex-cli` | OpenAI Codex CLI | `codex` | `@openai/codex` |
| `gemini-cli` | Google Gemini CLI | `gemini` | `@google/gemini-cli` |
| `opencode` | OpenCode CLI | `opencode` | `opencode-ai` |

Frontend UI should render by stable ids and service-provided order, not by matching display text.

## Runtime Boundary

```text
React settings page
  -> frontend CLI/Agent service interface
    -> runtime adapter
      -> Tauri adapter invokes Rust commands
      -> Web adapter reports native CLI detection as unsupported
    -> Rust/Tauri layer
      -> SQLite last-known status
      -> async operation registry
      -> guarded process execution
```

React components must not import Tauri APIs. Tauri `invoke()` calls belong only in the Tauri-specific frontend adapter.

## Data Model

The implementation should add a service-facing CLI status shape similar to:

```ts
type CliToolStatus = {
  agentId: string;
  displayName: string;
  provider: string;
  executableName: string;
  packageName: string;
  installed: boolean | null;
  currentVersion: string | null;
  latestVersion: string | null;
  availableVersions: string[];
  detectedPath: string | null;
  installCommand: string;
  lastCheckedAt: string | null;
  lastError: string | null;
  lastOperationId: string | null;
};
```

`installed: null` represents never detected or unsupported detection. The page should display that as not yet detected rather than installed or missing.

## Initial Read

Opening `CLI 管理` calls a read-only service method that returns the last persisted status. It must not execute `where`, `command -v`, CLI version commands, `npm view`, or `npm install`.

If no status exists on first startup, the app starts one asynchronous CLI detection refresh after the cached read returns. The initial render still shows the fixed four CLI cards with an undetected state while the background operation runs. The automatic refresh must use the same backend-managed operation path as the manual refresh action and must not block startup or rendering.

If persisted status exists, the page renders the cached result immediately and does not need to run an automatic refresh; the user can still trigger manual refresh.

## Refresh Detection

Clicking `刷新检测` starts a backend operation and returns an operation id immediately. The operation checks each CLI independently:

- Resolve executable path using `where` on Windows and `command -v` on macOS/Linux.
- Read the current local version using backend-owned version arguments.
- Query npm for the latest version.
- Query npm for the available versions list.
- Filter to the most recent 20 stable versions by default, excluding prerelease versions.
- Persist successful partial results and per-CLI errors without failing the whole refresh when one CLI fails.

## Install, Upgrade, And Downgrade

Install, upgrade, and downgrade use the same backend operation:

```text
npm install -g <package>@<targetVersion>
```

The frontend passes only `agentId` and `targetVersion`. The backend resolves the npm package from the fixed catalog and constructs the process invocation with explicit executable and argument values. The frontend must not send arbitrary command strings.

Button labeling is derived from installed state and selected version:

- Missing CLI plus selected version: install.
- Installed CLI plus newer selected version: upgrade.
- Installed CLI plus older selected version: downgrade.
- Installed CLI plus matching selected version: current version, disabled.

After a successful npm operation, the backend refreshes detection for that CLI and persists the updated status.

## Operations And Logs

All refresh and npm package operations run through the native operation registry. Each CLI card shows its most recent operation state and can expand to show logs. A running operation disables only controls for the affected operation or CLI; it must not block the settings shell or unrelated cards.

## Web Runtime

The Web adapter must not fake local CLI installation. It should return the fixed CLI catalog with undetected or unsupported status and user-displayable messaging that native CLI detection is available only in the desktop runtime unless a future HTTP backend provides equivalent data.
