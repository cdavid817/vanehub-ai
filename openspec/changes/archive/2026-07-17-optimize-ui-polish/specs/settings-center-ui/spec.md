## ADDED Requirements

### Requirement: Polished settings visual system
The settings center SHALL apply the shared visual design system consistently across the shell, navigation, page headers, page sections, cards, forms, tables, filters, and operation panels.

#### Scenario: Settings shell visual consistency
- **WHEN** the settings center shell renders
- **THEN** top navigation, sidebar navigation, page content, and fixed scroll regions SHALL share consistent typography, spacing, border strength, panel treatment, hover states, and focus rings
- **AND** the visual result SHALL remain coherent in both `futuristic` and `minimal` styles

#### Scenario: Settings page visual consistency
- **WHEN** Basic Configuration, CLI Management, SDK Dependencies, MCP Servers, Agents, or Skills pages render
- **THEN** page headers, stat summaries, section panels, cards, form controls, empty states, status messages, and operation logs SHALL use shared primitives or shared visual classes
- **AND** page-specific styling SHALL not create a conflicting radius, color, or spacing system

### Requirement: Icon-enhanced settings interactions
The settings center SHALL use icons to improve scanability of navigation and high-frequency actions.

#### Scenario: Settings navigation icons
- **WHEN** the settings sidebar renders page navigation
- **THEN** each navigation entry SHALL include a stable icon that reflects the page purpose
- **AND** the active, hover, and disabled states SHALL remain legible in both registered styles

#### Scenario: Settings action icons
- **WHEN** a settings page renders refresh, install, update, rollback, delete, import, export, add, edit, filter, copy, open, or settings actions
- **THEN** the action SHALL include a consistent icon unless the control is purely textual by design
- **AND** icon-only actions SHALL expose a translated tooltip or accessible label

### Requirement: Settings theme refinement
The settings center SHALL visibly differentiate and polish both registered styles without changing page behavior.

#### Scenario: Futuristic style refinement
- **WHEN** `futuristic` style is active
- **THEN** settings surfaces SHALL use a dark operational appearance with subtle depth, restrained translucent or glass-like panels, clear blue primary accents, and readable muted text
- **AND** borders and shadows SHALL add structure without making the page look noisy

#### Scenario: Minimal style refinement
- **WHEN** `minimal` style is active
- **THEN** settings surfaces SHALL use a bright, crisp, low-shadow appearance with restrained borders, clear primary accents, and higher information density
- **AND** the style SHALL not rely on dark-only contrast assumptions from `futuristic`
