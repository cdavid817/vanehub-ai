## ADDED Requirements

### Requirement: Responsive session selection
The workspace SHALL reflect selection of an already-loaded, non-archived session without waiting for active-session persistence to complete, while the frontend agent service remains the authoritative persistence boundary.

#### Scenario: Select an already-loaded session
- **WHEN** the user selects a different non-archived session card that is already present in the sidebar
- **THEN** the selected marker and workspace SHALL begin rendering that session immediately
- **AND** persistence SHALL continue asynchronously through the frontend agent service

#### Scenario: Session persistence fails
- **WHEN** persisting an optimistic session selection fails
- **THEN** the workspace SHALL restore the previously active session
- **AND** the user SHALL receive an error notification

#### Scenario: Rapid successive selection
- **WHEN** the user selects multiple sessions before earlier persistence requests finish
- **THEN** the most recently selected session SHALL remain visible
- **AND** a late result from an older request SHALL NOT replace or roll back the newer selection

#### Scenario: Select the current session
- **WHEN** the user selects the session that is already active
- **THEN** the workspace SHALL avoid resetting session-scoped tabs, drafts, or message subscriptions

#### Scenario: Revisit a recently displayed session
- **WHEN** the user returns to a session whose conversation data remains cached
- **THEN** the workspace SHALL render the cached conversation immediately while any required refresh continues in the background
