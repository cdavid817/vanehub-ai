## ADDED Requirements

### Requirement: Memory scoping
The system SHALL scope stored memories to the agent that produced them and, when available, the session's workspace folder, and SHALL fall back to an agent-global scope when no folder is available.

#### Scenario: Memory scoped to agent and folder
- **WHEN** a memory is saved during a session with a workspace folder
- **THEN** the system SHALL store it against that agent id and folder

#### Scenario: Memory scoped to agent only when no folder is available
- **WHEN** a memory is saved during a session with no workspace folder
- **THEN** the system SHALL store it in an agent-global scope rather than rejecting the save

#### Scenario: Memories do not cross agents
- **WHEN** two different agents share the same workspace folder
- **THEN** neither agent's memories SHALL be visible to the other

### Requirement: Explicit memory saving
The system SHALL provide a tool the model can call to save a memory, auto-approved without requiring user confirmation.

#### Scenario: Model saves a memory via the tool
- **WHEN** the model calls the memory-saving tool with content during a generation
- **THEN** the system SHALL persist it immediately without requiring approval
- **AND** it SHALL be available to future sessions in the same scope

### Requirement: Automatic memory extraction
The system SHALL attempt best-effort automatic extraction of memorable content from turns that context compaction is about to replace, without failing the generation if extraction fails.

#### Scenario: Extraction runs when compaction triggers
- **WHEN** context compaction triggers during a generation
- **THEN** the system SHALL make one additional call to extract memorable facts from the turns being compacted
- **AND** any extracted facts SHALL be persisted in the same store the explicit tool writes to

#### Scenario: Extraction finds nothing worth remembering
- **WHEN** the extraction call determines there is nothing worth remembering long-term
- **THEN** the system SHALL save no memories from that extraction, without treating this as a failure

#### Scenario: Extraction failure does not affect compaction
- **WHEN** the extraction call itself fails
- **THEN** the system SHALL log the failure and continue the generation and its compaction unaffected

### Requirement: Memory injection into the system prompt
The system SHALL inject an agent's scoped memories into its generation requests as part of the system prompt, bounded by a character budget, and SHALL never write memory content into the turns list context compaction manipulates.

#### Scenario: Memories injected alongside Skill content
- **WHEN** a generation runs for an agent with both bound Skills and stored memories in scope
- **THEN** the system prompt SHALL include both, as distinct sections

#### Scenario: Injected memories are bounded
- **WHEN** an agent's scoped memories exceed the injection character budget
- **THEN** the system SHALL include memories by recency up to the budget rather than including all of them unbounded

#### Scenario: Memory content survives compaction
- **WHEN** context compaction triggers during a generation with injected memory content
- **THEN** the injected memory content SHALL remain present, complete, and unchanged on every subsequent request of that generation

### Requirement: Memory management
The system SHALL let a user list and delete an agent's stored memories.

#### Scenario: List an agent's memories
- **WHEN** a user requests an agent's stored memories
- **THEN** the system SHALL return them with their content, source, and creation time

#### Scenario: Delete a memory
- **WHEN** a user deletes a stored memory
- **THEN** the system SHALL remove it
- **AND** it SHALL no longer be injected into future generations

### Requirement: Web runtime memory parity
The Web/mock runtime SHALL simulate the explicit save, automatic extraction, and injection behaviors deterministically without a real provider call.

#### Scenario: Web mock memory behaviors
- **WHEN** a mock generation would trigger the explicit tool, automatic extraction, or injection
- **THEN** the Web adapter SHALL simulate the corresponding behavior through the same event and service contracts the desktop runtime uses
- **AND** it SHALL NOT call a real provider to produce it
