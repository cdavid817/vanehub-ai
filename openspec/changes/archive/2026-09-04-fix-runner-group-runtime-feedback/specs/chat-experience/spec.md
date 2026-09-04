## MODIFIED Requirements

### Requirement: Assistant responses stream into the message list
The system SHALL display assistant responses incrementally as stream events arrive through the frontend agent service. When events target a newly created assistant message that is not yet present in the frontend cache, the client SHALL reconcile that message and preserve subsequent incremental events instead of remaining on an indefinite waiting indicator. Multi-seat messages SHALL retain their stable speaker-seat attribution throughout reconciliation and streaming.

#### Scenario: Assistant response starts
- **WHEN** the agent service emits a `started` event for the active session
- **THEN** an assistant message with `streaming` status SHALL appear
- **AND** a waiting indicator SHALL be visible until response content or another activity arrives

#### Scenario: Token event appends content
- **WHEN** the agent service emits a `token` event for a streaming assistant message
- **THEN** the token content SHALL be appended to that assistant message
- **AND** the message SHALL NOT be duplicated

#### Scenario: Newly created member message races the cache
- **WHEN** a started, thinking, tool, or token event targets a seat-attributed message not yet present in the active message cache
- **THEN** the client SHALL reconcile the persisted message and apply or recover the incremental state without waiting for terminal completion
- **AND** the message SHALL remain attributed to its stable speaker seat

#### Scenario: Thinking event appends thinking content
- **WHEN** the agent service emits a `thinking` event for a streaming assistant message
- **THEN** the thinking content SHALL be appended to a collapsible thinking block for that message

#### Scenario: Tool event appends tool use
- **WHEN** the agent service emits a `tool_use` event whose stable tool-use id is not present on the streaming assistant message
- **THEN** one logical tool activity SHALL be added to that message

#### Scenario: Tool status event updates its logical activity
- **WHEN** the agent service emits another `tool_use` event with a stable tool-use id already present on the message
- **THEN** the existing logical activity SHALL be updated with the latest status, input, and output
- **AND** the status transition SHALL NOT create a duplicate visible activity

#### Scenario: Completion marks message complete
- **WHEN** the agent service emits a `completed` event for a streaming assistant message
- **THEN** the assistant message status SHALL become `completed`
- **AND** the waiting indicator SHALL be hidden

#### Scenario: Failure marks message failed
- **WHEN** the agent service emits a `failed` event for a streaming assistant message
- **THEN** the assistant message status SHALL become `failed`
- **AND** the error SHALL be visible to the user

