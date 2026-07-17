## 1. Visual Audit and Standards

- [x] 1.1 Audit current workspace, settings shell, and settings pages for typography, spacing, borders, radius, icon usage, status treatments, and raw color usage.
- [x] 1.2 Capture applicable `cc-switch` visual reference patterns: compact toolbar groups, semantic tokens, soft panels, icon-backed controls, provider icon accents, and light/dark contrast handling.
- [x] 1.3 Add reusable UI standards to `openspec/project.md` covering token-first styling, typography scale, spacing rhythm, icon usage, radius, borders, theme parity, i18n, and visual QA.

## 2. Shared Tokens and Primitives

- [x] 2.1 Refine `src/styles.css` semantic tokens for `futuristic` and `minimal`, including panel, muted panel, border, input, primary, status, shadow, and focus roles.
- [x] 2.2 Update Tailwind/theme mappings only as needed to expose the refined semantic roles without introducing a new UI library.
- [x] 2.3 Polish shared UI primitives (`Button`, `Badge`, `Card`) for consistent sizing, icon alignment, radius, focus rings, disabled states, and theme-aware status tones.
- [x] 2.4 Add or refine shared page primitives in `src/settings/pages/page-parts.tsx` for icon-backed page headers, stat cards, section panels, status pills, and tag/list treatments.

## 3. Settings Center Polish

- [x] 3.1 Polish the settings shell navigation, page content region, scrolling behavior, active states, and style switcher for both registered styles.
- [x] 3.2 Add stable icons to settings navigation entries and high-frequency page actions with translated labels/tooltips where needed.
- [x] 3.3 Polish Basic Configuration, CLI Management, SDK Dependencies, MCP Servers, Agents, and Skills pages using shared primitives and semantic tokens.
- [x] 3.4 Ensure cards, forms, filters, dialogs, empty states, operation logs, and status/error banners keep consistent spacing, borders, icon alignment, and text hierarchy.

## 4. Workspace Shell Polish

- [x] 4.1 Polish top bar, conversation sidebar, main content panel, composer area, information panel, and status bar using the shared token system.
- [x] 4.2 Add or refine icons for sidebar utilities, session cards, activity/folder/archive groups, information panel tabs, create-session dialog, and compact action groups.
- [x] 4.3 Verify session cards, folder paths, agent labels, context actions, and collapse/expand controls do not overlap or shift at narrow desktop widths.

## 5. Theme and i18n Coverage

- [x] 5.1 Verify `futuristic` style has a dark focused operational appearance with subtle panel depth and readable muted text.
- [x] 5.2 Verify `minimal` style has a bright crisp operational appearance with restrained borders and low-shadow density.
- [x] 5.3 Add zh-CN and en i18n entries for all new visible labels, tooltips, empty states, and accessibility labels.
- [x] 5.4 Audit implementation for hard-coded user-visible strings and page-local hard-coded palettes introduced by this change.

## 6. Tests and Visual Verification

- [x] 6.1 Add or update frontend tests for theme registry/style token behavior and any new shared primitive logic.
- [x] 6.2 Run `npm run test`.
- [x] 6.3 Run `npm run build`.
- [x] 6.4 Run `cargo check --manifest-path src-tauri/Cargo.toml` if frontend changes require the full workspace verification pass.
- [x] 6.5 Run `openspec validate --specs --strict`.
- [x] 6.6 Run `openspec validate optimize-ui-polish --strict`.
- [x] 6.7 Use browser or Playwright screenshots to inspect representative workspace and settings pages in both `futuristic` and `minimal` styles at desktop and narrow widths.
