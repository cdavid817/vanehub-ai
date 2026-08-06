# custom-instructions Specification (Delta)

## ADDED Requirements

### Requirement: Custom instructions CLI prompt injection
The system SHALL prepend enabled, non-empty custom instructions to the Prompt-Hook-assembled effective prompt for every message sent to a CLI-wrapped agent (`claude-code`, `codex-cli`, `gemini-cli`, `opencode`), using the same section formatting as the OnePiece system-prompt section. This requirement governs only the CLI delivery mechanism; the Prompt Hook pipeline's own assembly, bindings, and template rendering are unaffected and remain governed by the `native-runtime-architecture` and Prompt Hook specifications.

#### Scenario: Custom instructions precede the Prompt Hook assembly
- **WHEN** a message is sent to a CLI-wrapped agent with custom instructions enabled and non-empty
- **THEN** the final text delivered to that CLI process SHALL contain the custom-instructions section before the Prompt-Hook-assembled content

#### Scenario: Repeated on every turn
- **WHEN** a CLI-wrapped agent session sends more than one message
- **THEN** the system SHALL prepend the custom-instructions section to each message independently, not only the first

#### Scenario: Disabled or empty produces no injection
- **WHEN** custom instructions are disabled or both fields are empty
- **THEN** the text delivered to the CLI process SHALL be exactly the Prompt-Hook-assembled content, unchanged from behavior before this requirement existed

#### Scenario: Settings lookup failure does not block the CLI message
- **WHEN** resolving custom instructions fails while sending a message to a CLI-wrapped agent
- **THEN** the system SHALL log the failure and send the Prompt-Hook-assembled content without the custom-instructions section
- **AND** it SHALL NOT fail or delay the message send

#### Scenario: Does not alter Prompt Hook template rendering
- **WHEN** custom instructions are prepended for a CLI-wrapped agent
- **THEN** the Prompt Hook pipeline's own template variables (including the rendered user message) SHALL reflect only the user's original input, unaffected by the prepended custom-instructions content

## MODIFIED Requirements

### Requirement: Custom instructions configuration and persistence
The system SHALL provide host-level custom instructions applied to OnePiece and to the CLI-wrapped agents (`claude-code`, `codex-cli`, `gemini-cli`, `opencode`), containing two independent text fields ("about you" and "response style") and one enablement toggle, persisted through the shared application settings model rather than a dedicated table, and SHALL NOT transmit their content to any remote service beyond the generation or CLI prompt requests they are injected into. Each field SHALL be limited to 3,000 Unicode characters; a value exceeding that limit SHALL be rejected rather than persisted. A single enablement toggle SHALL govern both OnePiece and the CLI-wrapped agents together.

#### Scenario: Save custom instructions for the first time
- **WHEN** a user fills in the about-you and/or response-style fields and saves
- **THEN** the system SHALL persist both fields and the enabled state through the settings service
- **AND** the saved values SHALL apply to subsequently started OnePiece generations and CLI-wrapped agent messages without an application restart

#### Scenario: Load default custom instructions
- **WHEN** no custom instructions have been saved yet
- **THEN** the system SHALL treat both fields as empty and the enabled toggle as on

#### Scenario: Oversized field is rejected
- **WHEN** a user submits a field exceeding 3,000 Unicode characters
- **THEN** the settings UI SHALL prevent saving and show the used/remaining character count
- **AND** the native command layer SHALL independently validate and reject an oversized value, defense in depth against a non-UI caller

#### Scenario: One toggle governs every agent
- **WHEN** a user disables the custom instructions enablement toggle
- **THEN** neither OnePiece nor any CLI-wrapped agent SHALL receive injected custom instructions until the toggle is re-enabled

### Requirement: Web runtime custom instructions parity
The Web/mock runtime SHALL simulate custom instructions persistence deterministically for every agent kind, without a real provider call or a real CLI process. Because the Web/mock `sendMessage` simulation does not model provider- or CLI-bound prompt content for any agent kind (its simulated responses are fixed templates, not a function of assembled prompt content), this requirement governs only settings persistence and loading — not simulating the CLI prepend mechanism itself, which has no user-observable effect to simulate in mock mode.

#### Scenario: Web mock custom instructions settings behavior
- **WHEN** custom instructions are saved or loaded through the Web/mock adapter, regardless of which agent kind will use them
- **THEN** the Web adapter SHALL simulate the corresponding persistence behavior through the same service contracts the desktop runtime uses
- **AND** it SHALL NOT access SQLite, contact a real provider, or launch a real CLI process to produce it

#### Scenario: No simulated prompt-content divergence
- **WHEN** custom instructions are enabled and non-empty during a mock message send to any agent kind
- **THEN** the Web adapter's simulated response SHALL behave identically to when custom instructions are disabled or empty
- **AND** this SHALL NOT be treated as a parity gap, since the desktop runtime's own prepended CLI prompt content is equally not observable through the chat UI
