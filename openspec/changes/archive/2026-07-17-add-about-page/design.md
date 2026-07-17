## Context

The settings center currently registers pages through `src/settings/settings-pages.ts` and renders each page inside the shared `SettingsShell`. Basic, CLI, SDK, MCP, Agents, and Skills pages already share page headers, section panels, semantic Tailwind tokens, lucide icons, and synchronized zh-CN/en locale resources.

The About page is informational. It does not require runtime-specific data, native commands, SQLite, or a frontend service boundary change.

## Goals / Non-Goals

**Goals:**

- Add a localized About page to the settings center navigation.
- Present product identity, runtime support, supported agent ecosystem, GitHub repository, changelog, update-check status, and build metadata in compact management UI sections.
- Preserve desktop and Web behavior without adding Tauri `invoke()` calls or adapter methods.
- Keep visual implementation consistent with existing settings page primitives.

**Non-Goals:**

- Reading package metadata dynamically from Node, Rust, or Tauri.
- Adding native auto-update installation, license dialogs, telemetry controls, or native About-window integration.
- Changing settings shell routing or persistence behavior.

## Decisions

- Register the page through `settingsPages` with a new `about` page id.
  - Rationale: the shell already drives navigation, search placeholders, breadcrumbs, and persistent mounting from this registry.
  - Alternative considered: add a separate footer link outside the settings page registry. That would create a second navigation pattern and bypass existing shell behavior.
- Place the About page as the final registry item.
  - Rationale: About is informational and should sit after operational settings tabs.
  - Alternative considered: place it near Basic Configuration. That mixes product metadata with editable settings.
- Keep product details frontend-local and localized, but put update checking behind a small frontend service.
  - Rationale: static details do not require backend dependency, while network update checks are easier to test and evolve when separated from the React component.
  - Alternative considered: expose metadata and update checks through Rust/Tauri. That would add native and Web adapter work before installation updates are in scope.
- Use shared `PageHeader`, `SectionPanel`, `Badge`, and lucide icons.
  - Rationale: this preserves settings visual consistency and avoids page-local design systems.
  - Alternative considered: custom cards and styling. That would increase drift from the settings visual system.
- Model the page after CC Switch's About section information hierarchy: product identity first, action row for GitHub/release notes/check updates, then environment/tooling details. Do not copy its heavier native update/install flow.

## Risks / Trade-offs

- Static version/build metadata can become stale → derive version from package metadata at build time where possible and keep build labels generic.
- Another settings page increases sidebar density → place About after operational pages, where users expect product information.
- GitHub update checks can fail offline or be rate-limited → show a localized non-blocking error and keep the GitHub/releases links available.

## Migration Plan

Additive frontend-only change. Rollback is removing the page component, registry entry, locale keys, tests, and spec delta.

## Open Questions

None.
