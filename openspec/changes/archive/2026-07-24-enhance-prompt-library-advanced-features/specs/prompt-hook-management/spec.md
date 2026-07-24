## ADDED Requirements

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
