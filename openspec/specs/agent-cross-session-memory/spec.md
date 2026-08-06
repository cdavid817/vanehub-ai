# agent-cross-session-memory Specification

## Purpose
TBD - created by archiving change add-agent-cross-session-memory. Update Purpose after archive.
## Requirements
### Requirement: Memory scoping
The system SHALL treat stored memories as a single host-level pool shared by every agent — OnePiece and all CLI-wrapped agents (`claude-code`, `codex-cli`, `gemini-cli`, `opencode`) alike — rather than scoping them to the agent or workspace folder that produced them. The system SHALL record which agent and, when available, which workspace folder produced each memory as provenance metadata on the stored record, without using either as a filter for injection, listing, or management.

#### Scenario: Memory scoped to agent and folder
- **WHEN** a memory is saved during a session with a workspace folder, whether by OnePiece's explicit tool, OnePiece's automatic extraction, or a CLI-wrapped agent's automatic extraction
- **THEN** the system SHALL record the producing agent id and that workspace folder as provenance metadata alongside the memory content
- **AND**, unlike before `add-cli-memory-support`, neither value SHALL restrict which future generations or management views can read that memory

#### Scenario: Memory scoped to agent only when no folder is available
- **WHEN** a memory is saved during a session with no workspace folder
- **THEN** the system SHALL still save it into the shared pool, recording no folder rather than rejecting the save
- **AND** a generation or management view operating in any workspace folder, or with no workspace folder at all, SHALL still be able to read, inject, and manage it

#### Scenario: Memories do not cross agents
- **WHEN** two different agents produce memories, whether or not they share a workspace folder
- **THEN**, reversing the isolation this scenario previously guaranteed, the system SHALL make each agent's memories visible to every other agent's generations and management views via the shared host-level pool, exactly as if they had produced it themselves

### Requirement: Explicit memory saving
The system SHALL provide a tool the model can call to save a memory, auto-approved without requiring user confirmation, while the memory enablement toggle is on. This tool is exposed only to OnePiece's own API tool-calling loop; CLI-wrapped agents produce memories through the separate mechanism governed by the "Automatic memory extraction for CLI-wrapped agents" requirement, since VaneHub does not control a CLI-wrapped agent's own internal tool system.

#### Scenario: Model saves a memory via the tool
- **WHEN** the model calls the memory-saving tool with content during a generation and the memory enablement toggle is on
- **THEN** the system SHALL persist it immediately without requiring approval
- **AND** it SHALL be available to future sessions for every agent, per the shared host-level pool

#### Scenario: Tool is inert when memory is disabled
- **WHEN** the model calls the memory-saving tool while the memory enablement toggle is off
- **THEN** the system SHALL reject the call without persisting anything

### Requirement: Automatic memory extraction
The system SHALL attempt best-effort automatic extraction of memorable content from turns that context compaction is about to replace, without failing the generation if extraction fails, while the memory enablement toggle is on and, when the compacted turns include a tool call, only while the tool-assisted chat extraction toggle is also on. This requirement governs only OnePiece's compaction-triggered extraction; CLI-wrapped agents have no VaneHub-visible compaction signal and are governed instead by the separate "Automatic memory extraction for CLI-wrapped agents" requirement.

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
The system SHALL inject the shared host-level memory pool into OnePiece's generation requests as part of the system prompt while the memory enablement toggle is on, bounded by a character budget, and SHALL never write memory content into the turns list context compaction manipulates. This requirement governs only OnePiece's system-prompt injection; CLI-wrapped agents are governed instead by the separate "Memory injection into CLI prompts" requirement.

#### Scenario: Memories injected alongside Skill content
- **WHEN** a generation runs for an agent with both bound Skills and stored memories in scope, and the memory enablement toggle is on
- **THEN** the system prompt SHALL include both, as distinct sections

#### Scenario: Injected memories are bounded
- **WHEN** the shared memory pool exceeds the injection character budget
- **THEN** the system SHALL include memories by recency up to the budget rather than including all of them unbounded

#### Scenario: Memory content survives compaction
- **WHEN** context compaction triggers during a generation with injected memory content
- **THEN** the injected memory content SHALL remain present, complete, and unchanged on every subsequent request of that generation

#### Scenario: No injection when memory is disabled
- **WHEN** the memory enablement toggle is off
- **THEN** the system SHALL NOT query stored memories for injection and SHALL send the request without a memory section

### Requirement: Memory management
The system SHALL let a user list every stored memory in the shared host-level pool, delete one at a time, and delete every stored memory in a single action, regardless of which agent produced them.

#### Scenario: List an agent's memories
- **WHEN** a user requests the stored memories
- **THEN**, unlike before `add-cli-memory-support` when this returned only the requesting agent's own memories, the system SHALL return every stored memory in the shared pool, each with its content, source, producing agent, and creation time

#### Scenario: Delete a memory
- **WHEN** a user deletes a stored memory
- **THEN** the system SHALL remove it
- **AND** it SHALL no longer be injected into any agent's future generations

#### Scenario: Reset all of an agent's memories
- **WHEN** a user confirms resetting stored memories
- **THEN** the system SHALL remove every stored memory in the shared pool, regardless of which agent produced it
- **AND** none of them SHALL be injected into any agent's future generations
- **AND** the confirmation prompt SHALL make clear that this affects every agent's memories, not only the one currently being viewed

#### Scenario: Reset is scoped to one agent
- **WHEN** a user confirms resetting stored memories
- **THEN**, reversing the single-agent scoping this scenario previously guaranteed, stored memories belonging to every other agent SHALL be removed too — reset is no longer scoped to one agent, since there is no longer a per-agent scope to reset

### Requirement: Web runtime memory toggle parity
Unlike custom instructions (whose assembled content has no observable effect in mock mode), the memory enablement toggle and the tool-assisted chat extraction toggle each gate a distinct, user-observable mock event (a simulated `remember` tool call, a simulated automatic-extraction card, and a simulated memory-injection card) in the Web/mock chat stream. The Web/mock runtime SHALL respect the memory enablement toggle for every agent kind — OnePiece and CLI-wrapped agents alike — and SHALL additionally respect the tool-assisted chat extraction toggle for OnePiece's own simulated automatic-extraction event, mirroring that this sub-toggle governs only OnePiece's compaction-triggered extraction and never a CLI-wrapped agent's independent extraction. The Web/mock runtime SHALL NOT simulate these events unconditionally or only for API-kind sessions.

#### Scenario: Memory disabled suppresses simulated memory events
- **WHEN** the memory enablement toggle is off during a mock generation, whether for OnePiece or a CLI-wrapped agent
- **THEN** the Web adapter SHALL NOT simulate a `remember`-equivalent event, an automatic-extraction event, or a memory-injection event for that generation

#### Scenario: Tool-assisted sub-toggle only gates simulated automatic extraction
- **WHEN** the memory enablement toggle is on and the tool-assisted chat extraction toggle is off during a mock generation for OnePiece
- **THEN** the Web adapter SHALL still simulate the explicit-save-equivalent event
- **AND** it SHALL NOT simulate OnePiece's automatic-extraction event
- **AND**, extending this scenario beyond OnePiece, this sub-toggle SHALL have no effect on a CLI-wrapped agent's simulated automatic-extraction event, mirroring that the sub-toggle governs only OnePiece's own compaction-triggered extraction

#### Scenario: Web mock memory behaviors
- **WHEN** a mock generation would trigger the explicit tool, automatic extraction, or injection, and both applicable toggles allow it
- **THEN** the Web adapter SHALL simulate the corresponding behavior through the same event and service contracts the desktop runtime uses
- **AND** it SHALL NOT call a real provider to produce it

#### Scenario: CLI-wrapped agent mock sessions simulate automatic extraction
- **WHEN** a mock generation for a CLI-wrapped agent completes and the memory enablement toggle is on
- **THEN** the Web adapter SHALL simulate an automatic-extraction event and persist a mock memory attributed to that agent, mirroring the desktop runtime's CLI-completion-triggered extraction
- **AND** this simulated event, like the desktop runtime's CLI extraction it mirrors, SHALL NOT depend on the tool-assisted chat extraction toggle, since that toggle governs only OnePiece's own compaction-triggered extraction

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

### Requirement: Memory injection into CLI prompts
The system SHALL prepend the shared host-level memory pool to the Prompt-Hook-assembled effective prompt for every message sent to a CLI-wrapped agent (`claude-code`, `codex-cli`, `gemini-cli`, `opencode`), after the custom-instructions section and before the Prompt Hook pipeline's own assembled content, while the memory enablement toggle is on. This requirement governs only the CLI delivery mechanism; OnePiece's own system-prompt injection remains governed by the "Memory injection into the system prompt" requirement.

#### Scenario: Memory section precedes the Prompt Hook assembly
- **WHEN** a message is sent to a CLI-wrapped agent with the memory enablement toggle on and at least one memory in the shared pool
- **THEN** the final text delivered to that CLI process SHALL contain the memory section after the custom-instructions section (if any) and before the Prompt-Hook-assembled content

#### Scenario: Disabled or empty produces no injection
- **WHEN** the memory enablement toggle is off, or the shared memory pool is empty
- **THEN** the text delivered to the CLI process SHALL be unchanged by this requirement

#### Scenario: Injection query failure does not block the CLI message
- **WHEN** resolving memories fails while sending a message to a CLI-wrapped agent
- **THEN** the system SHALL log the failure and send the message without the memory section
- **AND** it SHALL NOT fail or delay the message send

### Requirement: Automatic memory extraction for CLI-wrapped agents
The system SHALL, after a CLI-wrapped agent's generation completes successfully and while the memory enablement toggle is on, attempt best-effort automatic extraction of memorable content from that turn's exchange by making an independent model call using OnePiece's currently configured provider credentials. This extraction SHALL NOT block or delay delivery of the CLI generation's own result to the user, and SHALL NOT depend on or alter OnePiece's own compaction-triggered extraction mechanism.

#### Scenario: Extraction runs after a CLI generation completes
- **WHEN** a CLI-wrapped agent's generation completes successfully, the memory enablement toggle is on, and OnePiece has a usable configured credential
- **THEN** the system SHALL make one independent call to extract memorable facts from that turn's exchange
- **AND** any extracted facts SHALL be persisted in the shared host-level memory pool, attributed to the CLI-wrapped agent that produced them

#### Scenario: Extraction finds nothing worth remembering
- **WHEN** the extraction call determines there is nothing worth remembering from that turn
- **THEN** the system SHALL save no memory from that extraction, without treating this as a failure

#### Scenario: Extraction is skipped without a usable OnePiece credential
- **WHEN** a CLI-wrapped agent's generation completes and OnePiece has no usable configured credential
- **THEN** the system SHALL log the condition and skip extraction for that turn
- **AND** it SHALL NOT affect the CLI generation, which has already completed and been delivered

#### Scenario: Extraction call failure does not affect the already-delivered CLI result
- **WHEN** the independent extraction call itself fails
- **THEN** the system SHALL log the failure and skip persisting a memory for that turn
- **AND** the CLI generation's own result, already delivered to the user, SHALL be unaffected

#### Scenario: Extraction is skipped when memory is disabled
- **WHEN** the memory enablement toggle is off
- **THEN** the system SHALL NOT make an extraction call after a CLI-wrapped agent's generation completes

