# VaneHub AI Project Standards

## Frontend i18n

- All new or changed user-visible React UI text MUST use i18n resources instead of hard-coded literals.
- Every user-visible translation key MUST be present in both `src/i18n/locales/zh-CN.json` and `src/i18n/locales/en.json`.
- Page titles, descriptions, button labels, placeholders, status labels, notices, confirmations, modal labels, empty states, tooltips, and frontend-owned user-facing errors MUST support Simplified Chinese and English.
- Locale resources MUST stay semantically aligned: matching keys in zh-CN and en describe the same concept and action.
- User-visible date and time formatting MUST use the active application language or a locale derived from it.

Allowed literal exceptions:

- Product, provider, Agent, model, protocol, executable, npm package, command, file path, URL, log level, and stable id values MAY remain literal.
- User-provided content, backend-returned diagnostic text, and mock fixture names MAY remain literal when they represent data rather than UI labels.

Required checks:

- `src/i18n/i18n-resource-parity.test.ts` MUST pass for locale key parity.
- Frontend page changes SHOULD keep the hard-coded visible text guardrail passing, updating only the explicit allowlist for stable identifiers or fixture data.

## Frontend Visual Design

- New or changed React UI MUST prefer semantic CSS tokens and shared utility classes from `src/styles.css` over page-local hard-coded color palettes.
- The `futuristic` and `minimal` styles MUST expose equivalent semantic roles for background, foreground, panel, muted panel, border, input, primary, success, warning, danger, focus ring, and shadows.
- Shared controls SHOULD be updated through primitives in `src/components/ui/` and shared page parts before adding page-specific Tailwind class systems.
- Cards and panels SHOULD use 8px radius or less unless an existing shared primitive requires otherwise.
- Desktop management surfaces SHOULD use compact operational density: stable 8px-based spacing, readable 12-14px metadata/body text in dense panels, and no hero-scale text inside cards, sidebars, or toolbars.
- Buttons, badges, tabs, navigation rows, status labels, and compact action groups SHOULD align icons and text consistently; icon-only controls MUST provide an accessible label or translated tooltip.
- Hover, active, disabled, loading, and focus states MUST not resize controls or shift adjacent content.
- Page sections MUST avoid nested card-in-card decoration. Use cards for repeated items, dialogs, and framed tools; use full-width or unframed layouts for larger sections.
- Visual QA for substantial UI changes MUST inspect representative pages in both `futuristic` and `minimal` styles at desktop and narrow widths for overlap, clipping, unreadable contrast, and blank panels.
