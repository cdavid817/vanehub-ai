## Why

Agent evaluation has broad IPC and fixture coverage, but the desktop WebdriverIO workflow does not give operators a focused, reproducible way to prove an OpenCode evaluation and a credential-gated OnePiece evaluation end to end. The current smoke selection can also hide provider-specific dispatch and UI defects behind a generic multi-Agent run.

## What Changes

- Add focused WebdriverIO coverage for an OpenCode evaluation, including Agent selection, arena lifecycle, persisted results, diagnostics, and rendered details.
- Add an opt-in OnePiece evaluation path that reports `BLOCKED` when no credential is supplied and never records the credential in output or evidence.
- Separate deterministic fixture coverage from live-provider qualification so fixture success cannot be reported as a real-Agent pass.
- Diagnose and fix Agent evaluation defects exposed by these workflows while preserving the shared frontend service boundary and native evaluation runtime.
- Record bounded per-layer evidence and a clear `PASSED`, `FAILED`, `BLOCKED`, or `NOT RUN` result.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-evaluation`: Require provider-specific desktop qualification for OpenCode and OnePiece, with truthful fixture/live provenance and credential-safe blocked outcomes.
- `desktop-runtime-verification`: Require a focused WebdriverIO Agent-evaluation layer with isolated state and bounded evidence.

## Impact

- Desktop runtime and WebdriverIO harness only; Web/mock behavior remains unchanged unless a surfaced parity defect requires a shared service fix.
- Likely affected areas include `tests/desktop/`, `scripts/test-desktop.mjs`, evaluation UI/service adapters, and Rust evaluation dispatch/read models.
- No new dependency or direct React-to-Tauri access is introduced. Provider credentials remain process-scoped test inputs and must not enter repository files, SQLite evidence, screenshots, or unified logs.
