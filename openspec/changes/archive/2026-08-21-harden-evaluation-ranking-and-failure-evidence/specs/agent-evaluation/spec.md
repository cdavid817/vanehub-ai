## ADDED Requirements

### Requirement: Agent dispatch failures record bounded diagnostic evidence
When an evaluation attempt fails before or during Agent dispatch, the attempt SHALL record at least one failed, redacted, bounded diagnostic check naming why dispatch failed, and that check SHALL be presented as non-authoritative evidence that never converts a failure into a success.

#### Scenario: Agent cannot be dispatched
- **WHEN** an attempt's Agent cannot be dispatched — no configured model, unusable interaction mode, session creation refused, or the terminal channel disconnects
- **THEN** the attempt SHALL terminate as `agent_failed` and SHALL carry a failed diagnostic check whose summary states the reason within the bounded, redacted form applied to every persisted evaluation field

#### Scenario: Diagnostic evidence would leak execution data
- **WHEN** a dispatch failure reason contains a prompt, credential, environment value, raw tool payload, or private absolute path
- **THEN** the recorded summary SHALL omit or redact those values before persistence, export, and display, in the same way every other evaluation row is redacted

#### Scenario: Diagnostic evidence is not a verdict
- **WHEN** an attempt carries a dispatch diagnostic check
- **THEN** the check SHALL NOT be counted as a deterministic acceptance result and SHALL NOT rank the attempt below an attempt whose failure recorded no evidence at all

## MODIFIED Requirements

### Requirement: Arena comparison is transparent and versioned
The system SHALL compare attempts for the same task/version using a documented ranking algorithm version and independent metric columns, SHALL NOT synthesize an opaque total score from mutually unavailable metrics, and SHALL NOT let an attempt that recorded no evidence outrank an attempt that recorded evidence of the same or better quality. Attempts SHALL be ordered by outcome tier first — deterministic success, then deterministic task failure, then non-completion outcomes (Agent failure, timeout, stuck, cancelled, benchmark error) — before any evidence-count or metric comparison is applied, and the ranking SHALL be applied wherever an arena is read rather than only where it is computed. A change to this ordering SHALL be published as a new ranking algorithm version, and every arena, exported payload, and stored snapshot SHALL name the version its ordering was produced under.

#### Scenario: Agents have different metric coverage
- **WHEN** one result has reported token usage and another has unavailable token usage
- **THEN** the table SHALL show the coverage difference and SHALL NOT treat the unavailable value as zero or use it as a hidden ranking advantage

#### Scenario: Deterministic outcomes differ
- **WHEN** one Agent passes all deterministic acceptance and another fails
- **THEN** the passing result SHALL rank ahead regardless of optional judge score

#### Scenario: One attempt failed a check and another produced no checks at all
- **WHEN** one attempt is a deterministic task failure with one or more recorded failing checks and another attempt ended in a non-completion outcome with no recorded checks
- **THEN** the task failure SHALL rank ahead of the non-completion outcome, and the empty check list SHALL NOT be read as "zero failures"

#### Scenario: Arena is read back after it finished
- **WHEN** a stored arena is listed, fetched, or exported, in any order the underlying store returns its attempts
- **THEN** the attempts SHALL be presented in ranked order, and two attempts that the ranking cannot distinguish SHALL be returned in a stable order that does not vary between reads

#### Scenario: Ranking semantics change
- **WHEN** the comparison rules are changed such that two attempts could order differently than before
- **THEN** the ranking algorithm version SHALL be incremented, arenas recorded under an earlier version SHALL keep naming that earlier version, and exports SHALL NOT present results produced under different versions as directly comparable
