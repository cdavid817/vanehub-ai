## 1. Project Setup

- [x] 1.1 Initialize the Tauri application with React and TypeScript frontend structure.
- [x] 1.2 Add Tailwind CSS and shadcn/ui setup for the management UI.
- [x] 1.3 Configure Rust Tauri commands for frontend-to-native operations.
- [x] 1.4 Add SQLite local storage setup for registry, preferences, and workflow state.
- [x] 1.5 Add Playwright dependency and configuration for browser interaction mode.

## 2. Registry Model

- [x] 2.1 Define the agent registry data model with stable id, display name, provider, launch metadata, supported interaction modes, availability state, and capability tags.
- [x] 2.2 Create the SQLite schema and repository functions for registry entries and user preferences.
- [x] 2.3 Add initial registry entries for Claude Code, OpenCode, Codex CLI, and Gemini CLI.
- [x] 2.4 Expose Tauri commands for registry lookup by stable id and list retrieval for the React agent selection surface.
- [x] 2.5 Implement capability tag filtering for registered agents.
- [x] 2.6 Add frontend service interfaces so React components do not call Tauri commands directly.
- [x] 2.7 Add runtime adapters for Tauri desktop mode and Web/mock mode.

## 3. Availability Checks

- [x] 3.1 Define availability result states and user-facing unavailable reasons.
- [x] 3.2 Implement Rust availability check hooks for local CLI/native agents without launching interactive sessions.
- [x] 3.3 Implement browser readiness checks for agents that require browser-authenticated workflows.
- [x] 3.4 Surface availability state in the agent selection data returned to the React UI.
- [x] 3.5 Prevent unavailable agents from being selected as the active agent.

## 4. Agent Switching

- [x] 4.1 Add persisted workflow state for active agent id and active interaction mode.
- [x] 4.2 Build the React agent selector UI with availability, provider, capability, and mode indicators.
- [x] 4.3 Implement active agent selection for available registered agents.
- [x] 4.4 Validate that the selected interaction mode is supported by the active agent.
- [x] 4.5 Preserve current workflow intent when switching between compatible agents.
- [x] 4.6 Require mode reselection when the newly selected agent does not support the current interaction mode.

## 5. Interaction Modes

- [x] 5.1 Define browser and native desktop interaction mode identifiers and shared lifecycle states.
- [x] 5.2 Implement Playwright-backed browser mode launch routing for agents that support browser-based workflows.
- [x] 5.3 Report browser authentication readiness before launching workflows that require an authenticated web session.
- [x] 5.4 Implement Tauri/Rust native desktop mode launch routing for agents that support local desktop windows.
- [x] 5.5 Detect unsupported native desktop mode on the current platform and prevent launch through that mode.
- [x] 5.6 Keep agent-specific session details scoped to the agent adapter while exposing common lifecycle state.

## 6. Verification

- [x] 6.1 Add Rust tests for registry listing, stable id lookup, SQLite persistence, and capability filtering.
- [x] 6.2 Add frontend tests for available and unavailable agent selection behavior.
- [x] 6.3 Add tests for compatible and unsupported interaction mode selection.
- [x] 6.4 Add tests for switching agents while preserving workflow intent.
- [x] 6.5 Add tests for browser and native desktop mode lifecycle state reporting.
- [x] 6.6 Run OpenSpec validation for `unify-ai-agent-tool-management`.
