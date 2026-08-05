## MODIFIED Requirements

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

#### Scenario: Loop Center panel treatment
- **WHEN** the Loop Center renders its definition list, main content, or inspector columns
- **THEN** each column SHALL use the same shared panel/card tokens (background, border, radius, shadow) that the workspace shell and settings pages already use
- **AND** the columns SHALL be visually distinguishable from the base application background in both `futuristic` and `minimal` styles

#### Scenario: Settings section-group headers
- **WHEN** a settings page groups related items under a labeled subsection (for example, MCP servers grouped by configuration scope)
- **THEN** the group label SHALL use the same styled section-header treatment already used elsewhere in Settings
- **AND** it SHALL NOT be presented as unstyled raw text separators

#### Scenario: No nested card-in-card panels
- **WHEN** a panel such as the session info panel renders repeated field or metric rows inside an already-bordered container
- **THEN** the inner rows SHALL NOT each render their own separate bordered/background card
- **AND** repeated rows SHALL use unframed rows, dividers, or a definition-list layout instead of nested card decoration

#### Scenario: Settings disclosures avoid nested sub-panel cards and duplicate implementations
- **WHEN** a settings page groups multiple distinct sub-topics inside a collapsible disclosure (for example, Basic Settings' "高级配置"/Advanced configuration)
- **THEN** the sub-topics SHALL be separated by section headers or dividers instead of each rendering its own independently bordered/backgrounded card
- **AND** settings pages SHALL reuse one shared disclosure implementation instead of independently reimplementing the same expand/collapse interaction

#### Scenario: Sibling tab panels share structural treatment
- **WHEN** a page renders multiple agent/target tabs backed by a shared `tablist` (for example, Agent Configurations' Claude Code / OpenCode / Codex CLI / OnePiece tabs)
- **THEN** each tab's panel SHALL use the same structural layout as its sibling panels (toolbar controls, status summary, list section)
- **AND** a tab panel SHALL NOT wrap its content in an extra bordered/card container that its sibling panels render without
