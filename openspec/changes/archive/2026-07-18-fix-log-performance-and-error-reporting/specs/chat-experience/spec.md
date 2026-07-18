## ADDED Requirements

### Requirement: Main-window chat operation failure reporting
The main chat surface SHALL show localized feedback and report durable diagnostics through the frontend service boundary when a chat send, stop, or configuration-persistence operation fails.

#### Scenario: Chat send or stop request fails
- **WHEN** the main-window send or stop request reaches a terminal service failure
- **THEN** the chat surface SHALL show a localized user-displayable error without clearing unrelated loaded messages
- **AND** it SHALL report a `critical-operation-failure` event through the settings service boundary

#### Scenario: Configuration persistence fails
- **WHEN** saving a changed session chat configuration fails
- **THEN** the chat surface SHALL show a localized user-displayable error
- **AND** it SHALL report a `critical-operation-failure` event through the settings service boundary

#### Scenario: Web runtime reports a chat failure
- **WHEN** the app runs through the Web/mock adapter and reports a chat operation failure
- **THEN** it SHALL preserve the same visible feedback and service call
- **AND** it SHALL NOT write a local log file
