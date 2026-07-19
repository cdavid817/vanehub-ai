## MODIFIED Requirements

### Requirement: Native Agent terminal diagnostics
The desktop runtime SHALL persist redacted Agent terminal launch diagnostics for both successful starts and startup failures.

#### Scenario: Startup command is recorded
- **WHEN** the desktop runtime starts an Agent terminal process
- **THEN** it SHALL record a redacted startup command log entry associated with the VaneHub session id and Agent id

#### Scenario: Startup failure is recorded
- **WHEN** session validation, Agent lookup, availability validation, CLI profile loading, lifecycle update, invocation construction, wrapper generation, PTY creation, process spawning, reader setup, or writer setup fails
- **THEN** the runtime SHALL record a `session.agent_terminal` failure log entry before returning the error
- **AND** the log SHALL include the VaneHub session id and Agent id
- **AND** sensitive command content SHALL remain redacted before persistence

#### Scenario: Retained terminal attach reports running state
- **WHEN** the desktop runtime attaches to an existing retained Agent terminal process
- **THEN** it SHALL return a terminal session with `running` state
- **AND** it SHALL publish a terminal state event with `running` state
- **AND** the Workspace UI SHALL refresh session state after receiving the attach response or terminal state event

### Requirement: Windows managed CLI executable normalization
The desktop runtime SHALL normalize known Windows package-manager shim executables for managed Agent terminal launches when a concrete package binary can be found.

#### Scenario: Missing managed SDK does not block CLI terminal startup
- **WHEN** a single-Agent CLI session starts an Agent terminal for Claude Code, Codex CLI, or another managed CLI Agent
- **AND** the Agent has a missing managed SDK dependency
- **THEN** the runtime SHALL still load the interactive CLI profile for the selected Agent
- **AND** it SHALL attempt startup through the resolved CLI executable, such as `claude`, `codex`, `gemini`, or `opencode`
- **AND** only CLI executable/profile resolution failures SHALL stop the CLI terminal before process launch

#### Scenario: Known shim has package binary
- **WHEN** a managed CLI executable path points to a Windows `.cmd` or `.ps1` shim for Claude Code, Codex CLI, or OpenCode
- **AND** the corresponding package binary exists next to the shim's `node_modules` installation
- **THEN** the Agent terminal runtime SHALL launch the concrete package binary through the native-owned shell wrapper

#### Scenario: Shim cannot be resolved
- **WHEN** a configured executable is not a known Windows shim or no corresponding package binary exists
- **THEN** the runtime SHALL keep the configured executable unchanged
