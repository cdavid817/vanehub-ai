## ADDED Requirements

### Requirement: Periodic polls log only changes
A periodic runtime poll that observed nothing new SHALL NOT be persisted as a log record.

#### Scenario: Terminal usage poll with nothing to persist
- **WHEN** the terminal usage poll runs and persists nothing without error
- **THEN** no record SHALL be written for that tick
- **AND** a poll that persisted usage SHALL still be recorded at `debug`, a failed poll at `warn`, and the session-exit read at `info`
