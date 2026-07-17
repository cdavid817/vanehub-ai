## Why

The SDK settings page currently exposes CLI refresh and package-management failures without enough persisted diagnostics, making errors such as `launch failed: command timed out` and `npm install failed` difficult to reproduce and locate. CLI version selection also appears inconsistent across supported tools, preventing expected install, upgrade, and downgrade flows.

## What Changes

- Persist detailed error diagnostics for SDK page refresh, CLI detection, npm registry lookup, and CLI package operations through unified log management.
- Add frontend refresh-in-progress feedback for the SDK page refresh button so users can tell detection is running.
- Ensure Claude Code, OpenCode, Codex CLI, and Gemini CLI all support selected-version install, upgrade, and downgrade behavior through the same validated backend operation path.
- Improve CLI package operation failure reporting so npm command, target package, target version, stdout, stderr, exit status, timeout, and sanitized environment context are available in persisted logs while keeping user-facing errors concise.
- Review likely failure paths in SDK/CLI native and frontend service code and add structured error logging where missing.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `sdk-dependency-management`: Strengthen SDK page refresh state and CLI package version operation behavior for all managed CLIs.
- `unified-log-management`: Strengthen persistent diagnostic logging coverage for SDK page refresh, CLI detection, and CLI package operation failures.
- `native-runtime-architecture`: Clarify asynchronous CLI refresh/package operations must avoid command timeouts at the Tauri command boundary and persist detailed diagnostics on failure.
- `frontend-runtime-architecture`: Require service-backed SDK page refresh interactions to expose loading/refresh state and report critical async failures through the service boundary.

## Impact

- Desktop runtime: Rust CLI detection, npm version lookup, CLI install/upgrade/downgrade operations, task/operation logging, timeout handling, and unified log writes.
- Web runtime: Mock SDK/CLI service behavior should preserve interface compatibility and simulate refresh loading states without local log files.
- Frontend: SDK settings page state, refresh button disabled/loading display, service error handling, and adapter contracts.
- Service/API boundaries: React components must continue using service interfaces; Tauri-specific work remains in the Tauri adapter and Rust commands.
- Dependencies: No new package manager or UI library; npm remains the only package manager for CLI package operations.
