## Why

The Skills settings page mixes global and manually selected workspace contexts with repeated per-Agent controls, making it unclear which project is being changed and slow to manage a CLI's effective global capabilities. Global Skill administration belongs in Settings, while project Skill administration should derive its workspace from the active session and live beside the session information it affects.

## What Changes

- Make Settings → Skill Management a global-only administration surface with no scope switcher or manual workspace-directory field.
- Organize global Skills through a CLI Parameter Management-style two-column layout: dynamic Agent navigation on the left and assigned/available global Skills for the selected Agent on the right, plus All Skills and Unassigned views.
- Keep global Skill enablement exclusively in All Skills; selected CLI/API Agent views manage only the selected Agent's assignment and present Skill-wide enablement as read-only status.
- Preserve assignment intent across Skill-wide pause/resume: changing global enablement must not add or remove Agent bindings, and changing one Agent binding must not affect another Agent.
- Keep global Skill creation, import, edit, deletion, enablement, CLI/API binding, global drift synchronization, and CLI mount-path configuration in Settings.
- Place global/project scope presentation in the active session's Information Panel → Skill tab, using the session worktree path first and project path second instead of asking the user to choose a workspace.
- Show Effective, Global, and Project Skill views in the information panel; keep Global read-only there with navigation to global Settings, and provide full project Skill management through application dialogs tied to the active session Agent and workspace.
- Distinguish configured bindings from currently active CLI mounts when a Skill is disabled, and keep CLI mount binding separate from API prompt binding.
- Use compact lists, progressive advanced settings, actionable drift presentation, and explicit loading, error, empty, pending, confirmation, preview, and edit states across both surfaces.
- Keep the shared React behavior equivalent in the Tauri desktop and Web/mock runtimes without adding direct Tauri calls or changing Skill storage and persistence semantics.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `settings-skill-management-ui`: Make Settings a global-only, CLI-classified Skill administration surface with focused assignment, inventory, mount, drift, dialog, and feedback behavior.
- `main-layout-ui`: Expand the active session information panel's Skill tab into contextual Effective, Global, and Project views, with project Skill management derived from the session workspace.

## Impact

- Affects the shared React settings UI and active-session information panel in both the Tauri desktop runtime and Web/mock runtime.
- Primarily affects `src/settings/pages/skills-page.tsx`, its `src/settings/pages/skills/` children, `src/main-layout/session-info-panel.tsx`, new focused information-panel Skill children, localization resources, and frontend interaction/rendering tests.
- Reuses existing `AgentService` global/workspace overview and granular mutation methods; no direct runtime integration, SQLite schema, Rust command, or frontend/backend boundary change is expected.
- Preserves the underlying `global` and `workspace` Skill model: the change removes manual scope selection from Settings rather than removing scope isolation from the domain.
- Tightens existing frontend semantics without introducing a new persistence field or changing the service boundary: `enabled` remains Skill-wide, while bind/unbind operations remain Agent-specific.
- May reuse existing application-dialog, Agent visual identity, and Markdown rendering facilities; no alternative state manager, styling system, or UI library is introduced.
