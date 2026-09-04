## ADDED Requirements

### Requirement: Transcript reflects messages regardless of origin
While a session is open, its conversation view SHALL display messages created through the runtime regardless of how they originated — the composer, a programmatic send over the service boundary, an instant-messaging connector, a scheduled task, or a seat turn dispatched by the multi-agent coordinator — without requiring a reload. Runtime-originated assistant turns SHALL stream and settle in the transcript with the same status transitions as composer-originated ones.

#### Scenario: A backend-originated user message arrives while the session is open
- **WHEN** a user message is created for the open session through the service boundary rather than the composer
- **THEN** the message appears in the conversation view without a reload

#### Scenario: A seat turn dispatched by the coordinator streams into the open transcript
- **WHEN** the multi-agent coordinator dispatches a seat turn in the open session
- **THEN** the assistant message appears in the conversation view, streams its content, and shows its settled status
- **AND** the speaker identity renders with the same captured role and Agent labels as composer-originated turns

#### Scenario: The session is opened after backend-originated messages already exist
- **WHEN** a session containing runtime-originated messages is opened
- **THEN** the conversation view lists those messages in order alongside composer-originated ones
