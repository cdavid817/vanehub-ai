## ADDED Requirements

### Requirement: CLI chat applies Prompt Hooks before provider invocation
The desktop CLI chat runtime SHALL assemble enabled Prompt Hooks into the effective prompt before launching a provider CLI process.

#### Scenario: Apply hooks for bound CLI
- **WHEN** a user sends a message to an active non-archived CLI session whose stable agent id has enabled Prompt Hooks bound to it
- **THEN** the desktop runtime SHALL assemble those hooks with the user content before provider invocation
- **AND** the provider-specific invocation builder SHALL receive the assembled effective prompt

#### Scenario: Skip unbound hooks
- **WHEN** a Prompt Hook is not bound to the active session's stable agent id
- **THEN** the desktop runtime SHALL skip that hook for the invocation

#### Scenario: Preserve original user message
- **WHEN** Prompt Hooks are applied to a chat invocation
- **THEN** the persisted and displayed user message SHALL remain the original trimmed user input
- **AND** the assembled effective prompt SHALL NOT replace the user-visible message content

#### Scenario: Hook assembly failure
- **WHEN** Prompt Hook assembly fails during chat send
- **THEN** the user message SHALL remain persisted
- **AND** the assistant message SHALL be marked `failed` with a concise user-facing error
- **AND** detailed redacted diagnostics SHALL be written through unified logging

### Requirement: Web runtime preserves Prompt Hook chat parity
The Web/mock runtime SHALL preserve the Prompt Hook chat contract without claiming native CLI execution.

#### Scenario: Web mock applies deterministic hook preview
- **WHEN** the Web/mock adapter sends a mock chat message with enabled Prompt Hooks
- **THEN** it SHALL use deterministic Prompt Hook assembly behavior for mock response metadata or trace behavior
- **AND** it SHALL preserve the same original user-message display semantics as desktop mode
