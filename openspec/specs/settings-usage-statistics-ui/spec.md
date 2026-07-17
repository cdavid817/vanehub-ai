# settings-usage-statistics-ui Specification

## Purpose
TBD - created by archiving change split-settings-center-ui-spec. Update Purpose after archive.
## Requirements
### Requirement: Usage Statistics settings page
The settings center SHALL include a localized Usage Statistics monitoring page before the About page.

#### Scenario: Navigate to usage statistics
- **WHEN** the settings center navigation is rendered
- **THEN** it SHALL include a Usage Statistics entry
- **AND** the Usage Statistics entry SHALL appear before About

#### Scenario: Render usage monitoring
- **WHEN** a user opens the Usage Statistics settings page
- **THEN** the page SHALL show localized range and refresh controls, separated reported-token and estimated-character summaries, data coverage, counted session and response details, a daily trend, a stable-Agent-id breakdown, and accounting limitations

#### Scenario: Preserve data during refresh
- **WHEN** usage statistics refresh manually or while the page is mounted
- **THEN** the page SHALL keep previously loaded data visible with a refreshing state
- **AND** settings navigation SHALL remain interactive

#### Scenario: Render empty or failed query state
- **WHEN** the selected range has no usage or the usage request fails
- **THEN** the page SHALL render a localized empty or error state without showing misleading mixed totals or a blank content panel

#### Scenario: Preserve visual style parity
- **WHEN** the Usage Statistics page renders in either `futuristic` or `minimal` style at desktop or narrow width
- **THEN** the page SHALL use shared settings primitives, semantic design tokens, accessible icon-backed controls, and responsive layouts consistent with the rest of the settings center
- **AND** trend and breakdown content SHALL remain readable without overlap, clipping, or dark-style-only contrast assumptions

### Requirement: Usage Statistics page localization
The Usage Statistics settings page SHALL render all user-visible text and locale-sensitive values through synchronized zh-CN and en resources and active-locale formatting.

#### Scenario: Translation parity
- **WHEN** Usage Statistics translation keys are added or changed
- **THEN** both zh-CN and en locale resources SHALL contain matching keys for navigation, page copy, range and refresh controls, summary and coverage labels, trend and Agent breakdowns, loading, empty, error, accessibility, and accounting limitation text

#### Scenario: Format locale-sensitive values
- **WHEN** the page formats numbers, dates, or generated timestamps
- **THEN** it SHALL format them using the active application language or a locale derived from it
