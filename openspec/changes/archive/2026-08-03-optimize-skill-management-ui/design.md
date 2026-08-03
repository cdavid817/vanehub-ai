## Context

The current Skills settings page loads either a global or manually chosen workspace overview. That makes Settings responsible for inventing a “current project” outside an active session, while every Skill card repeats all CLI and API Agent checkboxes. The active-session information panel already knows the canonical worktree/project path and already lists global and project Skills, but it is read-only and groups “available” Skills separately from all project Skills without a direct management workflow.

The revised product boundary is contextual: Settings administers the global Skill library, and Information Panel → Skill administers project Skills for the active session while showing which global Skills also apply. The underlying `global` and `workspace` model remains unchanged because storage, identity, drift, mounting, and API prompt injection still depend on it.

Both surfaces run in the shared React frontend for Tauri desktop and Web/mock modes. Existing Skill overview, preview, mutation, binding, mount-path, drift, and directory-selection methods already cover the required behavior, so the implementation should preserve the React → `AgentService` → runtime adapter boundary.

## Goals / Non-Goals

**Goals:**

- Make Settings a global-only Skill administration surface organized by Agent in the same visual pattern as CLI Parameter Management.
- Remove the scope switcher and manual workspace path from Settings.
- Make the information panel the authoritative UI context for global/project Skill presentation and project Skill management.
- Derive project identity from the active session's worktree or project path.
- Keep CLI mounts, API prompt bindings, enablement, drift, and operation state semantically distinct.
- Preserve keyboard access, localization, responsive behavior, both themes, and Tauri/Web parity.

**Non-Goals:**

- Remove the `global` and `workspace` Skill scopes from storage or service contracts.
- Copy a shared Skill into per-CLI ownership, duplicate Skill documents, or hard-code the four built-in CLI ids in Skill UI logic.
- Add a marketplace, bulk mutations, server-side filtering, or a new state-management/UI library.
- Move filesystem, SQLite, canonicalization, or mount logic into React.

## Decisions

### Decision: Settings manages only the global Skill library

`SkillsPage` will always request and mutate `{ scope: "global", workspacePath: null }`. It will not render a scope selector, workspace-directory picker, or project-context label. Global create, import, edit, delete, restore, enablement, Agent assignment, drift synchronization, and CLI mount-path configuration remain available in Settings.

Rationale: Settings is application-level configuration and has no reliable current project when opened directly. Removing manual project selection eliminates ambiguous or stale workspace state while retaining every global administration capability.

Alternative considered: keep an implicit project mode when navigating from a session. That would make the same Settings page change meaning based on how it was opened and reintroduce a hidden scope state.

### Decision: Reuse the CLI Parameter Management two-column Agent pattern

The global Settings page will use a responsive two-column layout with a compact left navigation and right inventory:

- `All Skills` shows the complete global library and Skill lifecycle actions.
- Dynamic CLI Agent entries show `Assigned` and `Available` global Skills for that stable Agent id.
- Dynamic API Agent entries use the same pattern but label bindings as prompt injection rather than mounts.
- `Unassigned` shows global Skills with neither CLI nor API bindings.

The selected Agent view performs granular assignment directly in the list. It does not imply that a Skill is owned by that Agent. Known Agents reuse `AgentBrandIcon` and the visual-identity registry; custom compatible Agents render from overview data without agent-specific JSX branches.

Rationale: this matches a familiar settings interaction and makes “configure Codex Skills” a direct workflow. Agent count no longer multiplies controls inside every Skill row.

Alternative considered: keep a Skill-centric grid with a binding dialog. That is better for editing one Skill across many Agents but weaker for the requested CLI-classified management workflow.

### Decision: Information Panel → Skill owns session-context scope presentation

The Skill tab will expose three keep-alive subviews:

- `Effective`: enabled global and project Skills applicable to the active session Agent.
- `Global`: global Skills applicable to the active Agent, presented read-only with a route to global Skill Settings.
- `Project`: the complete workspace Skill inventory for the active session, including disabled, unbound, and drifted Skills.

The workspace key is resolved deterministically as `activeSession.worktreePath ?? activeSession.projectPath`. If neither exists, project operations are unavailable and the panel shows a localized no-project state; it never asks for a manual path.

Rationale: only an active session can accurately answer “current project,” especially when a Git worktree is in use. Showing both scopes beside session details explains the effective runtime without turning Settings into a project browser.

Alternative considered: display scopes in the information panel but send all project operations back to Settings. That would force Settings to acquire a hidden project context and split one project workflow across two surfaces.

### Decision: Project Skills are fully managed from the information panel

The Project subview provides compact inventory actions for create, import, preview, edit, enable/disable, delete, drift synchronization, and binding/unbinding to the active session Agent. Complex forms and destructive confirmations open shared application dialogs rendered outside the narrow panel; the panel itself remains a list and status surface.

For a CLI session, assignment uses the CLI mount binding operation. For an API Agent session, assignment uses the API prompt-binding operation. The UI never shows filesystem mount language for an API Agent. Managing bindings to other Agents remains a global Settings concern; the project panel is intentionally scoped to the active Agent.

Rationale: project Skill changes are contextual and should not require leaving the active session. Application-level dialogs preserve editing space, focus behavior, and reuse without widening the panel.

Alternative considered: make project Skills read-only in the panel. That would remove the only clear product surface for creating and repairing workspace Skills after project mode is removed from Settings.

### Decision: Preserve domain scope and share query caches by canonical context

Settings uses one global `getSkillOverview` query. The information panel uses the same global overview key plus a workspace overview keyed by the canonical session workspace path. Mutations invalidate only the affected global or workspace overview. Effective Skills are derived from the two loaded overviews using stable Skill identity and stable Agent ids.

No new backend operation is planned. Tauri continues to resolve and canonicalize workspace paths and perform SQLite/filesystem operations; Web/mock continues to simulate equivalent behavior. React components call only the existing service interface.

Rationale: shared query keys prevent redundant global fetches and keep Settings and the information panel coherent after a mutation.

Alternative considered: introduce a session-specific aggregate endpoint. It could simplify the component but duplicates existing overview behavior and expands the runtime boundary without necessity.

### Decision: Separate assignment, enablement, and active state

Selected-Agent lists use assignment as their only mutable Skill-state control. Skill enablement remains a Skill-wide state and is mutable only from All Skills because disabling a Skill can affect more than the selected Agent. Selected-Agent rows present global enablement as read-only status and may navigate to All Skills when a paused Skill needs to be enabled. A configured CLI binding on a disabled Skill is labeled paused, while an active mount is claimed only when binding data confirms it. API prompt bindings are never labeled mounted.

The mutations preserve a strict separation of intent. `setSkillEnabled` changes only the Skill-wide `enabled` state and retains all CLI/API assignments; re-enabling restores only previously assigned Agents. CLI/API bind and unbind operations target one stable Agent id and never change global enablement or another Agent's assignment. Runtime mounted, prompt-injected, paused, and conflict states are derived rather than exposed as independent user toggles.

Rationale: a CLI-classified view can otherwise make global enablement look Agent-specific and cause unintended cross-Agent changes. Restricting that control to All Skills makes “globally available” and “assigned to this Agent” visually and behaviorally distinct without adopting unconditional provider cascade semantics.

### Decision: Use progressive configuration and explicit operational states

Global mount paths appear under the selected CLI's advanced disclosure rather than as an all-Agent panel above the inventory. Healthy drift is compact; global issues remain prominent in Settings and project issues remain prominent in the Project information-panel subview. Loading, retryable error, true-empty, filtered-empty, pending, stale-conflict, migration, synchronization, and destructive confirmation states stay attached to their affected surface.

Create/edit/import/preview/delete flows use the shared application-dialog pattern. Markdown editing uses Edit/Preview modes rather than requiring a permanently split editor. Initial implementation uses a normal compact list; virtualization is deferred until measured large-inventory performance demonstrates a need.

## Risks / Trade-offs

- [Risk] Full project management can overcrowd the narrow information panel. → Keep only compact rows, status, and primary actions in the panel; render forms and confirmations in application-level dialogs.
- [Risk] Global and project Skills with the same id can look duplicated in Effective view. → Always display a localized scope badge and preserve scope in the React key and action input.
- [Risk] A session worktree and repository root have separate workspace Skill stores. → Display the resolved path in Project view and consistently prefer `worktreePath` so actions match the running session.
- [Risk] Global enablement in a selected-CLI view may be mistaken for per-CLI enablement. → Do not render a global enablement control in selected-Agent views; show read-only global status and keep the mutable control in All Skills.
- [Risk] Information-panel mutations and Settings mutations can show stale global data. → Reuse canonical React Query keys and invalidate the exact affected overview after every mutation.
- [Risk] Removing project mode from Settings changes an established workflow. → Provide equivalent project lifecycle actions in the information panel and cover direct Settings entry and active-session entry in interaction tests.

## Migration Plan

1. Add shared Skill presentation/filter helpers and synchronized localization for global Settings and information-panel project management.
2. Refactor Settings to a global-only overview and the dynamic Agent navigation/inventory layout.
3. Move selected-CLI mount configuration and global drift presentation into the new global layout.
4. Split the information panel Skill pane into focused children and add Effective, Global, and Project subviews using the active session path.
5. Add reusable project Skill dialogs and active-Agent binding operations through the existing service boundary.
6. Add query-cache coherence, responsive, accessibility, theme, Tauri/Web parity, and regression coverage; run all required validations.

Rollback is frontend-focused: restore the Settings scope selector and the read-only information-panel groups. No Skill documents, bindings, mount paths, or database rows need migration or rollback.

## Open Questions

None. The confirmed product boundary is global administration in Settings and session-context project administration in Information Panel → Skill.
