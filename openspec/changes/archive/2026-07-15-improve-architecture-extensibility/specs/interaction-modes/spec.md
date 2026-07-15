## ADDED Requirements

### Requirement: Interaction task lifecycle alignment
Interaction mode lifecycle reporting SHALL align with the common observable operation model while preserving mode-specific session details behind runtime adapters.

#### Scenario: Browser operation lifecycle
- **WHEN** a browser interaction workflow starts, runs, fails, or stops
- **THEN** the system SHALL report lifecycle updates using the common operation status values while keeping browser-specific readiness details scoped to the browser adapter

#### Scenario: Native desktop operation lifecycle
- **WHEN** a native desktop interaction workflow starts, runs, fails, or stops
- **THEN** the system SHALL report lifecycle updates using the common operation status values while keeping OS/window-specific details scoped to the native adapter
