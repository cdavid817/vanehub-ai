# agent-context-compaction-control Specification

## Purpose
Defines the controls and generation-scoped safety guards that decide when OnePiece must skip an otherwise eligible automatic context compaction without weakening request continuity or observability.
## Requirements
### Requirement: Request-level automatic compaction suppression
Each OnePiece generation request SHALL carry an automatic-compaction mode of `automatic` or `suppressed`. A suppressed request SHALL bypass every automatic compaction attempt for that generation while leaving its provider request content unchanged; an omitted mode SHALL preserve the existing enabled behavior.

#### Scenario: Caller suppresses automatic compaction
- **WHEN** a generation request declares automatic compaction `suppressed`
- **THEN** the system SHALL NOT optimize, summarize, or otherwise mutate that generation's context through automatic compaction
- **AND** it SHALL continue the provider request with the original prepared context

#### Scenario: No explicit control is supplied
- **WHEN** a generation request does not declare an automatic-compaction override
- **THEN** the system SHALL treat automatic compaction as enabled

### Requirement: Generation-scoped compaction cooldown
After a successful automatic compaction, the system SHALL suppress another automatic compaction in the same generation until the prepared context has grown by the versioned minimum-growth budget. Cooldown state SHALL NOT carry into another generation.

#### Scenario: Context has not grown enough
- **WHEN** automatic compaction succeeded earlier in the generation and subsequent context growth is below the minimum-growth budget
- **THEN** the system SHALL skip the repeated automatic compaction attempt
- **AND** it SHALL send the current prepared context unchanged

#### Scenario: Context grows beyond cooldown
- **WHEN** automatic compaction succeeded earlier in the generation and subsequent context growth reaches the minimum-growth budget
- **THEN** cooldown SHALL no longer suppress an otherwise eligible automatic compaction

#### Scenario: New generation starts
- **WHEN** a new generation begins for the same session
- **THEN** it SHALL start with fresh cooldown state

### Requirement: Automatic compaction failure circuit breaker
The system SHALL count consecutive automatic compaction failures within one generation and SHALL open a generation-scoped circuit after the versioned failure limit. An open circuit SHALL suppress later automatic attempts, a successful compaction SHALL reset the count, and no failure state SHALL carry into another generation.

#### Scenario: Consecutive failures reach the limit
- **WHEN** optimizer and compatibility fallback both fail to install a compacted candidate on enough consecutive eligible attempts to reach the failure limit
- **THEN** the system SHALL open the automatic-compaction circuit for the rest of that generation
- **AND** it SHALL preserve the current prepared context

#### Scenario: Successful compaction resets failures
- **WHEN** an automatic compaction succeeds before the failure limit is reached
- **THEN** the system SHALL reset the consecutive failure count

#### Scenario: Open circuit receives another eligible context
- **WHEN** the circuit is open and the context would otherwise trigger automatic compaction
- **THEN** the system SHALL skip all optimizer and summary provider calls for that attempt

### Requirement: Content-free compaction-control evidence
The system SHALL record automatic trigger-source, suppression, cooldown, failure, and circuit transitions through unified logging using bounded counters, policy versions, measurement qualities, correlations, and stable reason codes only.

#### Scenario: Automatic compaction is bypassed
- **WHEN** request suppression, cooldown, or an open circuit prevents automatic compaction
- **THEN** diagnostics SHALL identify the bounded bypass reason and current generation-scoped control state
- **AND** they SHALL NOT contain prompts, messages, tool arguments, tool results, summaries, credentials, headers, or raw provider payloads

#### Scenario: Circuit state changes
- **WHEN** an automatic compaction failure increments the circuit or a success resets it
- **THEN** diagnostics SHALL record the bounded result, consecutive-failure count, and whether the circuit is open

### Requirement: Persisted user preference suppresses automatic compaction
The automatic-compaction decision SHALL combine the persisted user preference with request-level suppression and generation-scoped safety guards. A disabled user preference SHALL suppress every automatic compaction attempt for generations started from that settings snapshot without mutating the provider request context.

#### Scenario: User preference is disabled before generation
- **WHEN** a OnePiece generation starts with automatic context compaction disabled in application settings
- **THEN** the generation SHALL NOT optimize, summarize, or otherwise mutate its context through automatic compaction
- **AND** normal provider generation SHALL continue with the unmodified prepared context

#### Scenario: Preference changes during an active generation
- **WHEN** the user changes the automatic-compaction preference while a generation is active
- **THEN** the active generation SHALL retain its captured preference
- **AND** the new preference SHALL apply to later generations

#### Scenario: Preference is enabled but request is suppressed
- **WHEN** the persisted preference is enabled and the generation request declares automatic compaction suppressed
- **THEN** request-level suppression SHALL still prevent automatic compaction

