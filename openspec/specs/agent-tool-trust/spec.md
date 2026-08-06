# agent-tool-trust Specification

## Purpose
TBD - created by archiving change add-agent-tool-trust. Update Purpose after archive.
## Requirements
### Requirement: Persistent per-agent tool trust setting
The system SHALL persist a per-agent boolean setting controlling whether that native API agent's `shell`, file-write, and file-edit tool calls require per-call approval, defaulting to requiring approval for every existing and newly registered agent.

#### Scenario: New agents default to requiring approval
- **WHEN** a native API agent is registered
- **THEN** its shell, file-write, and file-edit tool calls SHALL require approval until the trust setting is explicitly enabled for that agent

#### Scenario: Setting persists across sessions
- **WHEN** the trust setting is enabled for an agent
- **THEN** every subsequent session with that agent, regardless of project, SHALL reflect the enabled setting until it is explicitly disabled

### Requirement: MCP tool calls remain unconditionally gated regardless of trust
The system SHALL require approval for every MCP-sourced tool call for a native API agent regardless of that agent's trust setting.

#### Scenario: Trusted agent still requires approval for an MCP tool
- **WHEN** a native API agent with the trust setting enabled requests an MCP-sourced tool call
- **THEN** the system SHALL still require approval before executing it

### Requirement: Plan mode overrides the trust setting unconditionally
The system SHALL reject a trusted agent's shell call, file write, or file edit during plan mode exactly as it would for an untrusted agent, with the trust setting having no effect.

#### Scenario: Plan mode still blocks a trusted agent's shell call
- **WHEN** a native API agent with the trust setting enabled is generating in plan mode and requests a shell call
- **THEN** the system SHALL reject the call, consistent with plan mode's existing behavior for an untrusted agent

#### Scenario: Plan mode still blocks a trusted agent's file edit
- **WHEN** a native API agent with the trust setting enabled is generating in plan mode and requests a file-edit tool call
- **THEN** the system SHALL reject the call, consistent with plan mode's existing behavior for an untrusted agent

### Requirement: Enabling the trust setting requires explicit confirmation
The system SHALL require an explicit, distinct confirmation step before enabling the trust setting for an agent, describing what is being granted. Disabling it SHALL NOT require confirmation.

#### Scenario: Enabling shows a warning before taking effect
- **WHEN** a user chooses to enable the trust setting for an agent
- **THEN** the system SHALL present a confirmation describing that the agent will run shell commands and modify files without per-call approval, in every future session
- **AND** SHALL NOT enable the setting unless the user confirms

#### Scenario: Disabling takes effect immediately
- **WHEN** a user disables the trust setting for an agent
- **THEN** the system SHALL disable it without requiring confirmation

### Requirement: Web runtime trust-setting parity
The Web/mock runtime SHALL simulate the trust setting's effect on the shell tool-call simulation through the same service contracts the desktop runtime uses.

#### Scenario: Mock trusted agent skips the simulated approval step
- **WHEN** a user exercises the simulated tool-call sequence in Web/mock mode for an agent with the mock trust setting enabled
- **THEN** the simulated shell tool call SHALL complete without the simulated approval step

### Requirement: A trusted agent's shell, file-write, and edit calls skip approval
The system SHALL execute a trusted native API agent's `shell` calls, file tool `write` operations, and file-edit calls immediately, without prompting for approval, while leaving every other tool's approval behavior unchanged.

#### Scenario: Trusted agent runs a shell command without a prompt
- **WHEN** a native API agent with the trust setting enabled requests a shell tool call
- **THEN** the system SHALL execute it immediately without an approval prompt

#### Scenario: Trusted agent writes a file without a prompt
- **WHEN** a native API agent with the trust setting enabled requests a file tool call with a write operation
- **THEN** the system SHALL execute it immediately without an approval prompt

#### Scenario: Trusted agent edits a file without a prompt
- **WHEN** a native API agent with the trust setting enabled requests a file-edit tool call
- **THEN** the system SHALL execute it immediately without an approval prompt

#### Scenario: Untrusted agent is unaffected
- **WHEN** a native API agent without the trust setting enabled requests a shell call, a file write, or a file edit
- **THEN** the system SHALL require approval exactly as it did before this capability existed

