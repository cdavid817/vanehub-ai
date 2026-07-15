## Why

The current app can only represent one active workflow state and the sidebar still relies on static mock conversations, so users cannot create, switch, organize, or recover multiple agent sessions. VaneHub needs persistent session management now so the desktop runtime and Web adapter can expose the same core workflow model before deeper agent execution features depend on it.

## What Changes

- Add persistent session entities with stable ids, titles, agent ids, interaction modes, lifecycle state, folder, pinned, archived, created-at, and updated-at metadata.
- Add service-level session operations for listing active and archived sessions, creating sessions, switching the active session, renaming, pinning, unpinning, archiving, unarchiving, and deleting sessions.
- Add Tauri desktop runtime commands backed by SQLite for session persistence and active-session tracking.
- Add Web/mock adapter behavior that mirrors the desktop session contract without requiring SQLite.
- Update the main layout sidebar so the "new" action creates a session, session cards are selectable, pinned and archived sessions are visible in the expected sections, and context actions support rename, pin, archive, restore, and delete.
- Preserve frontend/backend isolation by keeping React components dependent on `AgentService` and routing runtime-specific persistence through the Tauri and Web adapters.

## Capabilities

### New Capabilities

- `session-management`: Defines the session entity, lifecycle metadata, persistence expectations, active-session selection, and session organization actions.

### Modified Capabilities

- `main-layout-ui`: Changes sidebar requirements from static mock conversation display to interactive session creation, selection, grouping, pinning, archiving, renaming, and deletion behavior.

## Impact

- Affects both desktop runtime and Web runtime.
- Updates `src/types/agent.ts`, `src/services/agent-service.ts`, `src/services/tauri-agent-client.ts`, and `src/services/web-agent-client.ts`.
- Updates `src/main-layout/main-layout.tsx` and related sidebar UI behavior.
- Updates `src-tauri/` with SQLite migration logic, session data mapping, and Tauri commands.
- Adds Rust dependencies for stable session ids and timestamp handling if they are not already available.
- Requires OpenSpec validation, frontend build/type checks, and Rust checks to verify the shared service contract remains coherent across runtimes.
