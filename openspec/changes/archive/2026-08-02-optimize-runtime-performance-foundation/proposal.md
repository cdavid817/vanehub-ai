## Why

The current desktop release profile enables debug-oriented compilation, settings eagerly mount service-backed pages, historical-session search performs wildcard scans and per-result loading, and retained terminal output repeatedly copies a large string. These costs affect startup, navigation, search latency, and long-running memory/CPU usage, while the existing build checks do not prevent regressions.

## What Changes

- Restore the optimized Rust release profile and add a contract check that prevents debug settings from returning.
- Load settings pages on first visit and retain only visited pages, preserving state without starting every page's data work at settings startup.
- Debounce historical-session search, index persisted message content in SQLite, and return matches without per-result database queries.
- Replace repeatedly truncated terminal transcript strings with bounded chunk buffers in Rust and the React runtime.
- Add deterministic bundle and structural performance gates to the existing validation workflow.
- Keep SQLite as the source of truth and do not add Redis or a general-purpose application cache in this phase.

## Capabilities

### New Capabilities

- `runtime-performance-governance`: Defines measurable release-build, frontend bundle, settings-mount, search-query, and terminal-buffer performance safeguards for desktop and web builds.

### Modified Capabilities

- `settings-center-ui`: All service-backed settings pages become first-visit lazy modules while already visited pages remain mounted to preserve local state.
- `session-management`: Historical-session search gains bounded, debounced requests and SQLite-indexed message matching without per-result loading.
- `agent-terminal-runtime`: Retained terminal transcripts use bounded incremental storage while preserving attach/replay behavior.

## Impact

- Frontend: settings page registry/shell, main-layout session search, terminal replay buffering, bundle validation scripts, and focused tests.
- Native: Rust release profile, SQLite migration and session repository search, terminal process buffering, and contract tests.
- Interfaces: no component-to-Tauri calls and no public service-interface change; desktop and web adapters remain compatible.
- Runtime dependencies: no new external service and no Redis deployment requirement.
