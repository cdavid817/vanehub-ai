## ADDED Requirements

### Requirement: Native Prompt Hook persistence
The native runtime SHALL persist Prompt Hook overrides, user-created hooks, CLI bindings, and recent trace summaries in SQLite through additive migrations.

#### Scenario: Migrate Prompt Hook storage
- **WHEN** the native runtime opens an empty or older VaneHub database
- **THEN** it SHALL add Prompt Hook storage without deleting or rewriting existing agents, settings, sessions, messages, CLI statuses, Skills, SDK data, MCP data, IM data, or usage records

#### Scenario: Persist hook mutation atomically
- **WHEN** a Prompt Hook mutation updates enabled state, user hook content, metadata, or CLI bindings
- **THEN** the native runtime SHALL validate the complete mutation and commit it atomically

#### Scenario: Reject invalid hook mutation
- **WHEN** a Prompt Hook mutation contains invalid manifest data, unsupported category, unsupported stable agent id, unsafe content, or an immutable built-in edit
- **THEN** the native runtime SHALL reject the complete mutation
- **AND** it SHALL retain the previously committed state

### Requirement: Native Prompt Hook pipeline
The native runtime SHALL provide a provider-agnostic Prompt Hook pipeline before CLI provider invocation.

#### Scenario: Assemble effective prompt
- **WHEN** a CLI chat invocation starts for `claude-code`, `codex-cli`, `gemini-cli`, or `opencode`
- **THEN** the native runtime SHALL evaluate enabled hooks bound to that stable agent id in deterministic stage and order
- **AND** it SHALL produce one effective prompt for the provider invocation builder

#### Scenario: Preserve provider-specific launch ownership
- **WHEN** Prompt Hook assembly completes
- **THEN** provider-specific command construction, stdin or argument prompt delivery, session resume tokens, and CLI parameter mapping SHALL remain owned by the provider invocation builder

#### Scenario: Avoid script execution
- **WHEN** the Prompt Hook pipeline renders built-in or user-created hooks
- **THEN** it SHALL treat hook templates as prompt text
- **AND** it SHALL NOT execute hook-provided shell commands, scripts, or arbitrary code

### Requirement: Native Prompt Hook commands remain bounded
Native Prompt Hook management and preview commands SHALL remain bounded request/response operations.

#### Scenario: Return Prompt Hook list directly
- **WHEN** the frontend lists Prompt Hooks or recent trace summaries
- **THEN** the native command MAY return the result directly after bounded catalog and SQLite reads
- **AND** it SHALL NOT spawn external commands, access networks, or launch provider CLIs

#### Scenario: Preview without provider launch
- **WHEN** the frontend requests Prompt Hook or effective prompt preview
- **THEN** the native runtime SHALL render the preview without launching a provider CLI process
