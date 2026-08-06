# agent-cross-session-memory Specification (Delta)

## RENAMED Requirements

- FROM: `### Requirement: Web runtime memory parity`
- TO: `### Requirement: Web runtime memory toggle parity`

## ADDED Requirements

### Requirement: Memory enablement toggle
The system SHALL provide a host-level toggle controlling whether an agent's cross-session memory feature is active. When disabled, explicit saves, automatic extraction, and system-prompt injection SHALL all stop. Disabling the toggle SHALL NOT delete previously stored memories; re-enabling it SHALL make them available again exactly as before.

#### Scenario: Memory disabled stops all memory activity
- **WHEN** the memory enablement toggle is off
- **THEN** the explicit save tool SHALL be rejected, automatic extraction SHALL be skipped, and no memory section SHALL be injected into the system prompt

#### Scenario: Re-enabling restores prior memories
- **WHEN** the memory enablement toggle is turned back on after being off
- **THEN** memories saved before it was disabled SHALL again be listed and injected, since disabling never deleted them

#### Scenario: Default preserves existing behavior
- **WHEN** no value has been saved for the memory enablement toggle
- **THEN** the system SHALL treat it as enabled, matching the feature's behavior before this toggle existed

### Requirement: Tool-assisted chat extraction toggle
The system SHALL provide a second host-level toggle, meaningful only while the memory enablement toggle is on, controlling whether automatic extraction runs for a compaction whose compacted turns include a tool call. This toggle SHALL NOT affect the explicit save tool, which remains available regardless of its value.

#### Scenario: Tool-assisted extraction disabled
- **WHEN** the tool-assisted chat extraction toggle is off and the turns being compacted include at least one tool call
- **THEN** the system SHALL skip automatic extraction for that compaction

#### Scenario: Non-tool-assisted sessions are unaffected
- **WHEN** the tool-assisted chat extraction toggle is off but the turns being compacted include no tool call
- **THEN** automatic extraction SHALL proceed exactly as if the toggle were on

#### Scenario: Explicit saves are unaffected
- **WHEN** the tool-assisted chat extraction toggle is off
- **THEN** the model calling the explicit save tool SHALL still persist the memory normally

#### Scenario: Default preserves existing behavior
- **WHEN** no value has been saved for the tool-assisted chat extraction toggle
- **THEN** the system SHALL treat it as enabled, matching the feature's behavior before this toggle existed

## MODIFIED Requirements

### Requirement: Explicit memory saving
The system SHALL provide a tool the model can call to save a memory, auto-approved without requiring user confirmation, while the memory enablement toggle is on.

#### Scenario: Model saves a memory via the tool
- **WHEN** the model calls the memory-saving tool with content during a generation and the memory enablement toggle is on
- **THEN** the system SHALL persist it immediately without requiring approval
- **AND** it SHALL be available to future sessions in the same scope

#### Scenario: Tool is inert when memory is disabled
- **WHEN** the model calls the memory-saving tool while the memory enablement toggle is off
- **THEN** the system SHALL reject the call without persisting anything

### Requirement: Automatic memory extraction
The system SHALL attempt best-effort automatic extraction of memorable content from turns that context compaction is about to replace, without failing the generation if extraction fails, while the memory enablement toggle is on and, when the compacted turns include a tool call, only while the tool-assisted chat extraction toggle is also on.

#### Scenario: Extraction runs when compaction triggers
- **WHEN** context compaction triggers during a generation and both applicable toggles allow it
- **THEN** the system SHALL make one additional call to extract memorable facts from the turns being compacted
- **AND** any extracted facts SHALL be persisted in the same store the explicit tool writes to

#### Scenario: Extraction finds nothing worth remembering
- **WHEN** the extraction call determines there is nothing worth remembering long-term
- **THEN** the system SHALL save no memories from that extraction, without treating this as a failure

#### Scenario: Extraction failure does not affect compaction
- **WHEN** the extraction call itself fails
- **THEN** the system SHALL log the failure and continue the generation and its compaction unaffected

#### Scenario: Extraction skipped when memory is disabled
- **WHEN** context compaction triggers while the memory enablement toggle is off
- **THEN** the system SHALL NOT make an extraction call

#### Scenario: Extraction skipped for a tool-assisted session when the sub-toggle is off
- **WHEN** context compaction triggers, the memory enablement toggle is on, the tool-assisted chat extraction toggle is off, and the compacted turns include a tool call
- **THEN** the system SHALL NOT make an extraction call for that compaction

### Requirement: Memory injection into the system prompt
The system SHALL inject an agent's scoped memories into its generation requests as part of the system prompt while the memory enablement toggle is on, bounded by a character budget, and SHALL never write memory content into the turns list context compaction manipulates.

#### Scenario: Memories injected alongside Skill content
- **WHEN** a generation runs for an agent with both bound Skills and stored memories in scope, and the memory enablement toggle is on
- **THEN** the system prompt SHALL include both, as distinct sections

#### Scenario: Injected memories are bounded
- **WHEN** an agent's scoped memories exceed the injection character budget
- **THEN** the system SHALL include memories by recency up to the budget rather than including all of them unbounded

#### Scenario: Memory content survives compaction
- **WHEN** context compaction triggers during a generation with injected memory content
- **THEN** the injected memory content SHALL remain present, complete, and unchanged on every subsequent request of that generation

#### Scenario: No injection when memory is disabled
- **WHEN** the memory enablement toggle is off
- **THEN** the system SHALL NOT query stored memories for injection and SHALL send the request without a memory section

### Requirement: Memory management
The system SHALL let a user list an agent's stored memories, delete one at a time, and delete all of an agent's stored memories in a single action.

#### Scenario: List an agent's memories
- **WHEN** a user requests an agent's stored memories
- **THEN** the system SHALL return them with their content, source, and creation time

#### Scenario: Delete a memory
- **WHEN** a user deletes a stored memory
- **THEN** the system SHALL remove it
- **AND** it SHALL no longer be injected into future generations

#### Scenario: Reset all of an agent's memories
- **WHEN** a user confirms resetting an agent's memories
- **THEN** the system SHALL remove every stored memory for that agent across all of its folder scopes
- **AND** none of them SHALL be injected into future generations

#### Scenario: Reset is scoped to one agent
- **WHEN** a user resets one agent's memories
- **THEN** stored memories belonging to any other agent SHALL remain unaffected

### Requirement: Web runtime memory toggle parity
Unlike custom instructions (whose assembled content has no observable effect in mock mode), the memory enablement toggle and the tool-assisted chat extraction toggle each gate a distinct, user-observable mock event (a simulated `remember` tool call, a simulated automatic-extraction card, and a simulated memory-injection card) in the Web/mock chat stream. The Web/mock runtime SHALL respect both toggles when deciding whether to simulate these events, rather than simulating them unconditionally.

#### Scenario: Memory disabled suppresses simulated memory events
- **WHEN** the memory enablement toggle is off during a mock generation
- **THEN** the Web adapter SHALL NOT simulate a `remember` tool call, an automatic-extraction event, or a memory-injection event for that generation

#### Scenario: Tool-assisted sub-toggle only gates simulated automatic extraction
- **WHEN** the memory enablement toggle is on and the tool-assisted chat extraction toggle is off during a mock generation
- **THEN** the Web adapter SHALL still simulate the explicit `remember` tool call
- **AND** it SHALL NOT simulate the automatic-extraction event

#### Scenario: Web mock memory behaviors
- **WHEN** a mock generation would trigger the explicit tool, automatic extraction, or injection, and both toggles allow it
- **THEN** the Web adapter SHALL simulate the corresponding behavior through the same event and service contracts the desktop runtime uses
- **AND** it SHALL NOT call a real provider to produce it
