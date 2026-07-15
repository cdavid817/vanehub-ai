## 1. TypeScript Contract

- [x] 1.1 Add the `Session` type to `src/types/agent.ts` with id, title, agentId, interactionMode, lifecycleState, folder, pinned, archived, createdAt, and updatedAt fields.
- [x] 1.2 Extend `AgentService` in `src/services/agent-service.ts` with list, active-session, create, switch, rename, pin, unpin, archive, unarchive, and delete methods.
- [x] 1.3 Update service call sites or type exports so the new session contract is available without using `any` or bypass comments.

## 2. Desktop Persistence and Commands

- [x] 2.1 Add Rust dependencies needed for UUID generation and timestamp handling.
- [x] 2.2 Add a versioned SQLite migration that creates the `sessions` table.
- [x] 2.3 Extend the `workflow_state` schema with nullable `active_session_id` using an additive migration.
- [x] 2.4 Add a Rust `Session` model with camelCase serialization matching the TypeScript type.
- [x] 2.5 Implement row mapping helpers for loading session records from SQLite.
- [x] 2.6 Implement Tauri commands for `create_session`, `list_sessions`, `list_archived_sessions`, and `get_active_session`.
- [x] 2.7 Implement Tauri commands for `switch_session`, `rename_session`, `pin_session`, `unpin_session`, `archive_session`, `unarchive_session`, and `delete_session`.
- [x] 2.8 Ensure archive and delete operations clear `active_session_id` when they affect the active session.
- [x] 2.9 Register all session commands in the Tauri command handler.

## 3. Runtime Adapters

- [x] 3.1 Implement all session methods in `src/services/tauri-agent-client.ts` using Tauri `invoke()` only inside the adapter.
- [x] 3.2 Implement matching in-memory behavior in `src/services/web-agent-client.ts`.
- [x] 3.3 Keep Web adapter ordering, active-session clearing, default title, and archive/restore behavior aligned with the Tauri contract.
- [x] 3.4 Add or update focused service tests for Web adapter session creation, listing, switching, archiving, restoring, and deleting.

## 4. Sidebar UI

- [x] 4.1 Replace static sidebar conversation data with service-backed session queries.
- [x] 4.2 Wire the sidebar new-session action to create a session and refresh session state.
- [x] 4.3 Add selectable session cards that switch the active session and show selected state.
- [x] 4.4 Render pinned sessions in a dedicated area before normal groups.
- [x] 4.5 Preserve activity grouping and folder grouping using session metadata.
- [x] 4.6 Add an archived sessions view with archived count and restore support.
- [x] 4.7 Add session context actions for rename, pin, unpin, archive, restore, and delete.
- [x] 4.8 Add confirmation for destructive delete and avoid browser-native context menus over the custom menu.
- [x] 4.9 Ensure long session lists continue to scroll inside the sidebar only.

## 5. Verification

- [x] 5.1 Run `openspec validate "add-session-management" --strict`.
- [x] 5.2 Run `npm run test`.
- [x] 5.3 Run `npm run build`.
- [x] 5.4 Run `cargo test --manifest-path src-tauri/Cargo.toml`.
- [x] 5.5 Run `cargo check --manifest-path src-tauri/Cargo.toml`.
- [x] 5.6 Manually verify the sidebar can create, select, rename, pin, archive, restore, and delete sessions in Web/mock mode.
