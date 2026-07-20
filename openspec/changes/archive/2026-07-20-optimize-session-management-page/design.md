## Context

The current workspace session sidebar is a compact navigation surface with activity, folder, category, archived, pinned, search, and single-session context actions. It already depends on `AgentService` and does not call Tauri APIs directly. The requested optimization is a UI-heavy workflow improvement inspired by cc-switch's session manager: batch selection, list/category presentation, Agent filtering, and careful confirmation for destructive operations.

## Goals / Non-Goals

**Goals:**

- Let users enter a batch-management mode, select multiple visible sessions, and delete them after confirmation.
- Let users switch between a flat list presentation and a categorized presentation.
- Let users filter sessions by All, Claude Code, OpenCode, Codex CLI, and Gemini CLI.
- Preserve existing sidebar capabilities: search, pinned sessions, archived sessions, category assignment, right-click single-session actions outside batch mode, and active-session navigation.
- Keep desktop and Web behavior aligned through the existing service boundary.
- Keep zh-CN and en translations synchronized.

**Non-Goals:**

- Do not add a new database schema or Rust-owned bulk-delete command for the first implementation.
- Do not copy cc-switch's component stack, virtualization, or styling system into VaneHub.
- Do not change session creation, Agent Terminal lifecycle, or session message persistence semantics except as needed after deletion refresh.

## Decisions

1. Batch deletion will be UI-orchestrated over the existing `AgentService.deleteSession(sessionId)` operation.

   Rationale: the current native and Web adapters already implement single-session deletion with correct active-session clearing and message ownership cleanup. A UI-level `Promise.all`/sequential mutation keeps the first implementation small and avoids introducing an adapter and Tauri command solely for a batch wrapper.

   Alternative considered: add `deleteSessions(sessionIds)` to `AgentService` and Rust. This is more efficient for large selections but expands the service contract before there is evidence that transaction-level partial-failure semantics are required.

2. Batch mode will be explicit and modal within the sidebar.

   In batch mode, session cards show checkbox controls and clicks toggle selection instead of switching active sessions. Context menu and drag-to-category behavior are disabled in this mode to avoid mixed editing models.

3. The presentation switch will be separate from Agent filtering.

   Agent filtering controls the candidate session set; list/category presentation controls how that set is displayed. Search continues to narrow the visible candidate set.

4. Categorized presentation will reuse the existing user-defined category groups.

   The current sidebar already has category groups and drag assignment. The optimization should make category/list a clearer presentation choice, while preserving the existing activity and archive concepts where they remain useful.

5. New controls will use existing VaneHub UI primitives and semantic tokens.

   Use Tailwind classes, `Button`, lucide icons, and existing `ucd-*` utility classes. Avoid introducing a new UI library or cc-switch-specific classes.

## Risks / Trade-offs

- UI-orchestrated batch deletion can partially succeed if one delete fails -> Show a concise localized failure notification and refresh session queries so the UI reflects persisted state.
- Selection across filtered views can surprise users -> Keep selected ids bounded to currently visible sessions when filters/search change during batch mode, and expose selected count clearly.
- Sidebar width is narrow -> Use compact icon+text controls and stable heights; keep destructive actions in a confirmation dialog rather than adding large inline controls.
- Existing category drag interactions conflict with checkbox selection -> Disable drag and context menu while batch mode is active.
