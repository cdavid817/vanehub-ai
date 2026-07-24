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

### Requirement: Allowlisted Prompt Hook template variables
The system SHALL render Prompt Hook templates from a backend-owned variable catalog and SHALL treat substitutions as inert text rather than executable expressions.

#### Scenario: Render canonical dynamic variables
- **WHEN** a published Prompt Hook references `{{agent_id}}`, `{{agent_name}}`, `{{current_time}}`, `{{sample_input}}`, or `{{session_id}}`
- **THEN** the pipeline SHALL replace each reference from the current invocation context
- **AND** `current_time` SHALL use one RFC 3339 UTC clock snapshot for the complete assembly

#### Scenario: Preserve existing variable aliases
- **WHEN** an existing template references `{{agentId}}` or `{{sampleInput}}`
- **THEN** the renderer SHALL resolve it to the same value as `{{agent_id}}` or `{{sample_input}}` respectively

#### Scenario: Reject unknown variables at publication
- **WHEN** a user attempts to publish a draft containing a variable outside the allowlisted catalog and compatibility aliases
- **THEN** the service SHALL reject publication with the unknown variable names
- **AND** the current published version SHALL remain unchanged

#### Scenario: Keep template text non-executable
- **WHEN** a template or replacement value contains shell syntax, command substitutions, markup, or script-like text
- **THEN** the renderer SHALL preserve it as literal Prompt text
- **AND** it SHALL NOT execute it or use it to read environment, filesystem, process, or credential data

### Requirement: User Prompt Hook draft and publication lifecycle
The system SHALL keep user Hook drafts separate from immutable published versions and SHALL use only the selected published version for live Prompt assembly.

#### Scenario: Create and edit a draft
- **WHEN** a user creates a Hook or saves changes to an existing user Hook
- **THEN** the service SHALL persist a draft without changing the current published version
- **AND** an unpublished new Hook SHALL not participate in live Prompt assembly

#### Scenario: Publish a valid draft
- **WHEN** a user publishes a valid draft against its expected current revision
- **THEN** the service SHALL atomically append the next monotonically increasing immutable version and select it as published
- **AND** subsequent live assemblies SHALL use that version

#### Scenario: Reject stale publication
- **WHEN** a user publishes a draft using a stale draft revision or stale published-version expectation
- **THEN** the service SHALL reject the operation without changing the draft, version history, or selected published version

#### Scenario: List version history
- **WHEN** a user requests version history for a user Hook
- **THEN** the service SHALL return immutable versions newest first with version number, publication timestamp, content hash, publication kind, and rollback source when present
- **AND** full template content SHALL only be included in an explicit version-detail or preview response

### Requirement: Prompt Hook version rollback
The system SHALL roll back a user Hook by publishing historical content as a new immutable version.

#### Scenario: Roll back to historical version
- **WHEN** a user selects an existing historical version for rollback
- **THEN** the service SHALL append the next version with the selected snapshot content and a `rollback_from_version` reference
- **AND** it SHALL select the new version for subsequent live assemblies

#### Scenario: Preserve unrelated draft during rollback
- **WHEN** a Hook has an unpublished draft and a user rolls back its published version
- **THEN** the service SHALL leave the draft unchanged
- **AND** it SHALL not silently discard or publish the draft

#### Scenario: Reject built-in version mutation
- **WHEN** a user attempts to save a draft, publish, or roll back a backend-owned built-in Hook
- **THEN** the service SHALL reject the operation and leave the built-in catalog unchanged

### Requirement: Prompt Hook version execution evaluation
The system SHALL record safe, idempotent terminal execution observations for each published Prompt Hook version fired by a live Agent invocation.

#### Scenario: Attribute terminal outcome
- **WHEN** an Agent invocation terminates after one or more published Prompt Hook versions fired
- **THEN** the system SHALL attribute the invocation id, terminal outcome, elapsed milliseconds, stable agent id, Hook id, and exact version to each fired version
- **AND** a retry of the same observation SHALL not increase its counts

#### Scenario: Exclude sensitive execution content
- **WHEN** an execution observation is persisted or logged
- **THEN** it SHALL NOT contain rendered Prompt text, user input, model output, raw errors, credentials, command arguments, unrestricted paths, or session content

#### Scenario: Aggregate version effectiveness
- **WHEN** a caller requests evaluation summaries for a Hook
- **THEN** the service SHALL return bounded per-version successful, failed, and cancelled counts, success rate, and average, minimum, and maximum elapsed milliseconds
- **AND** success rate and latency SHALL exclude cancelled executions

#### Scenario: Avoid causal claims
- **WHEN** version evaluation summaries are presented
- **THEN** the system SHALL describe them as operational outcomes for comparison
- **AND** it SHALL NOT claim that a Prompt version caused the observed result

