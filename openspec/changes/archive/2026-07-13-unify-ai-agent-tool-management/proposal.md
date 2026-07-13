## Why

Developers increasingly use multiple AI coding agents across daily workflows, but switching between Claude Code, OpenCode, Codex CLI, Gemini CLI, and similar tools creates fragmented configuration, context, and execution surfaces. This change introduces a unified management layer so developers can select, launch, and operate agents consistently across browser-based and native desktop interaction modes.

## What Changes

- Add a unified tool registry for supported AI coding agents, including Claude Code, OpenCode, Codex CLI, and Gemini CLI.
- Add agent switching so users can choose the active agent for a task without manually changing tools or environments.
- Support two interaction modes:
  - Web browser mode for browser-based sessions and web UI workflows.
  - Native desktop window mode for local desktop app workflows.
- Provide consistent metadata for agent capabilities, launch method, availability, and interaction mode support.
- Establish a foundation for future per-agent configuration, credentials, routing, and workflow automation.

## Capabilities

### New Capabilities

- `agent-tool-registry`: Defines how AI coding agents are registered, described, discovered, and made available for selection.
- `agent-switching`: Defines how users select and switch between available AI coding agents for development workflows.
- `interaction-modes`: Defines support for web browser and native desktop window interaction modes.

### Modified Capabilities

None.

## Impact

- Adds new OpenSpec capabilities for agent registry, switching, and interaction modes.
- Affects future application architecture around tool discovery, agent launch flows, session routing, and UI state.
- May introduce dependencies on browser automation, desktop window management, or per-agent CLI/app integration in implementation.
