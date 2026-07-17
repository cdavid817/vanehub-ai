## ADDED Requirements

### Requirement: Usage Statistics settings page
The settings center SHALL include a localized Usage Statistics page before the About page.

#### Scenario: Navigate to usage statistics
- **WHEN** the settings center navigation is rendered
- **THEN** it SHALL include a Usage Statistics entry
- **AND** the Usage Statistics entry SHALL appear before About

#### Scenario: Render usage statistics page
- **WHEN** a user opens the Usage Statistics settings page
- **THEN** the page SHALL show localized page title, description, time range controls, usage stat cards, counted session and message details, and first-version accounting constraints

#### Scenario: Preserve visual style parity
- **WHEN** the Usage Statistics page renders in either `futuristic` or `minimal` style
- **THEN** the page SHALL use shared settings primitives, semantic design tokens, and icon-backed controls consistent with the rest of the settings center

### Requirement: Usage Statistics page localization
The Usage Statistics settings page SHALL render all user-visible text through synchronized zh-CN and en translation resources.

#### Scenario: Translation parity
- **WHEN** Usage Statistics translation keys are added
- **THEN** both zh-CN and en locale resources SHALL contain matching keys for navigation, search placeholder, page copy, controls, labels, empty/error states, and limitation notes
