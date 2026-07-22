## Context

The Settings About page is a frontend-only settings surface shared by the Tauri desktop runtime and the browser Web runtime. Previous UI changes removed runtime/agent and local CLI environment cards, leaving the remaining product information split across several separate panels.

## Goals / Non-Goals

**Goals:**

- Present About content in a horizontal, scannable layout that follows the Basic Configuration page's spacing and card language.
- Keep related product metadata, links, and update status in a single software details panel.
- Keep changelog and product positioning together as related informational content.
- Preserve localized strings and the existing `about-service` update-check boundary.

**Non-Goals:**

- No new settings navigation entries.
- No new runtime, Tauri, SQLite, or Web adapter behavior.
- No reintroduction of runtime/agent or local CLI environment sections.

## Decisions

- Use the existing `PageHeader` and `SectionPanel` primitives instead of introducing a page-specific layout system. This keeps the About page aligned with other settings pages and semantic design tokens.
- Use a responsive two-column grid for desktop widths and a single-column flow on narrower viewports. This provides the requested horizontal display without causing small-screen overflow.
- Keep update checking inside the software details panel. The update state and repository actions are part of software metadata, so grouping them reduces scattered controls.
- Keep all update behavior routed through `checkAboutUpdates()` from `about-service`. React components still avoid direct Tauri or network-runtime specific calls.

## Risks / Trade-offs

- Wider panels can expose long repository URLs. Metadata cells keep `break-all` behavior to prevent overflow.
- Combining changelog and product positioning reduces the number of independent cards. The section uses internal dividers to preserve scanability.
- Visual verification remains frontend-focused because this change has no native behavior.

## Migration Plan

- Apply the frontend layout change.
- Update About page regression coverage to assert the page still renders localized product details and does not render the removed local CLI environment section.
- Validate TypeScript, targeted tests, production build, and OpenSpec.

## Open Questions

None.
