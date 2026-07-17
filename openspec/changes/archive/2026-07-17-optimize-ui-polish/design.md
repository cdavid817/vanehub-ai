## Context

The current UI already uses Tailwind CSS, semantic CSS variables, shared primitives, and a two-style registry (`futuristic`, `minimal`). The visual result is functional, but page-level Tailwind decisions are not yet governed by a shared design standard, so typography, border intensity, spacing rhythm, icon usage, and status treatment vary by surface.

`cc-switch` provides a useful reference for a polished desktop management tool: compact toolbar groups, semantic light/dark tokens, soft glass/panel surfaces, provider icon accents, icon-first utility controls, clear hover states, and small-radius list rows. VaneHub should borrow these principles while keeping its own architecture, i18n, service boundaries, and existing theme registry.

## Goals / Non-Goals

**Goals:**

- Establish a cross-page visual design standard for typography, spacing, borders, iconography, panel treatment, and responsive density.
- Improve both registered styles, `futuristic` and `minimal`, through semantic tokens in `src/styles.css` and shared UI primitives.
- Apply polish to the workspace shell, settings center shell, shared page parts, and major settings pages.
- Add icons where they improve scanning: navigation items, page headers, stats, status badges, empty states, filters, and primary actions.
- Record reusable UI constraints in `openspec/project.md` so future page changes keep the same design language.
- Preserve frontend/backend isolation; this is a frontend visual change and should not introduce native APIs.

**Non-Goals:**

- Replacing Tailwind CSS or introducing a UI component library.
- Copying `cc-switch` source code, assets, or exact branding.
- Changing service contracts, runtime adapters, Rust commands, data schemas, or CLI/SDK behavior.
- Creating a marketing landing page or changing the product information architecture.
- Replacing the two-style registry with a dark/light theme system.

## Decisions

1. Use semantic tokens as the primary implementation surface.

   The visual refresh should adjust `src/styles.css`, Tailwind token mappings, and shared primitives before touching page-specific classes. This keeps `futuristic` and `minimal` behavior centralized and avoids conditional style branches inside feature pages. Page-specific classes are acceptable only for layout needs or local component composition.

2. Treat `cc-switch` as visual reference, not dependency.

   The implementation should adopt the observed patterns that fit VaneHub: compact desktop density, subtle translucent panels, icon-backed segmented controls, restrained border contrast, provider color accents, and clear hover/active states. It should not import `cc-switch` components, copy generated icon bundles, or bring in dependencies.

3. Keep radii and density conservative.

   VaneHub should keep cards and panels at 8px radius or less unless an existing primitive demands otherwise. The UI should feel like an operational desktop tool: compact enough for repeated work, but with enough spacing to avoid crowding. This aligns with existing project design instructions and avoids oversized marketing-style surfaces.

4. Prefer shared primitives for typography and controls.

   Buttons, badges, cards, page headers, section panels, stats, list rows, and status pills should be updated once and reused. This reduces drift across Basic, CLI, SDK, MCP, Agents, Skills, and workspace surfaces.

5. Store future UI constraints in project standards.

   The answer to whether UI constraints should be written into project standards is yes. Font stack, type scale, icon sizing, border/radius rules, token-first styling, and i18n requirements are cross-cutting rules. They should live in `openspec/project.md`, with testable requirements in specs and implementation tasks in this change.

## Risks / Trade-offs

- **Risk:** A broad polish pass can become unbounded. -> **Mitigation:** work through shared tokens/primitives first, then audit named surfaces with focused visual acceptance checks.
- **Risk:** The two styles can drift if page classes hard-code colors. -> **Mitigation:** require semantic token usage and add a CSS/class audit for raw one-off palettes.
- **Risk:** More icons can add clutter. -> **Mitigation:** use icons only where they improve scanning or replace text-heavy controls; add tooltips for icon-only controls.
- **Risk:** Visual changes can regress mobile or narrow desktop layouts. -> **Mitigation:** verify desktop and narrow viewport screenshots, text wrapping, and no overlap for both styles.
- **Risk:** `cc-switch` has light/dark semantics while VaneHub has `futuristic`/`minimal`. -> **Mitigation:** translate principles into VaneHub tokens instead of mirroring theme names.

## Migration Plan

1. Update shared tokens and primitives.
2. Update settings shell and workspace shell layout polish.
3. Update page-specific panels, cards, filters, status areas, empty states, and action controls.
4. Add or update i18n strings for new labels/tooltips.
5. Add project UI standards to `openspec/project.md`.
6. Run tests/build and use browser screenshots for both `futuristic` and `minimal` styles.

Rollback is straightforward because the change is frontend-only: revert token, primitive, and page style changes without database or native migration impact.

## Open Questions

- None blocking. During implementation, exact token values should be tuned against screenshots rather than fixed in the proposal.
