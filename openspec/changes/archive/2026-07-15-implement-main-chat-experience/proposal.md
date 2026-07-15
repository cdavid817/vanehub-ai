## Why

VaneHub AI already manages agents, sessions, MCP services, and runtime settings, but the main workspace still stops short of a usable chat loop. The center panel needs to evolve from a management surface into a working conversation surface where users can send prompts, see streamed agent responses, and reload session history.

## What Changes

- Add a chat experience for the main workspace that combines the completed chat input controls with a message list, empty state, streaming assistant responses, stop-generation behavior, and persisted history.
- Extend the frontend agent service boundary with message operations and stream subscriptions so React components do not call Tauri APIs directly.
- Provide Web/mock behavior for message sending and streaming so the browser runtime remains usable without SQLite or local CLI access.
- Add desktop runtime support for storing messages in SQLite, exposing Tauri commands for message operations, and routing messages to Agent CLI runtimes through adapter boundaries.
- Introduce a first backend Agent runtime/parser path for Claude Code, with a generic line-based fallback for other agents.
- Keep prompt enhancement out of scope; the Phase 1 enhance button remains a disabled placeholder for a future change.

## Capabilities

### New Capabilities

- `chat-experience`: Defines chat input submission, valid chat configuration, message listing, streaming assistant responses, stop-generation behavior, and message persistence for both desktop and Web runtimes.

### Modified Capabilities

- `main-layout-ui`: The existing chat-first main content area gains a real message list and composer-backed interaction behavior instead of static transcript/input placeholders.
- `session-management`: Session records gain associated persisted chat messages that are deleted with their owning session.

## Impact

- Affects shared React UI under `src/components/chat/` and `src/main-layout/`.
- Extends `src/types/chat.ts` and the frontend service contract in `src/services/agent-service.ts`.
- Requires matching implementations in `src/services/web-agent-client.ts` and `src/services/tauri-agent-client.ts`.
- Affects the Tauri desktop runtime under `src-tauri/` with SQLite migration, message repository logic, Tauri commands, runtime process management, and stream event emission.
- Affects both desktop and Web runtimes; desktop uses SQLite and Agent CLI processes, while Web uses mock message storage and mock streaming.
- Preserves frontend/backend isolation: React components depend on service interfaces, Tauri `invoke()` remains inside the Tauri frontend adapter, and Agent-specific process behavior remains behind runtime/parser adapters.
