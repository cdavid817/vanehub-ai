## ADDED Requirements

### Requirement: Native Agent terminal ownership
The Rust native runtime SHALL own Agent Terminal launch, attach, input, resize, stop, idle cleanup, shutdown cleanup, and diagnostics behind bounded context application use cases and Tauri command adapters.

#### Scenario: Handle terminal command
- **WHEN** a frontend adapter requests Agent Terminal start, attach, input, resize, stop, or event subscription
- **THEN** the Tauri command layer SHALL map the transport request into an application use case
- **AND** it SHALL NOT construct shell commands, execute SQL, or decide Agent runtime policy in the command handler

#### Scenario: Keep React isolated
- **WHEN** React UI code renders or controls the Agent Terminal
- **THEN** it SHALL call the frontend service interface
- **AND** Tauri `invoke()` usage SHALL remain inside Tauri-specific frontend adapters

### Requirement: Native shell wrapper safety
The native runtime SHALL construct Agent Terminal shell wrappers without frontend-supplied shell strings and SHALL record only redacted command diagnostics.

#### Scenario: Generate wrapper from validated tokens
- **WHEN** an Agent Terminal process is launched
- **THEN** the native runtime SHALL resolve the CLI executable and validated argument tokens before writing or invoking a shell wrapper
- **AND** it SHALL NOT accept an arbitrary shell command string from React components

#### Scenario: Redact launch diagnostics
- **WHEN** the native runtime records Agent Terminal launch diagnostics
- **THEN** it SHALL redact prompts, credentials, tokens, secret-like values, and sensitive runtime context before persistence
- **AND** it SHALL write diagnostics through the unified logging service

### Requirement: Agent terminal registry cleanup
The native runtime SHALL maintain a bounded registry of live Agent Terminal processes and clean it up deterministically.

#### Scenario: One live terminal per session
- **WHEN** a start request targets a session with an existing live Agent Terminal process
- **THEN** the native runtime SHALL attach to the existing process rather than spawning a second Agent CLI process for that session

#### Scenario: Cleanup idle terminal
- **WHEN** a live Agent Terminal process exceeds the configured 30-minute inactivity threshold
- **THEN** the native runtime SHALL stop the process and remove its live registry entry
- **AND** it SHALL preserve persisted session metadata needed for later resume

#### Scenario: Cleanup on shutdown
- **WHEN** the desktop runtime begins application shutdown
- **THEN** it SHALL stop all live Agent Terminal processes before shutdown completes when possible
- **AND** cleanup failures SHALL be logged through unified logging with redaction
