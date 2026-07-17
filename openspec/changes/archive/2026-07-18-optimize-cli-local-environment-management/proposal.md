## Why

VaneHub AI can currently detect and manage four supported CLIs, but it records only the first executable found and always performs npm-based package operations. On machines with desktop, WinGet, npm, or other parallel installations, this can update a different installation from the one the user actually runs and cannot distinguish missing tools from installed-but-broken tools.

## What Changes

- Extend local CLI detection for Claude Code, Codex CLI, Gemini CLI, and OpenCode to report the active PATH entry, all discovered installations, installation source, environment type, runnable state, version, and conflict state.
- Keep cached initial rendering and add single-tool refresh alongside the existing all-tool asynchronous refresh.
- Add backend-owned lifecycle planning that selects a safe source-aware update target, requires confirmation for multiple installations, and never trusts frontend-supplied command text.
- Serialize global CLI package mutations while keeping navigation, detection, and unrelated read-only UI responsive.
- Redesign the CLI Management page around compact local-environment cards with diagnostics, source/environment badges, current/latest versions, conflicts, manual commands, operation progress, and localized error states.
- Refine the About page using the same compact information hierarchy and expose a localized CLI environment summary that links users to CLI Management without duplicating lifecycle controls.
- Preserve honest Web/mock behavior, semantic styling for both `futuristic` and `minimal`, synchronized zh-CN/en resources, and unified operation logging.
- Add a maintained technical note that separates the first-version implementation from known limitations and future optimization paths such as WSL, broader package-manager support, richer repair guidance, and incremental probing.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `agent-tool-registry`: Expand persisted CLI status from one resolved path to source-aware installation distribution, active-entry, runnable, and conflict semantics.
- `settings-center-ui`: Add compact local-environment diagnostics, conflict confirmation, single-tool refresh, dual-style rendering, localized states, and About-page environment summary behavior.
- `native-runtime-architecture`: Require safe asynchronous installation enumeration, source-aware lifecycle planning, backend command ownership, and serialized package mutations.
- `frontend-runtime-architecture`: Extend Agent service and desktop/Web adapter parity for detailed CLI environment status and targeted refresh behavior.

## Impact

- Frontend contracts, Agent service interface, Tauri and Web adapters, CLI Management UI, About UI, locale resources, and focused frontend tests.
- Rust CLI detection, command planning, task execution, SQLite status persistence, unified diagnostic logging, and Rust tests.
- OpenSpec delta specs and a technical implementation/optimization note under this change.
- Both desktop and Web runtimes are affected; native inspection and mutation remain desktop-only, while the Web adapter exposes the same contract with explicit unsupported local state.
- No new runtime dependencies or new supported CLI products are introduced in the first version.
