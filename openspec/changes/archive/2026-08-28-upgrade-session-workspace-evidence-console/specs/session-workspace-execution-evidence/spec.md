## ADDED Requirements

### Requirement: Canonical session workspace evidence scope

The system SHALL represent cross-panel evidence selection with one serializable scope containing the selected session and any available seat, run, trace, span, operation, command, relative-path, hunk, and occurrence-time identifiers.

#### Scenario: Navigate from an error log to its span

- **WHEN** a session log record contains run, trace, and span correlation
- **THEN** activating its Trace action SHALL select the Traces tab and the correlated span through the shared workspace evidence scope
- **AND** the navigation SHALL NOT require the Traces component to import Logs state or implementation details

#### Scenario: Select another session

- **WHEN** the user changes the selected session
- **THEN** run, trace, span, operation, command, path, hunk, and occurrence-time fields from the previous session SHALL be cleared
- **AND** no panel SHALL query the new session using identifiers owned by the previous session

#### Scenario: Panel cannot consume a target field

- **WHEN** a navigation target includes a scope field unsupported by the destination panel
- **THEN** the destination SHALL identify the unsupported filter or safely ignore it with an explicit user-visible notice
- **AND** it SHALL NOT silently claim that the field was applied

### Requirement: Effective workspace seat scope

The workspace SHALL apply the selected seat only to tabs whose declared capability supports or requires seat scope.

#### Scenario: Filter Terminal History by seat

- **WHEN** a multi-Agent user selects one seat while Terminal History is active
- **THEN** the execution-record query SHALL include that stable seat id
- **AND** the rendered records SHALL identify the active seat filter

#### Scenario: Open a Shell for a multi-Agent session

- **WHEN** Shell is active for a multi-Agent session
- **THEN** the workspace SHALL require one concrete active seat before creating or attaching an interactive Shell
- **AND** the Shell descriptor SHALL retain the same seat id

#### Scenario: Open a session-scoped tab

- **WHEN** Changes, Documents, Files, Traces, or Report is active in its default session-scoped mode
- **THEN** the global workspace seat selector SHALL NOT imply that the tab data has changed

### Requirement: Append-only execution evidence journal

The desktop runtime SHALL append versioned metadata-only execution evidence events in SQLite and SHALL derive bounded query projections without using the journal as a replacement for the canonical run, trace, usage, log, workspace, Shell-output, or review stores.

#### Scenario: Record a command completion

- **WHEN** a native or proxied command boundary completes with safe correlation and terminal status
- **THEN** the evidence service SHALL append one versioned event and update the command projection in one transaction
- **AND** it SHALL retain references to available run, trace, span, operation, Agent, seat, and command ids

#### Scenario: Query evidence with OTLP disabled

- **WHEN** optional OTLP export is disabled or unavailable
- **THEN** locally committed evidence SHALL remain queryable through the native service boundary

#### Scenario: Canonical source changes

- **WHEN** a canonical Run, usage observation, Review Session, log record, or workspace snapshot changes
- **THEN** its owning context SHALL remain authoritative
- **AND** the evidence journal SHALL contain only a safe event/reference or rebuildable projection rather than an independently mutable copy of the aggregate

### Requirement: Idempotent and conflict-aware evidence ingestion

Every evidence producer SHALL provide a stable source context and source event id, and the evidence repository SHALL enforce idempotent ingestion.

#### Scenario: Retry an identical event

- **WHEN** an identical normalized event is submitted again with the same source context and source event id
- **THEN** the service SHALL report idempotent success without appending or projecting a duplicate

#### Scenario: Reuse a source id with conflicting content

- **WHEN** a producer submits different normalized content for an already persisted source context and source event id
- **THEN** the service SHALL preserve the original event
- **AND** it SHALL mark affected coverage partial and emit a bounded redacted conflict diagnostic

### Requirement: Metadata-only evidence privacy

Execution evidence SHALL be allowlisted, bounded, and redacted before local journal persistence, query projection, frontend notice publication, unified logging, or optional telemetry export.

#### Scenario: Producer includes sensitive execution content

- **WHEN** an evidence input contains a raw prompt, model response, tool or MCP payload, unrestricted command arguments, terminal output, source code, full diff, review prose, secret, header, environment value, private key, or absolute user path
- **THEN** the service SHALL reject, omit, fingerprint, normalize, or redact that field before any evidence sink
- **AND** the original sensitive value SHALL NOT appear in journal payloads or identifier-only live notices

#### Scenario: Persist a local command display summary

- **WHEN** a command record is permitted to expose a bounded redacted local display summary
- **THEN** the summary SHALL be stored only in the local command projection
- **AND** it SHALL NOT be copied into trace attributes, OTLP attributes, unified diagnostic logs, or journal payloads that prohibit command content

### Requirement: Non-blocking bounded evidence publication

Evidence capture SHALL use bounded resources and SHALL NOT determine the success of the owning Agent, Shell, review, logging, usage, or workspace operation.

#### Scenario: Evidence repository is unavailable

- **WHEN** an owning operation succeeds but evidence persistence fails
- **THEN** the owning operation SHALL retain its canonical result
- **AND** the system SHALL emit a rate-limited redacted diagnostic without recursively using the failed evidence path

#### Scenario: Evidence queue reaches capacity

- **WHEN** a bounded producer queue cannot accept one or more evidence events
- **THEN** the producer SHALL remain responsive
- **AND** evidence coverage SHALL later expose one bounded gap with a safe dropped count rather than claiming completeness

### Requirement: Evidence-backed execution record query

The system SHALL provide bounded execution-record pages for Commands, Tools, Delegations, and Verification outcomes using stable ids, explicit status, fidelity, coverage, and safe correlation.

#### Scenario: List native command records

- **WHEN** Terminal History requests Commands for a session or seat
- **THEN** the service SHALL return bounded newest-first records with runtime kind, safe display availability, start and terminal timing, status, exit or signal data when observed, fidelity, output availability, and correlation links

#### Scenario: Merged PTY output is the only output

- **WHEN** a command is observed through a PTY that does not distinguish stdout and stderr
- **THEN** the record SHALL identify merged PTY output
- **AND** it SHALL NOT label the output as separate stdout or stderr

#### Scenario: Only the completion was observed

- **WHEN** the runtime observes a command, tool, delegation, or verification completing but never observed it starting
- **THEN** the record SHALL retain the terminal status that was actually observed
- **AND** it SHALL report the observed completion time as its end boundary
- **AND** it SHALL retain a duration only when its source supplied one explicitly
- **AND** the start boundary SHALL be omitted rather than derived from the completion time, the duration, or the event occurrence time
- **AND** coverage SHALL identify the unobserved start boundary with a safe reason code

#### Scenario: Command boundary is opaque

- **WHEN** the runtime knows a process or tool stage exists but cannot observe command text, exit data, or child details
- **THEN** the record SHALL be marked `opaque` or `inferred` according to available evidence
- **AND** unavailable fields SHALL remain absent rather than fabricated

### Requirement: Honest legacy activity projection

Historical chat-message tool activity MAY remain visible, but it MUST remain separate from native execution evidence and MUST expose its source and coverage limits.

#### Scenario: Show historical toolUse data

- **WHEN** a historical session contains persisted message `toolUse` entries with no native evidence ids
- **THEN** Terminal History SHALL show them as legacy message-history activity with `inferred` fidelity
- **AND** it SHALL NOT insert them into the execution journal as native commands or tools

#### Scenario: Message history is only partly loaded

- **WHEN** legacy activity is derived from a bounded or compacted message set
- **THEN** the UI SHALL identify the activity coverage as partial or unknown
- **AND** Report SHALL NOT treat it as a complete native execution ledger

### Requirement: Correlated file, review, verification, and usage evidence

Execution records and summaries SHALL expose safe links to observed file mutations, review decisions/findings, verification outcomes, and usage observations when their canonical owners provide correlation.

#### Scenario: Command produces an observed file mutation

- **WHEN** a command and a later workspace snapshot share trusted run, operation, or command correlation
- **THEN** the command detail SHALL expose a link to the relative file or Changes view
- **AND** the evidence store SHALL retain only safe relative-path display or fingerprint metadata rather than file content

#### Scenario: Verification finding has a source span

- **WHEN** a Test, Security, or Review action persists a finding with operation and span correlation
- **THEN** the finding SHALL link to the owning run/span and the run report SHALL count it once

#### Scenario: Usage is correlated to a run

- **WHEN** sessions persists a normalized usage observation with run correlation
- **THEN** evidence MAY retain its stable reference and quality classification
- **AND** token dimensions and accounting semantics SHALL continue to come from the sessions usage read model

### Requirement: Stable evidence pagination and truthful coverage

Append-heavy evidence queries SHALL use opaque query-bound keyset cursors and SHALL state whether the requested corpus is complete, indexing, partial, or unavailable.

#### Scenario: New evidence arrives between pages

- **WHEN** a client loads the first newest-first evidence page, newer evidence is appended, and the client requests the next page with the original cursor
- **THEN** the next page SHALL continue after the original boundary without duplicating or skipping records that existed at that boundary

#### Scenario: Reuse cursor with different filters

- **WHEN** a cursor created for one session, seat, kind, status, or search filter is submitted with another filter set
- **THEN** the service SHALL reject it with `cursor_filter_mismatch`
- **AND** it SHALL NOT interpret the cursor as an offset

#### Scenario: Evidence was dropped or expired

- **WHEN** known evidence gaps or retention expiry affect the requested scope
- **THEN** coverage SHALL be `partial` with safe reason codes, available boundaries, and bounded dropped counts when known
- **AND** the UI SHALL NOT describe the result as complete

### Requirement: Cross-panel evidence navigation

Every supported evidence link SHALL navigate through the shared workspace evidence target and SHALL preserve the owning session and relevant filters.

#### Scenario: Navigate from command to logs

- **WHEN** a command record has an operation, trace, span, or command correlation
- **THEN** its Logs action SHALL activate Logs with the strongest available structured filter
- **AND** Logs SHALL show the active filter and allow it to be cleared

#### Scenario: Navigate from report failure to trace

- **WHEN** a Report failure group has one or more correlated failed spans
- **THEN** selecting a concrete failure SHALL open the matching run/span in Traces

#### Scenario: Target evidence no longer retained

- **WHEN** navigation points to evidence that has expired or is outside current coverage
- **THEN** the destination SHALL show a localized unavailable/expired state while preserving the remaining valid scope

### Requirement: Bounded workspace evidence summary

The service SHALL provide one bounded workspace summary for tab badges and compact Basic Info health without requiring every workspace panel to mount or execute its full query.

#### Scenario: Render summary badges

- **WHEN** a selected session has running or failed execution records, live Shells, new error logs, active or failed traces, unviewed review files, failed verification, or partial report coverage
- **THEN** the summary SHALL return bounded counts/statuses for the owning tab
- **AND** each badge SHALL have a localized accessible meaning

#### Scenario: Summary source is partial

- **WHEN** one summary source is indexing, partial, or unavailable
- **THEN** the summary SHALL identify that source coverage
- **AND** it SHALL NOT turn an unknown count into a definitive zero

### Requirement: Authoritative session-run report

The system SHALL generate a bounded session-run report through backend application services using published evidence, observability, usage, log, workspace, review, and verification summaries rather than the set of messages currently mounted in React.

#### Scenario: Chat messages are only partly loaded

- **WHEN** Report is requested while React has loaded only part of the session message history
- **THEN** report totals and coverage SHALL be determined by backend read models
- **AND** loading another message page SHALL NOT change an otherwise identical report query

#### Scenario: Report has mixed usage quality

- **WHEN** a report contains reported, reported-derived, and estimated usage observations
- **THEN** it SHALL keep those qualities separate and expose coverage
- **AND** it SHALL NOT add estimated characters to reported Token totals

#### Scenario: One report section is unavailable

- **WHEN** log, change, verification, usage, or evidence data is unavailable while other sections are queryable
- **THEN** the report SHALL return the available sections and mark the affected section unavailable or partial
- **AND** it SHALL NOT fail the entire report or silently substitute complete zero values

#### Scenario: Monetary cost has no versioned pricing observation

- **WHEN** a report has Token usage but no explicitly versioned provider-pricing observation
- **THEN** monetary cost SHALL be absent or identified as unavailable
- **AND** the system SHALL NOT fabricate billing precision

### Requirement: Execution evidence service boundary and runtime parity

React SHALL access workspace evidence, summaries, subscriptions, and reports only through frontend service interfaces implemented by both Tauri and Web/mock adapters.

#### Scenario: Desktop queries evidence

- **WHEN** React requests execution records in the Tauri desktop runtime
- **THEN** it SHALL call the frontend evidence service
- **AND** the Tauri adapter SHALL invoke declared Rust commands rather than React reading SQLite, log files, terminal buffers, or Git directly

#### Scenario: Web queries evidence

- **WHEN** the same UI runs through the Web/mock adapter
- **THEN** it SHALL receive deterministic contract-compatible pages, coverage, Shell states, subscriptions, and reports
- **AND** simulated results SHALL NOT claim native SQLite, process, filesystem, SSH, Git, log export, or OTLP side effects

### Requirement: Evidence projection replay and retention

Execution evidence projections SHALL be rebuildable from retained journal events and SHALL expire consistently with configured evidence retention.

#### Scenario: Rebuild a projection

- **WHEN** a command or summary projection is removed in a controlled repair test
- **THEN** replaying retained journal events SHALL reproduce the same normalized projection and terminal state

#### Scenario: Retention expires evidence

- **WHEN** evidence exceeds configured retention
- **THEN** scheduled maintenance SHALL remove expired journal and projection rows without scanning the complete store on every event
- **AND** later queries SHALL expose the resulting oldest available boundary

#### Scenario: Application upgrades existing data

- **WHEN** an existing database is migrated to this capability
- **THEN** existing messages, traces, usage, logs, reviews, and workspace data SHALL remain readable
- **AND** the migration SHALL NOT synthesize native evidence for historical message activity
