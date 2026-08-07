## ADDED Requirements

### Requirement: Claude Code CLI error result normalization

The system SHALL treat a claude-code `result` event that reports an error as a generation failure and SHALL surface the CLI's own diagnostic text rather than a generic exit status.

#### Scenario: Result event reports an error

- **WHEN** claude-code emits a `result` event whose payload marks it as an error
- **THEN** the runtime SHALL normalize it into a failed generation event
- **AND** the failure diagnostic SHALL be the CLI's reported result text
- **AND** the runtime SHALL NOT normalize it into a completed event

#### Scenario: Error result carries a structured code

- **WHEN** an error result carries a structured error code, status, type, or reason
- **THEN** the runtime SHALL use it to classify the failure as retryable or non-retryable
- **AND** an authentication or policy rejection SHALL NOT be classified as retryable

#### Scenario: Result event reports success

- **WHEN** claude-code emits a `result` event that does not report an error
- **THEN** the runtime SHALL normalize it into a completed event with its token accounting unchanged

### Requirement: Process failure prefers a parsed diagnostic

The system SHALL report the most specific failure text available when a managed Agent process exits non-zero, and SHALL fall back to the exit status only when no diagnostic was parsed or written.

#### Scenario: Process exits non-zero after an error result

- **WHEN** a managed Agent process exits non-zero and the output stream already yielded a failure diagnostic
- **THEN** the runtime SHALL report that diagnostic
- **AND** it SHALL NOT replace it with the process exit status

#### Scenario: Process exits non-zero with empty stderr and no diagnostic

- **WHEN** a managed Agent process exits non-zero, writes nothing to standard error, and produced no parsed failure diagnostic
- **THEN** the runtime SHALL report the exit status, because no better information exists
