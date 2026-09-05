## ADDED Requirements

### Requirement: System activity service boundary
The Skill management service SHALL expose system-session listing, timeline pages, filters/search, unread state, projection health, preferences, rebuild, and export through matching desktop/Tauri and Web adapters. React components MUST NOT invoke native commands directly.

#### Scenario: Desktop timeline query
- **WHEN** the desktop UI requests a system timeline through the Skill service
- **THEN** the Tauri adapter invokes the native projection boundary and returns typed sanitized items

#### Scenario: Web timeline query
- **WHEN** Web/mock requests the same operation
- **THEN** the Web adapter returns equivalent in-memory results with explicit mock durability metadata

### Requirement: Conflict-safe projection controls
Preference, read-cursor, and rebuild operations SHALL require current versions where state can conflict and SHALL return the current safe state on stale updates.

#### Scenario: Two views update read cursor
- **WHEN** one view advances the cursor before another submits an older value
- **THEN** the service preserves monotonic read progress unless the user explicitly requests a bounded mark-unread operation

### Requirement: Bounded system activity payloads
Timeline, health, rebuild, search, and export operations SHALL be paginated and size bounded and MUST exclude unprojected sensitive source data.

#### Scenario: Timeline has many items
- **WHEN** the user loads the next page
- **THEN** the service uses a stable sequence cursor and returns completeness metadata

