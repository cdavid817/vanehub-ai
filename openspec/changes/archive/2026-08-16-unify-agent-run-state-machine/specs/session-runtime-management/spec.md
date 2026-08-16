## ADDED Requirements

### Requirement: Session generations project canonical Runs
Every accepted Session Agent generation SHALL project preparation, execution, approval/user waits, retry, verification, cancellation, and terminal outcomes to one canonical Run while preserving existing Session lifecycle, messages, stream events, commands, and provider resume metadata.

#### Scenario: Existing chat generation completes
- **WHEN** a Session generation completes normally
- **THEN** its message and Session behavior remain compatible and its canonical Run completes through verification
