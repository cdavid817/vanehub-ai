## ADDED Requirements

### Requirement: Effectful tool calls pass through a durable journal
The system SHALL journal every effectful tool call before executing it — a stable journal identity, the generation and round, the tool name, an input digest, and an effect class — advance the entry through Intent, Executing, and a terminal Completed, Failed, or UnknownEffect state, and record an outcome digest. Read-only tools MAY journal by class rather than per call. A journal entry's identity SHALL be stable across restarts so duplicate delivery of one terminal event cannot execute the tool twice.

#### Scenario: Journal-before-execute for an effectful tool
- **WHEN** the tool loop dispatches an effectful tool
- **THEN** the journal SHALL hold an Intent entry before the side effect can begin
- **AND** the entry SHALL reach a terminal state when the call ends

#### Scenario: Duplicate terminal delivery does not double-execute
- **WHEN** the same tool-call completion is delivered twice
- **THEN** the journal SHALL recognize the stable identity
- **AND** the tool SHALL NOT run a second time

### Requirement: Unknown-effect entries are surfaced, never auto-replayed
After an interruption, a journal entry left in Executing SHALL be resolved to UnknownEffect, and recovery SHALL surface it for review with the recorded intent. The system SHALL NOT automatically re-execute an UnknownEffect entry — the same conservatism session recovery applies to uncertain CLI side effects.

#### Scenario: A crash mid-execution becomes a reviewable entry
- **WHEN** the process crashes after an effectful tool started but before its outcome was recorded
- **THEN** startup recovery SHALL mark the entry UnknownEffect and surface it
- **AND** SHALL NOT re-run the tool on its own
