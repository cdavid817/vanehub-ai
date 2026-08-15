## MODIFIED Requirements

### Requirement: Chat input submits user messages
The system SHALL allow the user to submit a non-empty text message from the main chat input for the active session through the frontend agent service, except when the submitted input is consumed by the slash command runtime.

#### Scenario: Submit non-empty message
- **WHEN** an active session is selected and the chat input contains non-whitespace text
- **THEN** submitting the input SHALL send the message through the frontend agent service
- **AND** the submitted user message SHALL appear in the active session message list
- **AND** the input SHALL be cleared

#### Scenario: Do not submit empty message
- **WHEN** the chat input is empty or contains only whitespace
- **THEN** the send action SHALL be disabled or ignored
- **AND** no message SHALL be sent

#### Scenario: Preserve IME composition
- **WHEN** the user presses Enter while native IME composition is active
- **THEN** the system SHALL NOT submit the message
- **AND** the input composition SHALL continue normally

#### Scenario: Command-shaped input is not a message
- **WHEN** the active session has slash commands enabled and the submitted input is recognised as a command by the slash command runtime
- **THEN** the system SHALL NOT send the input through the frontend agent service
- **AND** no user message SHALL appear in the active session message list
- **AND** the input SHALL be cleared

#### Scenario: Command-shaped input in an ineligible session stays a message
- **WHEN** the active session does not have slash commands enabled and the submitted input begins with a slash
- **THEN** submitting the input SHALL send the message through the frontend agent service unchanged
