## ADDED Requirements

### Requirement: Antigravity CLI built-in agent registration
The native runtime SHALL register `antigravity-cli` as a built-in Agent whose provider is Google, whose launch kind is `cli`, whose launch command and executable name are both `agy`, and whose supported interaction modes are `cli`. Registration SHALL be idempotent for databases that already contain the row, and SHALL NOT require the `agy` executable to be present on the host.

#### Scenario: Built-in agent present after upgrade
- **WHEN** the native runtime starts against a database created before this change
- **THEN** the agent registry SHALL contain an `antigravity-cli` entry with agent origin `builtin`
- **AND** re-running the same startup against a database that already has the row SHALL leave it unchanged

#### Scenario: Availability reported without the executable installed
- **WHEN** an availability check runs for `antigravity-cli` on a host where `agy` is not on PATH and no known install directory contains it
- **THEN** the runtime SHALL report the agent as unavailable with the reason naming the missing `agy` command
- **AND** the check SHALL NOT start an interactive session or a CLI process

### Requirement: Antigravity CLI managed chat invocation contract
The native runtime SHALL build managed (non-interactive) chat invocations for `antigravity-cli` as `agy` invoked with the agent's mapped CLI parameters, `--output-format stream-json`, the prompt delivered through the `-p` argument, and — when a provider runtime session id is known — `--conversation <id>` to resume that conversation. The runtime SHALL NOT expose `-p`, `--output-format`, or `--conversation` as user-selectable parameters.

#### Scenario: Build a fresh invocation
- **WHEN** a CLI chat invocation starts for `antigravity-cli` with no persisted runtime session id
- **THEN** the built argument list SHALL contain `--output-format stream-json` and deliver the effective prompt as the `-p` argument value
- **AND** it SHALL NOT contain `--conversation`

#### Scenario: Resume a known conversation
- **WHEN** a CLI chat invocation starts for `antigravity-cli` for a session with a persisted runtime session id
- **THEN** the built argument list SHALL contain `--conversation` followed by that id

#### Scenario: Managed arguments cannot be overridden by user selections
- **WHEN** a saved CLI parameter profile for `antigravity-cli` would produce `-p`, `--output-format`, or `--conversation`
- **THEN** the invocation builder SHALL reject or drop that selection rather than emit a duplicate or conflicting argument

### Requirement: Antigravity CLI streaming output normalization
The native runtime SHALL parse `antigravity-cli` stdout as newline-delimited JSON carrying `init`, `step_update`, and `result` events, and SHALL normalize them into the runtime's existing chat event vocabulary. The runtime SHALL treat unrecognized event kinds and unrecognized fields within a recognized event as ignorable rather than as parse failures.

#### Scenario: Capture the runtime session id
- **WHEN** an `init` event carries a `conversation_id`
- **THEN** the runtime SHALL persist that value as the session's provider runtime session id

#### Scenario: Terminal status determines the lifecycle outcome
- **WHEN** a `result` event reports status `SUCCESS`
- **THEN** the invocation SHALL complete successfully, carrying the reported usage
- **AND** **WHEN** it reports `ERROR`, `INVALID`, `CANCELED`, or `INTERRUPTED`, the invocation SHALL fail non-retryably with the event's own reported error preserved as the diagnostic

#### Scenario: A self-reported cancel is not silently treated as success
- **WHEN** a `result` event reports status `CANCELED` or `INTERRUPTED`
- **THEN** the invocation SHALL NOT report a completed turn
- **AND** the failure SHALL be classified non-retryable, because re-running cannot resolve a cancellation the provider decided on

#### Scenario: Non-terminal status on a terminal event is a protocol violation
- **WHEN** a `result` event reports status `WAITING` or `RUNNING`
- **THEN** the runtime SHALL fail the invocation with a protocol error rather than treat it as success or silently discard it

#### Scenario: Unknown event kinds do not break a run
- **WHEN** stdout contains a JSON line whose event kind the runtime does not recognize
- **THEN** the runtime SHALL ignore that line and continue processing subsequent events

#### Scenario: Incremental step events are consumed without inventing a payload shape
- **WHEN** stdout contains a `step_update` event
- **THEN** the runtime SHALL consume it without emitting incremental output, until its payload has been captured from a live authenticated run
- **AND** the completed turn SHALL still deliver the full reply, which the `result` event carries in its `response` field

## MODIFIED Requirements

### Requirement: Native Prompt Hook pipeline
The native runtime SHALL provide a provider-agnostic Prompt Hook pipeline before CLI provider invocation.

#### Scenario: Assemble effective prompt
- **WHEN** a CLI chat invocation starts for `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, or `antigravity-cli`
- **THEN** the native runtime SHALL evaluate enabled hooks bound to that stable agent id in deterministic stage and order
- **AND** it SHALL produce one effective prompt for the provider invocation builder

#### Scenario: Preserve provider-specific launch ownership
- **WHEN** Prompt Hook assembly completes
- **THEN** provider-specific command construction, stdin or argument prompt delivery, session resume tokens, and CLI parameter mapping SHALL remain owned by the provider invocation builder

#### Scenario: Avoid script execution
- **WHEN** the Prompt Hook pipeline renders built-in or user-created hooks
- **THEN** it SHALL treat hook templates as prompt text
- **AND** it SHALL NOT execute hook-provided shell commands, scripts, or arbitrary code

### Requirement: Native custom instructions CLI injection precedes Prompt Hook assembly in the final effective prompt

The native runtime SHALL combine host-level custom instructions with the Prompt Hook pipeline's assembled output into one final effective prompt, before that text reaches the provider invocation builder. This requirement governs only where custom instructions are combined relative to the Prompt Hook pipeline; the "Native Prompt Hook pipeline" requirement's own hook evaluation, binding, and template rendering are unaffected.

#### Scenario: Combine custom instructions ahead of the Prompt Hook output
- **WHEN** a CLI chat invocation starts for `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, or `antigravity-cli` with custom instructions enabled and non-empty
- **THEN** the native runtime SHALL place the custom-instructions section before the Prompt Hook pipeline's assembled content in the final effective prompt handed to the provider invocation builder

#### Scenario: No custom instructions leaves Prompt Hook assembly unchanged
- **WHEN** custom instructions are disabled or empty
- **THEN** the final effective prompt SHALL be exactly the Prompt Hook pipeline's own assembled output, unchanged from behavior before this requirement existed

#### Scenario: Custom instructions resolution failure does not block CLI invocation
- **WHEN** resolving custom instructions fails during a CLI chat invocation
- **THEN** the native runtime SHALL log the failure and proceed with the Prompt Hook pipeline's assembled output alone
- **AND** it SHALL NOT fail or delay the CLI invocation

### Requirement: Native memory injection follows custom instructions and precedes Prompt Hook assembly in the final CLI effective prompt

The native runtime SHALL combine the shared host-level memory pool with the Prompt Hook pipeline's assembled output into the final effective prompt for CLI-wrapped agents, placed after any custom-instructions section and before the Prompt Hook pipeline's own assembled content, before that text reaches the provider invocation builder. This requirement governs only where the memory section sits relative to custom instructions and the Prompt Hook pipeline; the "Native Prompt Hook pipeline" requirement's own hook evaluation, binding, and template rendering are unaffected, and the "Native custom instructions CLI injection precedes Prompt Hook assembly in the final effective prompt" requirement's own ordering guarantee is unaffected.

#### Scenario: Combine memory content between custom instructions and the Prompt Hook output
- **WHEN** a CLI chat invocation starts for `claude-code`, `codex-cli`, `gemini-cli`, `opencode`, or `antigravity-cli` with the memory enablement toggle on and at least one memory in the shared pool
- **THEN** the native runtime SHALL place the memory section after the custom-instructions section (if present) and before the Prompt Hook pipeline's assembled content in the final effective prompt handed to the provider invocation builder

#### Scenario: No memory content leaves the rest of the effective prompt unchanged
- **WHEN** the memory enablement toggle is off, or the shared memory pool is empty
- **THEN** the final effective prompt SHALL be exactly what it would have been without this requirement, unchanged from behavior before this requirement existed

#### Scenario: Memory resolution failure does not block CLI invocation
- **WHEN** resolving the shared memory pool fails during a CLI chat invocation
- **THEN** the native runtime SHALL log the failure and proceed with the rest of the effective prompt (custom instructions and Prompt Hook output) unaffected
- **AND** it SHALL NOT fail or delay the CLI invocation
