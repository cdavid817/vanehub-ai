## Why

VaneHub AI already has service-backed settings and workspace surfaces, but the visual system still feels uneven across pages: typography, border strength, spacing, icon usage, cards, and the two registered visual styles are not governed by a consistent cross-page standard. This change uses the nearby `cc-switch` project as a practical desktop-tool reference to make the UI more polished, scannable, and consistent.

## What Changes

- Refine the shared visual language for fonts, type scale, border weight, border spacing, component radius, shadows, icon sizing, hover/active states, and page density.
- Optimize both registered styles, `futuristic` and `minimal`, through semantic tokens rather than page-specific style branches.
- Add consistent icon usage to page navigation, page headers, stats, empty states, action buttons, status badges, filters, and operational controls where an icon improves scanning.
- Apply the visual refresh across the workspace shell, settings center, and major settings pages without changing service contracts or backend behavior.
- Use `cc-switch` as reference for compact desktop-app patterns: semantic light/dark tokens, soft glass/panel treatment, compact toolbar groups, icon-first controls with tooltips, restrained borders, and brand/provider icon accents.
- Add durable UI design constraints to project standards so future page work keeps typography, spacing, icon, and theme behavior consistent.

## Capabilities

### New Capabilities
- `visual-design-system`: Defines cross-page UI polish requirements for typography, spacing, borders, icons, semantic tokens, and style quality gates.

### Modified Capabilities
- `settings-center-ui`: Strengthen requirements for polished settings pages, icon-backed navigation/actions, page density, and both registered visual styles.
- `main-layout-ui`: Strengthen requirements for workspace shell visual consistency, iconography, panel rhythm, and theme alignment.

## Impact

- Frontend UI: shared CSS tokens in `src/styles.css`, theme registry/provider behavior, UI primitives under `src/components/ui/`, main layout components, settings page shared parts, and page-specific Tailwind classes.
- Visual themes: both `futuristic` and `minimal` need coordinated token improvements and visual QA.
- i18n: any new tooltips, labels, empty states, or visible copy must use zh-CN and en resources.
- Project standards: UI constraints should be added to `openspec/project.md` because these rules apply to future page changes, not only this implementation.
- Runtime boundaries: no new Tauri commands or service APIs are expected; React components must remain behind existing service interfaces.
- Reference source: `D:\cdavid\Documents\code\cc-switch` is used as an inspiration source for visual patterns only, not as copied implementation.
