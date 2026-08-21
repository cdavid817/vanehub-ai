## Why

The Prompt Hooks settings page exposes management, authoring, publication, version evaluation, and runtime diagnostics at the same visual level, making routine enablement and binding work unnecessarily dense. The page should preserve its advanced capabilities while using progressive disclosure so users can scan Hooks, understand live state, and reach the correct workflow without deciphering overlapping controls.

## What Changes

- Replace the large statistics-and-card inventory with a compact summary and category-grouped Hook list optimized for scanning.
- Keep search, enabled state, and CLI binding as primary filters while moving lower-frequency criteria into an explicit additional-filters surface.
- Open a selected Hook in one detail surface that owns basic settings, CLI bindings, template drafting and publication, and version history.
- Unify the current edit and advanced entry points so user-created Hooks have one understandable draft-to-publish workflow.
- Move assembled-prompt preview and safe Hook trace summaries into a dedicated runtime-records view instead of displaying diagnostics below the management inventory.
- Keep built-in governance restrictions, explicit content preview, responsive behavior, accessibility, localization, and large-inventory windowing intact.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `settings-prompt-hooks-ui`: Revise the Prompt Hooks settings information architecture, list presentation, progressive filtering, unified detail workflow, and separation of management from runtime diagnostics.

## Impact

- Affects the Prompt Hooks React settings experience in both desktop and Web/mock runtimes.
- Primarily changes `src/settings/pages/prompt-hooks-page.tsx` and components under `src/settings/pages/prompt-hooks/`, with synchronized locale resources and UI tests.
- Preserves the existing `AgentService` Prompt Hook operations and the Tauri/Web adapter boundary; no new direct native calls, database changes, backend commands, or dependencies are expected.
- Requires Playwright coverage because navigation, dialogs or drawers, filtering, editing, and diagnostic access are user-visible behavior changes.
