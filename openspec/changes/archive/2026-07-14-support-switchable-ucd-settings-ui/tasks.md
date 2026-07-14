## 1. Theme Infrastructure

- [x] 1.1 Create a frontend theme registry for `futuristic` and `minimal` UCD styles with typed theme ids and labels.
- [x] 1.2 Add a theme provider/hook that applies the selected style through a root `data-theme` attribute.
- [x] 1.3 Persist and restore the selected style from frontend-local storage with validation against the theme registry.
- [x] 1.4 Extend shared CSS tokens for UCD surfaces, borders, text, accent, status, focus, and control states.

## 2. Settings Center Shell

- [x] 2.1 Add a settings page registry for basic configuration, provider management, SDK dependencies, MCP servers, agents, and skills.
- [x] 2.2 Implement `SettingsShell` with UCD-aligned top navigation, sidebar navigation, content area, and active page state.
- [x] 2.3 Implement the style switcher using the central theme registry rather than page-specific theme branches.
- [x] 2.4 Replace the current `App.tsx` layout with the theme provider and settings center shell.

## 3. UCD Page Implementation

- [x] 3.1 Implement the Basic Configuration page using UCD-aligned sections for general settings, model parameters, and data/storage controls.
- [x] 3.2 Implement the Provider Management page using frontend-local provider data and UCD-aligned provider cards/details.
- [x] 3.3 Implement the SDK Dependencies page using frontend-local SDK data and UCD-aligned status summary, dependency table, and install configuration.
- [x] 3.4 Implement the MCP Servers page using frontend-local MCP data and UCD-aligned server/status views.
- [x] 3.5 Implement the Skills page using frontend-local skill data and UCD-aligned filters, skill cards, and detail sections.

## 4. Agents Page Integration

- [x] 4.1 Move existing agent listing, capability filtering, mode selection, active workflow, launch, and session details behavior into the Agents settings page.
- [x] 4.2 Ensure the Agents page calls `AgentService` with stable agent ids and does not match agents by display name.
- [x] 4.3 Preserve availability checks, compatible interaction mode validation, browser readiness checks, and launch flow separation.
- [x] 4.4 Verify both Tauri and Web runtime adapters remain behind `AgentService` with no direct `invoke()` calls from React page components.

## 5. Responsive and Visual QA

- [x] 5.1 Check the futuristic theme against the UCD assets for dark surfaces, blue accents, panel boundaries, status badges, and layout hierarchy.
- [x] 5.2 Check the minimal theme against the UCD assets for white surfaces, line-based borders, low-saturation accents, and layout hierarchy.
- [x] 5.3 Verify text fits within navigation items, buttons, badges, table cells, and cards at desktop and narrow viewport widths.
- [x] 5.4 Verify switching styles preserves the active page and relevant page state.

## 6. Tests and Validation

- [x] 6.1 Add or update frontend tests for theme registry validation, style persistence fallback, and Agents page service behavior.
- [x] 6.2 Run `npm run test`.
- [x] 6.3 Run `npm run build`.
- [x] 6.4 Run `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"; $env:CARGO_NET_OFFLINE="true"; cargo check --manifest-path src-tauri\Cargo.toml`.
- [x] 6.5 Run `openspec validate "support-switchable-ucd-settings-ui" --strict`.
