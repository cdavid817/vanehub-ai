## MODIFIED Requirements

### Requirement: Usage Statistics settings page
The settings center SHALL include a localized Usage Statistics monitoring page before the About page and SHALL present invocation-derived consumption without implying provider invoice reconciliation.

#### Scenario: Navigate to usage statistics
- **WHEN** the settings center navigation is rendered
- **THEN** it SHALL include a Usage Statistics entry before About

#### Scenario: Render usage monitoring
- **WHEN** a user opens the Usage Statistics settings page
- **THEN** the page SHALL show range and refresh controls, reported and reported-derived Token summaries, estimated characters, coverage, call and session counts, daily trends, and Agent, provider, model, purpose, quality, and status breakdowns
- **AND** user-response consumption SHALL remain distinguishable from internal compaction and memory consumption

#### Scenario: Filter observed consumption
- **WHEN** a user selects one or more supported Agent, provider, model, purpose, quality, or status filters
- **THEN** summaries, trends, coverage, and breakdowns SHALL use the same filter set
- **AND** unavailable historical dimensions SHALL be represented as unknown rather than invented

#### Scenario: Inspect session consumption
- **WHEN** a session contains multiple invocation purposes or providers
- **THEN** its usage surface SHALL show the total and a bounded breakdown without requiring access to raw provider payloads

#### Scenario: Preserve data during refresh
- **WHEN** usage statistics refresh manually or while the page is mounted
- **THEN** the page SHALL keep previously loaded data visible with a refreshing state
- **AND** settings navigation SHALL remain interactive

#### Scenario: Render empty or failed query state
- **WHEN** the selected filters contain no usage or the request fails
- **THEN** the page SHALL render a localized empty or error state without mixed units, stale totals, or a blank content panel

#### Scenario: Preserve visual style parity
- **WHEN** the Usage Statistics page renders in either supported style at desktop or narrow width
- **THEN** it SHALL use shared settings primitives, semantic tokens, accessible controls, and responsive layouts
- **AND** trend and breakdown content SHALL remain readable without overlap, clipping, or style-specific contrast assumptions

## ADDED Requirements

### Requirement: Usage accounting disclosure and localization
The Usage Statistics UI SHALL localize all accounting dimensions and SHALL continuously disclose measurement quality, unsupported sources, and non-billing semantics.

#### Scenario: Explain quality classes
- **WHEN** reported, reported-derived, or estimated measurements are shown
- **THEN** zh-CN and en resources SHALL explain their origin and precision without calling estimates reported Tokens

#### Scenario: Explain provider semantics
- **WHEN** cache, cache-write, reasoning, or provider-total dimensions are displayed
- **THEN** the UI SHALL avoid presenting overlapping dimensions as an additive stack unless their adapter semantics permit it

#### Scenario: Format locale-sensitive values
- **WHEN** the page formats numbers, dates, percentages, durations, or generated timestamps
- **THEN** it SHALL format them using the active application language or a locale derived from it

