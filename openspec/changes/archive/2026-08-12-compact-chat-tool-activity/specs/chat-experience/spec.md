## MODIFIED Requirements

### Requirement: Assistant responses stream into the message list
The system SHALL display assistant responses incrementally as stream events arrive through the frontend agent service.

#### Scenario: Assistant response starts
- **WHEN** the agent service emits a `started` event for the active session
- **THEN** an assistant message with `streaming` status SHALL appear
- **AND** a waiting indicator SHALL be visible until response content arrives

#### Scenario: Token event appends content
- **WHEN** the agent service emits a `token` event for a streaming assistant message
- **THEN** the token content SHALL be appended to that assistant message
- **AND** the message SHALL NOT be duplicated

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

## ADDED Requirements

### Requirement: Tool-heavy turns preserve an actionable visual hierarchy
The chat UI SHALL present tool activities in a localized compact group that keeps action-required and unsuccessful work discoverable without allowing completed history to dominate the assistant message.

#### Scenario: Summarize multiple tool activities
- **WHEN** an assistant message contains multiple tool activities
- **THEN** the UI SHALL show localized totals for active, approval-required, failed, and completed activities
- **AND** individual activities SHALL remain available through keyboard-accessible disclosure controls

#### Scenario: Prioritize actionable activities
- **WHEN** tool activities include approval-required, active, failed, and completed statuses
- **THEN** approval-required activities SHALL remain visible with their approval controls
- **AND** active activities SHALL remain visible before terminal history

#### Scenario: Collapse recoverable failure history
- **WHEN** one or more tool activities fail but the containing assistant message is not in terminal failed status
- **THEN** the UI SHALL show the failed activity count in a failure-history disclosure that is collapsed by default
- **AND** the user SHALL be able to expand the disclosure and inspect every failed activity

#### Scenario: Disclose a blocking failure
- **WHEN** one or more tool activities fail and the containing assistant message enters terminal failed status
- **THEN** the failure-history disclosure SHALL be open initially
- **AND** the most recent failure SHALL remain identifiable

#### Scenario: Aggregate repeated failures visually
- **WHEN** consecutive failed activities have the same tool, safe input preview, and error output signature
- **THEN** the failure history SHALL represent them as one row with an occurrence count
- **AND** expanding the row SHALL retain access to every occurrence payload

#### Scenario: Collapse completed history
- **WHEN** a tool activity is completed and does not require user action
- **THEN** the UI SHALL include it in a completed-history section that is collapsed by default
- **AND** the user SHALL be able to expand the section and inspect the activity input and output

#### Scenario: Explain an activity concisely
- **WHEN** a tool activity has structured input containing a command, path, query, or action
- **THEN** the UI SHALL show a bounded safe preview next to a localized tool label
- **AND** raw structured input and output SHALL remain bounded inside on-demand details

#### Scenario: Render a single completed activity
- **WHEN** an assistant message contains only one completed tool activity
- **THEN** the compact group SHALL still identify the activity and its completed status without requiring a tall standalone card

#### Scenario: Collapse the complete activity region after success
- **WHEN** an assistant message completes successfully with no pending approval and the user has not manually chosen the activity-region state
- **THEN** the complete tool-activity content SHALL collapse
- **AND** its localized status counts SHALL remain visible in the header

#### Scenario: Inspect or hide activity content manually
- **WHEN** the user activates the tool-activity header toggle and no approval is pending
- **THEN** the UI SHALL toggle the complete activity content
- **AND** SHALL retain that choice for subsequent tool snapshots on the same message

#### Scenario: Keep approval controls visible
- **WHEN** any tool activity requires approval
- **THEN** the complete activity region SHALL remain expanded regardless of a prior collapsed preference
- **AND** the approval controls SHALL remain operable

#### Scenario: Summarize collapsed active work
- **WHEN** the user collapses a region containing active work
- **THEN** the header SHALL continue to show the active count and a bounded preview of the current activity
