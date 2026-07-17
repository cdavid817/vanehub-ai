## Context

CLI management is rendered in the settings/providers surface and calls `AgentService` methods through runtime adapters. The Tauri adapter invokes Rust commands for listing CLI statuses, starting detection refresh, and starting selected-version npm operations. The Rust runtime already has a single four-CLI catalog for Claude Code, Codex CLI, Gemini CLI, and OpenCode, plus an operation registry and unified logging service.

The reported failures show two gaps. First, refresh and npm failure paths can produce concise UI errors without enough durable context in the active log directory. Second, selected-version package operations must behave identically for all four CLI definitions instead of becoming install-only or failing to install non-latest selected versions.

## Goals / Non-Goals

**Goals:**

- Make CLI refresh a visibly running operation in the SDK/CLI settings UI until the backend operation settles.
- Persist detailed diagnostics for CLI refresh, CLI detection, npm registry lookup, CLI package install, upgrade, and downgrade failures through unified logging.
- Keep Claude Code, OpenCode, Codex CLI, and Gemini CLI on one backend-owned package-operation path with selected target versions.
- Keep frontend components behind `AgentService` and existing runtime adapters.
- Preserve Web/mock behavior with simulated refresh/package operations and no local file writes.

**Non-Goals:**

- Replacing npm with another package manager.
- Adding new managed CLIs or changing package names unless a package mapping is proven incorrect during implementation.
- Building a streaming CLI installer UI beyond the existing operation log polling model.
- Changing SDK dependency installation storage under `~/.vanehub/dependencies/`.

## Decisions

1. Treat CLI refresh and package operations as backend-managed asynchronous operations.

   Tauri commands should return an operation id quickly, and all slow work should remain in background tasks. This directly addresses command-boundary timeout errors such as `launch failed: command timed out`. The alternative is increasing frontend or command timeouts, but that keeps the UI tied to external command latency and does not solve registry or network stalls.

2. Centralize CLI npm operation metadata in the existing Rust CLI catalog.

   The operation should resolve package name, executable name, and supported agent id from `CLI_TOOL_DEFINITIONS`, validate the requested stable semver target, and run explicit npm arguments equivalent to `npm install -g <package>@<targetVersion>`. This avoids per-CLI frontend branches and keeps command construction backend-owned.

3. Log every failure-producing edge with structured context through unified logging.

   Detection refresh should log operation start, each CLI detection start/end, executable resolution failures, version command failures, npm view failures/timeouts, partial completion, and final failure. Package operations should log npm executable, package name, target version, operation id, stdout, stderr, exit code/status, timeout, and sanitized environment context. User-facing errors remain short, while persisted logs carry diagnostics.

4. Preserve operation log display and add durable logs.

   Existing operation logs shown in the CLI card should continue coming through the operation registry/service boundary. The same important lines and failures should also be written to unified logs. The alternative of only writing files would regress in-page troubleshooting.

5. Represent refresh progress from operation status in the frontend.

   The refresh button should disable and show the existing `RefreshCw` icon in a spinning/loading state while the refresh mutation is pending or the returned refresh operation is queued/running. This handles the short command-start phase and the longer backend refresh phase. Web/mock should use the same service shape and return a mock operation.

## Risks / Trade-offs

- **Risk:** npm registry or global install commands can hang or be slow on restricted networks. -> **Mitigation:** keep bounded timeouts in native command helpers, record timeout values and stderr/stdout, and complete the backend operation as failed rather than blocking the Tauri command.
- **Risk:** Logging too much command output could persist sensitive values. -> **Mitigation:** route all durable entries through unified logging redaction and avoid logging raw full environment variables.
- **Risk:** Some CLI packages may expose versions or executable names differently after install. -> **Mitigation:** validate all four catalog entries in tests and refresh detection after successful package operations, preserving the target version in operation result/logs when executable version probing fails.
- **Risk:** Frontend loading state can get stuck if operation polling fails. -> **Mitigation:** clear active refresh/package operation state on terminal operation statuses and show service errors without dropping already loaded CLI status data.
