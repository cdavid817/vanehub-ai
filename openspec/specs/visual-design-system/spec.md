# visual-design-system Specification

## Purpose
Defines cross-page frontend visual design requirements for semantic tokens, typography, spacing, iconography, responsive stability, and visual quality checks shared by workspace and settings surfaces.

## Requirements
### Requirement: Cross-page visual design tokens
The frontend SHALL define cross-page visual tokens for typography, spacing, borders, radius, shadow, panel treatment, focus rings, status tones, and icon sizing so pages can share a coherent visual language.

#### Scenario: Shared token usage
- **WHEN** a page, shared primitive, or layout shell renders visual styling
- **THEN** it SHALL use semantic tokens or shared utility classes for colors, borders, panel backgrounds, status tones, focus rings, and shadows
- **AND** it SHALL avoid page-local hard-coded palettes when an existing semantic token can express the same role

#### Scenario: Two registered styles use same semantics
- **WHEN** either `futuristic` or `minimal` is active
- **THEN** both styles SHALL expose equivalent semantic token roles for background, foreground, panel, muted panel, border, input, primary, success, warning, danger, and shadow
- **AND** components SHALL switch visual appearance by token values rather than by page-specific theme branches

### Requirement: Typography and spacing rhythm
The frontend SHALL use a consistent desktop-tool typography and spacing rhythm across workspace and settings pages.

#### Scenario: Page typography hierarchy
- **WHEN** a page renders a title, section heading, body copy, metadata label, badge, or code/log text
- **THEN** the text SHALL use a consistent type scale and weight hierarchy appropriate to its container
- **AND** compact surfaces such as cards, sidebars, tables, and toolbars SHALL NOT use oversized hero-style text

#### Scenario: Stable spacing rhythm
- **WHEN** cards, list rows, form fields, toolbars, and panel sections are rendered
- **THEN** their padding and gaps SHALL follow shared spacing steps
- **AND** dynamic content, hover states, icons, badges, and loading labels SHALL NOT resize or shift the surrounding layout unexpectedly

### Requirement: Iconography and control affordance
The frontend SHALL use icons consistently for navigation, actions, statuses, and compact controls where icons improve recognition or reduce repetitive text.

#### Scenario: Icon-backed navigation and actions
- **WHEN** a navigation item, page header action, toolbar action, status label, filter control, empty state, or primary operation action is rendered
- **THEN** it SHALL include an appropriate lucide icon or existing project icon when an icon improves scanning
- **AND** icon-only controls SHALL provide accessible labels or tooltips

#### Scenario: Icon sizing consistency
- **WHEN** icons are used inside buttons, badges, tabs, navigation rows, list rows, or status indicators
- **THEN** icon sizes and stroke weights SHALL be consistent within each control class
- **AND** icons SHALL align with adjacent text without causing clipping, overlap, or layout shift

### Requirement: Visual quality verification
The UI polish implementation SHALL include visual QA for both registered styles and key responsive widths.

#### Scenario: Verify both styles
- **WHEN** the implementation is ready for review
- **THEN** the developer SHALL verify representative workspace and settings pages in both `futuristic` and `minimal` styles
- **AND** screenshots or browser inspection SHALL confirm no blank panels, overlapping text, clipped buttons, or unreadable contrast

#### Scenario: Check project standards
- **WHEN** cross-page UI rules are added or changed
- **THEN** `openspec/project.md` SHALL document the reusable UI constraints for future contributors
- **AND** the documented constraints SHALL include i18n, token-first styling, icon usage, radius, spacing, and visual QA expectations

### Requirement: Auxiliary desktop surface visual consistency
Auxiliary VaneHub desktop surfaces SHALL use the same semantic visual system, icon conventions, accessibility treatment, and registered style meanings as workspace and settings surfaces.

#### Scenario: Render the futuristic floating surface
- **WHEN** the floating assistant renders with `futuristic` active
- **THEN** it SHALL use token-provided dark operational backgrounds, restrained translucent depth, blue primary accents, readable muted text, and visible status/focus contrast

#### Scenario: Render the minimal floating surface
- **WHEN** the floating assistant renders with `minimal` active
- **THEN** it SHALL use token-provided bright solid backgrounds, restrained borders, low shadow, clear primary accents, and readable compact-density text

#### Scenario: Avoid theme-specific component branches
- **WHEN** the floating assistant switches between registered styles
- **THEN** its components SHALL change appearance through semantic token values or shared classes and SHALL NOT branch on theme names or introduce a page-local hard-coded palette

#### Scenario: Use compact icon and status semantics
- **WHEN** the collapsed control, quick actions, session status, or mini-chat controls render
- **THEN** they SHALL use consistent Bot/Sparkles or action icons, translated accessible labels, and status text or accessible names in addition to color

#### Scenario: Verify auxiliary surface quality
- **WHEN** the floating assistant implementation is ready for review
- **THEN** visual QA SHALL cover collapsed, menu, chat, empty, streaming, and error states in both registered styles
- **AND** inspection SHALL confirm transparent edges, focus visibility, readable contrast, reduced-motion behavior, and absence of clipped or overlapping content at every native surface size
