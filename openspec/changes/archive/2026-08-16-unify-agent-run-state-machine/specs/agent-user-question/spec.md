## ADDED Requirements

### Requirement: User questions project canonical state
A valid interactive question SHALL transition its canonical Run to waiting user; an answer SHALL resume it, and cancellation or restart SHALL invalidate the ephemeral question without allowing a late answer.

#### Scenario: Restart interrupts a question
- **WHEN** the desktop restarts while a Run waits for a user answer
- **THEN** the question is invalidated and the Run records the owner-defined interrupted outcome rather than remaining waiting
