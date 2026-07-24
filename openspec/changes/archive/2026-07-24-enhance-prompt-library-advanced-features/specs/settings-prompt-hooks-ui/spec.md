## ADDED Requirements

### Requirement: Prompt Hook variable guidance
The Prompt Hooks settings page SHALL expose the supported template-variable catalog through localized, service-backed guidance.

#### Scenario: Discover supported variables
- **WHEN** a user edits or previews a Prompt Hook template
- **THEN** the page SHALL show each canonical variable name, localized description, availability, and preview example returned through the Agent service
- **AND** selecting a variable SHALL insert its exact token without hard-coding a separate frontend catalog

#### Scenario: Display unknown-variable validation
- **WHEN** publication fails because a draft contains unknown variables
- **THEN** the page SHALL preserve the draft and show a localized error identifying those variables

### Requirement: Prompt Hook lifecycle controls
The Prompt Hooks settings page SHALL provide localized draft, publish, version-history, and rollback controls for user-created Hooks.

#### Scenario: Show draft state
- **WHEN** a user Hook has unpublished changes
- **THEN** its card and editor SHALL distinguish the draft revision from the active published version
- **AND** the inventory SHALL continue to reflect whether a live published version exists

#### Scenario: Publish a draft
- **WHEN** a user confirms publication of a valid draft
- **THEN** the page SHALL publish through the Agent service, refresh the Hook and version history, and identify the newly active version

#### Scenario: Roll back from version history
- **WHEN** a user confirms rollback to a historical version
- **THEN** the page SHALL request rollback through the Agent service and identify the new published version
- **AND** it SHALL show that any unrelated draft remains unpublished

#### Scenario: Protect built-in Hooks
- **WHEN** a backend-owned built-in Hook is displayed
- **THEN** draft, publish, and rollback mutation controls SHALL not be offered

### Requirement: Prompt Hook version evaluation display
The Prompt Hooks settings page SHALL display compact operational outcome summaries for immutable versions without exposing raw execution content.

#### Scenario: Compare version summaries
- **WHEN** evaluation data exists for a Hook
- **THEN** the version panel SHALL show execution count, success rate, successful, failed, and cancelled counts, and localized elapsed-time summaries for each version
- **AND** the active published version SHALL be visually identifiable

#### Scenario: Explain limited attribution
- **WHEN** version evaluation summaries are shown
- **THEN** the page SHALL state that the metrics are correlated operational outcomes rather than proof that a Prompt caused the result

#### Scenario: Show empty evaluation state
- **WHEN** a version has no evaluated live executions
- **THEN** the page SHALL show a localized no-data state rather than a zero-percent success rate
