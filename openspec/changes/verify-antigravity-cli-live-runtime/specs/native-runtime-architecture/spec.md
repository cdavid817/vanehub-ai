## MODIFIED Requirements

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
- **WHEN** a live authenticated capture has established the `step_update` payload shape and stdout contains a supported incremental event
- **THEN** the runtime SHALL map the observed payload to incremental output rather than withholding it until the turn completes
- **AND** the completed turn SHALL NOT duplicate content already emitted incrementally
