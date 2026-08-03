## Context

The settings center originally separated Agent Management from Agent Configuration. A prior implementation removed the first navigation entry but moved its registration, registry, lifecycle, and runtime controls into Agent Configuration. The required product boundary is stricter: those management workflows must be removed from settings altogether, while OnePiece and CLI provider configuration remain available.

The underlying registry and runtime services are also used by session creation, execution, and native lifecycle handling. Removing their settings presentation does not authorize deleting those shared backend capabilities. Archived OpenSpec changes remain immutable project history.

## Goals / Non-Goals

**Goals:**

- Remove the `agents` settings page id, navigation item, lazy loader, standalone page component, and page-only localized copy.
- Delete settings UI for API Agent registration, registered-Agent lists/status, edit/delete, runtime selection/modes, workflow launch/details, tool trust, and memories.
- Keep Agent Configuration limited to OnePiece and CLI provider configuration, including supported configuration deep links.
- Remove tests and active design statements that expect the deleted workflows or relocate them to Agent Configuration.

**Non-Goals:**

- Deleting the Agent registry, runtime/session services, stable Agent ids, native commands, SQLite records, or adapter contracts used outside settings.
- Removing OnePiece provider configuration, CLI profile management, or Agent selection inside session-creation workflows.
- Rewriting archived OpenSpec artifacts.

## Decisions

### 1. Remove the management destination without replacement

The `agents` page id and navigation registration are removed. The `agent-configurations` page remains, but it represents provider configuration only and is not a renamed or consolidated management page.

### 2. Delete management-only UI modules instead of extracting them

Components that exist solely for registration, registry/runtime cards, editing/deleting registered Agents, workflow control, tool trust, or memory management are deleted. They are not rendered in another settings page and are not retained as dormant UI.

### 3. Keep Agent Configuration focused on OnePiece and CLI providers

Agent Configuration continues to present its OnePiece and CLI tabs, provider credentials, status, saved profiles, and configuration dialogs. It must not display registered-Agent inventory or runtime controls. Configuration-tab selection remains independent from session/runtime Agent selection.

### 4. Preserve shared service and backend capabilities

`AgentService`, desktop/Web adapters, Rust commands, SQLite storage, and runtime/session behavior remain because non-settings consumers rely on them. This avoids coupling deletion of a presentation surface to an unrelated runtime migration.

## Risks / Trade-offs

- [Risk] Users can no longer register or edit arbitrary API Agents through settings. → This is an intentional product removal; no replacement settings workflow is introduced.
- [Risk] Registry/runtime APIs remain without a settings management surface. → Retain them only for session and execution consumers, and prevent new settings dependencies through tests.
- [Risk] Removing shared-looking components could break session creation. → Delete only modules whose imports are confined to the removed settings management surface, then run repository-wide tests and reference searches.
- [Risk] Historical documents still describe Agent Management. → Preserve archived records by policy; current main specs and this change define the active behavior.

## Migration Plan

1. Revise this change’s deltas from consolidation to full settings-surface removal.
2. Remove the runtime/registry section from Agent Configuration and delete management-only components.
3. Remove unused localization and update component/E2E tests to assert that no management controls appear in Agent Configuration.
4. Search active code and non-archived artifacts for stale relocation assumptions.
5. Run frontend, Rust, and strict OpenSpec validation. No persisted-data migration is required.

## Open Questions

None.
