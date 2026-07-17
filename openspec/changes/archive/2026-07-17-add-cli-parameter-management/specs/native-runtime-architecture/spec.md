## ADDED Requirements

### Requirement: Native CLI parameter persistence
The native runtime SHALL persist validated CLI parameter selections in a dedicated SQLite table through an additive migration and bounded repository commands.

#### Scenario: Migrate existing database
- **WHEN** the native runtime opens an empty or older VaneHub database
- **THEN** it SHALL add CLI parameter storage without deleting or rewriting existing agents, settings, sessions, messages, CLI statuses, or skills

#### Scenario: Save profile transaction
- **WHEN** the native save command receives a complete valid profile for a managed agent id
- **THEN** it SHALL validate every selection against the native catalog and commit the profile atomically

#### Scenario: Reject invalid profile transaction
- **WHEN** any submitted selection has an unknown id, wrong value type, unsupported value, reserved conflict, or control character
- **THEN** the native runtime SHALL reject the complete mutation
- **AND** it SHALL retain the previously committed profile

### Requirement: Native provider argument composition
The native runtime SHALL keep CLI parameter conversion and token placement inside provider-specific launch builders keyed by stable agent id.

#### Scenario: Compose without shell interpolation
- **WHEN** a provider invocation includes saved or per-message values
- **THEN** the native runtime SHALL pass each executable argument as a distinct process argument
- **AND** it SHALL NOT construct or execute a shell command string from those values

#### Scenario: Preserve required runtime tokens
- **WHEN** user-controlled selections are composed with a provider invocation
- **THEN** provider subcommands, structured output, session/resume, prompt delivery, and stdin tokens SHALL remain native-runtime controlled

### Requirement: Native profile diagnostics use unified logging
The native runtime SHALL report profile validation, compatibility, persistence, and provider-rejection diagnostics through unified logging with redaction.

#### Scenario: Persist profile diagnostic
- **WHEN** loading, saving, resetting, or applying a profile produces a warning or error
- **THEN** the native runtime SHALL write a redacted entry with the stable agent id and parameter id when available
- **AND** it SHALL NOT write a feature-local log file

#### Scenario: Audit launched arguments
- **WHEN** a provider process is launched with effective parameters
- **THEN** command diagnostics SHALL redact prompts, credentials, tokens, secret-like values, and sensitive runtime context before persistence

### Requirement: CLI parameter commands remain bounded
Native list, save, and reset commands SHALL perform catalog validation and small SQLite operations only and SHALL NOT probe executables, access networks, or wait for provider processes.

#### Scenario: Return profile mutation directly
- **WHEN** a valid save or reset request is handled
- **THEN** the bounded Tauri command MAY return the normalized profile directly without creating a long-running operation

