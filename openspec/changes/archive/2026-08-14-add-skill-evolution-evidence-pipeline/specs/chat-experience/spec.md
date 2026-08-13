## ADDED Requirements

### Requirement: Explicit assistant-message feedback
The chat experience SHALL let users submit one current feedback state of `helpful`, `unhelpful`, or `corrected` for a completed assistant message. Corrected feedback MAY include one bounded optional correction note. Feedback SHALL be sent through the frontend service boundary and SHALL not edit the assistant message.

#### Scenario: Mark response helpful
- **WHEN** a user marks a completed assistant message helpful
- **THEN** the page SHALL persist structured helpful feedback and show the saved state on that message

#### Scenario: Mark response unhelpful
- **WHEN** a user marks a completed assistant message unhelpful
- **THEN** the page SHALL persist structured unhelpful feedback without requiring a free-form note

#### Scenario: Submit correction
- **WHEN** a user selects corrected and submits a note within the configured limit
- **THEN** the service SHALL sanitize and persist the feedback projection and show the corrected state

#### Scenario: Feedback on incomplete message
- **WHEN** a message is streaming, failed before producing a completed response, or belongs to an inaccessible session
- **THEN** feedback submission SHALL be unavailable or rejected without creating evidence

#### Scenario: Replace prior feedback
- **WHEN** a user changes feedback on the same completed message
- **THEN** the service SHALL retain one current feedback state while preserving an evidence audit transition without producing duplicate active feedback signals

### Requirement: Feedback privacy and failure behavior
Feedback correction notes SHALL display their character limit and privacy warning, SHALL be sanitized before evidence persistence, and SHALL not be written to frontend files or feature-specific logs. A failed save SHALL remain visible and retryable without changing the assistant message.

#### Scenario: Sensitive correction note
- **WHEN** a correction note contains a recognized sensitive value
- **THEN** persisted feedback evidence SHALL contain only the sanitized bounded form

#### Scenario: Feedback save fails
- **WHEN** the adapter reports that feedback was not persisted
- **THEN** the UI SHALL show a localized row-scoped error, retain unsaved input for retry, and SHALL not display the feedback as saved

#### Scenario: Web feedback parity
- **WHEN** feedback is submitted in Web/mock mode
- **THEN** the adapter SHALL simulate the same states, sanitization-result shape, replacement behavior, and failure contract without native persistence

