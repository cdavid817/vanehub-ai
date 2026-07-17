## ADDED Requirements

### Requirement: Desktop CLI chat streams provider runtime output
The desktop runtime SHALL stream assistant output from provider-specific Agent CLI execution for CLI sessions instead of parsing only after command completion.

#### Scenario: Stream provider CLI stdout
- **WHEN** a user sends a message to an active non-archived session whose interaction mode is `cli`
- **THEN** the desktop runtime SHALL start a provider-specific CLI invocation for the session's stable agent id
- **AND** stdout events SHALL be normalized into `started`, `token`, `thinking`, `tool_use`, `completed`, `failed`, or `cancelled` chat events for that session
- **AND** token events SHALL be emitted as output becomes available rather than only after process exit

#### Scenario: Use provider-specific command path
- **WHEN** the active session references `claude-code`, `codex-cli`, `gemini-cli`, or `opencode`
- **THEN** the desktop runtime SHALL build the CLI invocation using that provider's supported headless command contract
- **AND** it SHALL NOT rely on a single generic `executable prompt` command shape for all providers

#### Scenario: Prefer stdin for prompt delivery
- **WHEN** a provider CLI supports reading the prompt from stdin
- **THEN** the desktop runtime SHALL send the prompt through stdin instead of placing the full prompt in process arguments
- **AND** command audit logs SHALL redact prompt content

### Requirement: Desktop CLI chat persists streamed content
The desktop runtime SHALL persist streamed assistant content and terminal status for CLI chat generations.

#### Scenario: Persist streamed assistant content
- **WHEN** a provider CLI emits token output for an assistant message
- **THEN** the desktop runtime SHALL append the emitted content to the persisted assistant message
- **AND** the visible chat event stream SHALL match the persisted message content after refresh

#### Scenario: Persist terminal runtime outcome
- **WHEN** the provider CLI exits successfully after streamed output
- **THEN** the assistant message SHALL be marked `completed`
- **AND** token usage SHALL be persisted when provider metadata is available

#### Scenario: Persist failed runtime outcome
- **WHEN** the provider CLI fails to start, exits unsuccessfully, or emits a structured error event
- **THEN** the user message SHALL remain persisted
- **AND** the assistant message SHALL be marked `failed` with a concise user-facing error
- **AND** detailed diagnostics SHALL be written through unified logging
