## MODIFIED Requirements

### Requirement: Desktop CLI chat streams provider runtime output
The desktop runtime SHALL stream assistant output from provider-specific Agent CLI execution for CLI sessions instead of parsing only after command completion.

#### Scenario: Stream provider CLI stdout
- **WHEN** a user sends a message to an active non-archived session whose interaction mode is `cli`
- **THEN** the desktop runtime SHALL start a provider-specific CLI invocation for the session's stable agent id
- **AND** stdout events SHALL be normalized into `started`, `token`, `thinking`, `tool_use`, `completed`, `failed`, or `cancelled` chat events for that session
- **AND** token events SHALL be emitted as output becomes available rather than only after process exit

#### Scenario: Use provider-specific command path
- **WHEN** the active session references `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, or `antigravity-cli`
- **THEN** the desktop runtime SHALL build the CLI invocation using that provider's supported headless command contract
- **AND** it SHALL NOT rely on a single generic `executable prompt` command shape for all providers

#### Scenario: Prefer stdin for prompt delivery
- **WHEN** a provider CLI supports reading the prompt from stdin
- **THEN** the desktop runtime SHALL send the prompt through stdin instead of placing the full prompt in process arguments
- **AND** command audit logs SHALL redact prompt content
