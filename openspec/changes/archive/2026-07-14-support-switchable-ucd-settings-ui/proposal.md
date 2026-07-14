## Why

The current frontend is a compact Agent Switcher, while the provided UCD assets define a broader settings-center experience with dedicated pages for basic configuration, providers, SDK dependencies, MCP servers, agents, and skills. The app needs that settings-center structure and must support switching between the provided futuristic and minimal visual styles without hard-coding style branches into each feature component.

## What Changes

- Replace the current single-page frontend shell with a UCD-aligned settings center using top navigation, settings sidebar, page content, and detail/action areas.
- Add a switchable visual style system for `futuristic` and `minimal` themes based on shared semantic design tokens.
- Persist the selected UI style locally so both the browser Web UI and Tauri desktop UI reopen with the last selected style.
- Keep the Agents page connected to the existing `AgentService` boundary for listing, filtering, selecting, and launching agents.
- Add UCD-aligned settings pages for basic configuration, providers, SDK dependencies, MCP servers, and skills using local frontend data where no service boundary exists yet.
- Structure layout, page definitions, and theme registration so future settings pages and visual styles can be added without rewriting existing page components.

## Capabilities

### New Capabilities

- `settings-center-ui`: Defines the settings-center navigation, page composition, theme switching, and UCD-aligned frontend behavior.

### Modified Capabilities

- `agent-switching`: Updates the agent selection user interface to live inside the UCD-aligned Agents settings page while preserving stable agent ids, interaction mode selection, availability checks, and launch behavior.

## Impact

- Affects both the Tauri desktop frontend and browser Web runtime because both render the same React application.
- Does not require Rust command, SQLite schema, process launch, or Tauri native runtime changes.
- Preserves the existing frontend/backend isolation: React components continue to call `AgentService` rather than direct Tauri `invoke()` calls.
- Adds frontend-only theme registration, theme persistence, reusable settings shell components, and UCD page components under `src/`.
- Uses existing dependencies where possible: React, TypeScript, Tailwind CSS, shadcn-style components, and `lucide-react`.
