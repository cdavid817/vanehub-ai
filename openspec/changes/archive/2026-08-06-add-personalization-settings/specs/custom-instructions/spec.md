# custom-instructions Specification (Delta)

## ADDED Requirements

### Requirement: Custom instructions configuration and persistence
The system SHALL provide host-level custom instructions for OnePiece, containing two independent text fields ("about you" and "response style") and one enablement toggle, persisted through the shared application settings model rather than a dedicated table, and SHALL NOT transmit their content to any remote service beyond the OnePiece generation requests they are injected into. Each field SHALL be limited to 3,000 Unicode characters; a value exceeding that limit SHALL be rejected rather than persisted.

#### Scenario: Save custom instructions for the first time
- **WHEN** a user fills in the about-you and/or response-style fields and saves
- **THEN** the system SHALL persist both fields and the enabled state through the settings service
- **AND** the saved values SHALL apply to subsequently started OnePiece generations without an application restart

#### Scenario: Load default custom instructions
- **WHEN** no custom instructions have been saved yet
- **THEN** the system SHALL treat both fields as empty and the enabled toggle as on

#### Scenario: Oversized field is rejected
- **WHEN** a user submits a field exceeding 3,000 Unicode characters
- **THEN** the settings UI SHALL prevent saving and show the used/remaining character count
- **AND** the native command layer SHALL independently validate and reject an oversized value, defense in depth against a non-UI caller

### Requirement: Custom instructions system-prompt section assembly
The system SHALL assemble enabled, non-empty custom instructions into one distinct system-prompt section, with response style ordered before about-you within that section. When custom instructions are disabled or both fields are empty, the system SHALL produce no section and SHALL skip any related lookup. This requirement governs only the internal shape of the custom-instructions section; its position relative to core instructions, Skills, and memory is governed by the `agent-skill-injection` capability.

#### Scenario: Both fields present
- **WHEN** a OnePiece generation starts with custom instructions enabled and both fields non-empty
- **THEN** the assembled section SHALL present response style before about-you

#### Scenario: Disabled produces no section
- **WHEN** the custom instructions enabled toggle is off
- **THEN** the generation request SHALL contain no custom-instructions section

#### Scenario: Only one field is populated
- **WHEN** only one of the two fields is non-empty
- **THEN** the assembled section SHALL contain only that field's content, with no empty placeholder for the other

#### Scenario: Settings lookup fails
- **WHEN** resolving custom instructions fails during generation
- **THEN** the system SHALL log the failure and omit the custom-instructions section
- **AND** it SHALL NOT fail the generation or affect independently resolved core-instruction, Skill, or memory sections

### Requirement: Web runtime custom instructions parity
The Web/mock runtime SHALL simulate custom instructions persistence deterministically without a real provider call. Because the Web/mock `sendMessage` simulation does not model provider-bound prompt content (its simulated responses are fixed templates, not a function of assembled system-prompt content), this requirement governs only settings persistence and loading — not simulating system-prompt assembly itself, which has no user-observable effect to simulate in mock mode.

#### Scenario: Web mock custom instructions settings behavior
- **WHEN** custom instructions are saved or loaded through the Web/mock adapter
- **THEN** the Web adapter SHALL simulate the corresponding persistence behavior through the same service contracts the desktop runtime uses
- **AND** it SHALL NOT access SQLite or contact a real provider to produce it

#### Scenario: No simulated prompt-content divergence
- **WHEN** custom instructions are enabled and non-empty during a mock generation
- **THEN** the Web adapter's simulated response SHALL behave identically to when custom instructions are disabled or empty
- **AND** this SHALL NOT be treated as a parity gap, since the desktop runtime's own assembled system-prompt content is equally not observable through the chat UI
