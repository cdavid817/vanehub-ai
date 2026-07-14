## Context

The current React frontend is implemented primarily in `src/App.tsx` as a single Agent Switcher layout. It already uses the correct runtime boundary: UI components call `AgentService`, `runtime-agent-client.ts` selects the Tauri or Web implementation, and only `tauri-agent-client.ts` calls Tauri `invoke()`.

The UCD assets under `ucd/futuristic` and `ucd/minmal` describe the same settings-center information architecture in two visual styles. The structure includes top navigation, settings sidebar navigation, page-specific search/actions, content panels, tables/cards, and detail areas. The visual difference is primarily theme tokens: futuristic is dark, blue-accented, and panel-heavy; minimal is white, line-based, and low-saturation.

This change affects both the Tauri desktop frontend and browser Web runtime because both render the same React application. It does not require Rust command, SQLite, CLI detection, process launch, or native window management changes.

## Goals / Non-Goals

**Goals:**

- Introduce a UCD-aligned settings-center shell that replaces the current compact Agent Switcher UI.
- Support `futuristic` and `minimal` visual styles through a theme registry and semantic CSS tokens.
- Persist the selected visual style locally in the frontend.
- Keep the Agents page connected to the existing `AgentService` in both Tauri and Web runtimes.
- Provide UCD-aligned pages for basic settings, providers, SDK dependencies, MCP servers, agents, and skills.
- Make future themes and settings pages additive through registration/configuration rather than rewriting existing components.

**Non-Goals:**

- No Rust/Tauri command changes.
- No SQLite schema migration.
- No new external UI dependency.
- No real provider, SDK, MCP, or skill persistence service in this change.
- No browser automation or Playwright launch behavior changes.
- No change to agent ids, interaction mode semantics, availability checks, or launch routing.

## Decisions

### Use a Theme Registry and Semantic Tokens

The theme system will define registered theme ids such as `futuristic` and `minimal`, expose them through a provider/hook, and apply the selected style with a root `data-theme` attribute. CSS will define semantic tokens for app background, panels, borders, foreground text, muted text, accent, status colors, focus rings, and interactive states.

Rationale: Components should consume semantic tokens, not branch on concrete theme names. This keeps future visual styles additive.

Alternative considered: Conditional Tailwind class maps inside every component. This was rejected because it duplicates layout code and makes each new theme a cross-cutting refactor.

### Keep Layout and Page Definitions Separate from Theme

The settings center will be composed from reusable components:

- `SettingsShell`
- `SettingsTopBar`
- `SettingsSidebar`
- page route/selection state
- page components for each UCD section

Page metadata such as id, label, icon, badge count, search placeholder, and component will live in a page registry.

Rationale: UCD structure is shared by both styles; only presentation tokens differ. Separating page registration from visual tokens makes adding a new settings section low-risk.

Alternative considered: Implement two complete copies of the settings center, one per style. This was rejected because it would double feature work and increase behavioral drift.

### Preserve the Existing Runtime Boundary

Agent data and actions will continue to flow through `AgentService`:

```text
React settings UI
  -> AgentService interface
  -> Tauri adapter or Web adapter
  -> Rust/native layer only for Tauri runtime
```

The Agents page may reorganize the UI into UCD cards, filters, status badges, and a details panel, but it must continue using stable `agent.id` values when selecting and launching agents.

Rationale: The existing boundary already supports both desktop and Web runtimes. UI redesign does not justify coupling React components to Tauri commands.

Alternative considered: Let the new Agents page call Tauri commands directly to simplify wiring. This was rejected because it violates the project's adapter constraints and would break browser Web preview.

### Use Local Frontend Data for Pages Without Services

Basic settings, providers, SDK dependencies, MCP servers, and skills will initially use local frontend data shaped to match the UCD assets. These views can expose controls and actions visually, but they will not claim durable backend persistence unless a service exists.

Rationale: The UCD asks for a full settings-center surface, while the current backend only implements agent registry/workflow behavior. Local data lets the frontend structure land without inventing incomplete native APIs.

Alternative considered: Build new Tauri/Rust services for every settings page in this change. This was rejected as too broad and unrelated to the requested frontend design and style switching.

### Theme Persistence Is Frontend-Local

The selected theme will be stored in browser-compatible local storage under a stable key such as `vanehub.uiStyle`. On startup, the provider will validate the stored value against the theme registry and fall back to a default theme.

Rationale: This works in both Web and Tauri WebView runtimes without requiring backend storage.

Alternative considered: Persist the selected theme in SQLite through a new Tauri command. This was rejected because Web runtime needs the same behavior and durable app settings are not otherwise modeled yet.

## Risks / Trade-offs

- [Risk] UCD pages beyond Agents may look interactive before real services exist -> Mitigation: keep data local and avoid implying backend persistence in code paths; add services later behind explicit frontend service interfaces.
- [Risk] Theme tokens may be too narrow for future styles -> Mitigation: use semantic tokens for surfaces, text, accents, status, and interaction states rather than concrete colors.
- [Risk] Replacing `App.tsx` wholesale can obscure existing agent behavior -> Mitigation: move agent behavior into a dedicated Agents settings page and keep service calls unchanged.
- [Risk] Tailwind utility classes can bypass theme tokens accidentally -> Mitigation: prefer existing CSS variable-backed Tailwind colors and add shared semantic classes for UCD panels, nav items, and controls.
- [Risk] Browser and Tauri runtimes can diverge visually if storage or startup logic differs -> Mitigation: implement theme selection entirely in shared React code.

## Migration Plan

1. Add the theme registry/provider and semantic token CSS.
2. Add settings shell, navigation, top bar, and shared UCD UI primitives.
3. Move existing agent list/selection/launch behavior into the Agents settings page.
4. Add UCD-aligned local-data pages for basic settings, providers, SDK dependencies, MCP servers, and skills.
5. Replace `App.tsx` with the new shell while preserving shared `agentService` usage.
6. Validate with frontend build and targeted tests for theme selection and agent service behavior.

Rollback is straightforward because this is frontend-only: revert the new shell/theme files and restore the previous `App.tsx` layout. No native data migration is introduced.

## Open Questions

- Should the default theme be `futuristic` to match a product-forward dark control center, or `minimal` for lower visual weight?
- Should non-Agent pages be explicitly read-only/demo in the first implementation, or should visual controls update local component state for a more complete prototype?
