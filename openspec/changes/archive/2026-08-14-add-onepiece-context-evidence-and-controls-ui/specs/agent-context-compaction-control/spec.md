## ADDED Requirements

### Requirement: Persisted user preference suppresses automatic compaction
The automatic-compaction decision SHALL combine the persisted user preference with request-level suppression and generation-scoped safety guards. A disabled user preference SHALL suppress every automatic compaction attempt for generations started from that settings snapshot without mutating the provider request context.

#### Scenario: User preference is disabled before generation
- **WHEN** a OnePiece generation starts with automatic context compaction disabled in application settings
- **THEN** the generation SHALL NOT optimize, summarize, or otherwise mutate its context through automatic compaction
- **AND** normal provider generation SHALL continue with the unmodified prepared context

#### Scenario: Preference changes during an active generation
- **WHEN** the user changes the automatic-compaction preference while a generation is active
- **THEN** the active generation SHALL retain its captured preference
- **AND** the new preference SHALL apply to later generations

#### Scenario: Preference is enabled but request is suppressed
- **WHEN** the persisted preference is enabled and the generation request declares automatic compaction suppressed
- **THEN** request-level suppression SHALL still prevent automatic compaction

