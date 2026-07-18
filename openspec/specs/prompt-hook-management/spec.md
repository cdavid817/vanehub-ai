# prompt-hook-management Specification

## Purpose
TBD - created by archiving change enhance-prompt-hook-management. Update Purpose after archive.
## Requirements
### Requirement: Prompt Hook manifest registry
The system SHALL expose a Prompt Hook registry with backend-owned built-in hooks and user-created hooks using a normalized manifest contract.

#### Scenario: List registered Prompt Hooks
- **WHEN** the frontend requests Prompt Hooks
- **THEN** the service SHALL return built-in and user-created hooks with stable id, name, category, stage, order, version, source, enabled state, disableable flag, CLI bindings, governance metadata, and localized description keys or user-provided descriptions
- **AND** built-in hooks SHALL come from VaneHub-defined catalog data rather than Clowder-AI content

#### Scenario: Validate hook categories
- **WHEN** a Prompt Hook is loaded or saved
- **THEN** its category SHALL be one of `bootstrap`, `callback`, `dynamic`, `law`, `navigation`, `routing`, or `static`

### Requirement: Built-in Prompt Hook governance
The system SHALL allow safe global management of built-in Prompt Hooks without allowing built-in content edits.

#### Scenario: Toggle disableable built-in hook
- **WHEN** a user enables or disables a built-in Prompt Hook whose `disableable` flag is true
- **THEN** the service SHALL persist the override globally and return the updated effective hook

#### Scenario: Reject disabling immutable built-in hook
- **WHEN** a user attempts to disable a built-in Prompt Hook whose `disableable` flag is false
- **THEN** the service SHALL reject the change with a concise user-displayable error
- **AND** the hook SHALL remain enabled

#### Scenario: Reject built-in content edit
- **WHEN** a user attempts to edit the template body or immutable manifest fields of a built-in Prompt Hook
- **THEN** the service SHALL reject the edit

### Requirement: User-created Prompt Hooks
The system SHALL allow users to create, edit, delete, enable, disable, bind, and preview custom Prompt Hooks.

#### Scenario: Create user hook
- **WHEN** a user submits a valid custom Prompt Hook
- **THEN** the service SHALL persist it with source `user`
- **AND** the hook SHALL participate in the same listing, preview, binding, and pipeline behavior as built-in hooks

#### Scenario: Reject unsafe user hook
- **WHEN** a user hook contains an invalid id, unsupported category, duplicate order within its stage and category context, control characters, or an unsupported CLI binding
- **THEN** the service SHALL reject the mutation without changing the previous stored hooks

#### Scenario: Delete user hook
- **WHEN** a user deletes a user-created Prompt Hook
- **THEN** the service SHALL remove that hook and its bindings
- **AND** built-in hooks SHALL remain unaffected

### Requirement: Prompt Hook CLI bindings
The system SHALL bind Prompt Hooks to supported CLI agents by stable agent id.

#### Scenario: Bind hook to CLI agents
- **WHEN** a user updates a Prompt Hook's CLI bindings
- **THEN** the service SHALL persist only supported stable CLI agent ids among `claude-code`, `codex-cli`, `gemini-cli`, and `opencode`
- **AND** it SHALL NOT match agents by display name

#### Scenario: Unbound hook does not apply
- **WHEN** a Prompt Hook has no binding for the active session's stable CLI agent id
- **THEN** the Prompt Hook pipeline SHALL skip that hook for the invocation

### Requirement: Prompt Hook preview
The system SHALL support explicit preview of rendered Prompt Hook content and assembled effective prompt content without exposing it by default.

#### Scenario: Preview one hook
- **WHEN** a user explicitly requests preview for a Prompt Hook
- **THEN** the service SHALL render that hook with deterministic preview context
- **AND** it SHALL return the rendered content and trace metadata through the service boundary

#### Scenario: Preview assembled prompt
- **WHEN** a user explicitly requests an assembled prompt preview for a CLI agent and sample user input
- **THEN** the service SHALL return the effective prompt and trace metadata without launching a CLI process

### Requirement: Prompt Hook trace summaries
The system SHALL produce safe trace summaries for Prompt Hook execution.

#### Scenario: Produce trace summary
- **WHEN** the Prompt Hook pipeline evaluates hooks for a chat invocation or preview
- **THEN** it SHALL produce trace events with hook id, category, stage, status, version when fired, content hash when content is rendered, token estimate when available, skip reason when skipped, and timestamp
- **AND** the summary SHALL NOT include full rendered content unless the caller requested explicit preview

#### Scenario: Persist recent desktop traces
- **WHEN** desktop runtime executes Prompt Hooks for a CLI chat invocation
- **THEN** the native runtime SHALL persist recent safe trace summaries in SQLite for settings inspection
- **AND** it SHALL NOT persist raw effective prompt content in trace storage

### Requirement: Prompt Hook runtime parity
The Tauri and Web/mock adapters SHALL expose equivalent Prompt Hook service contracts.

#### Scenario: Desktop Prompt Hook management
- **WHEN** the frontend runs in the Tauri desktop runtime
- **THEN** the Tauri adapter SHALL call declared native commands for Prompt Hook listing, mutation, preview, and trace queries

#### Scenario: Web Prompt Hook management
- **WHEN** the frontend runs in Web/mock mode
- **THEN** the Web adapter SHALL provide deterministic Prompt Hook catalog, mutation, preview, and mock trace behavior without accessing SQLite or launching local CLI processes

