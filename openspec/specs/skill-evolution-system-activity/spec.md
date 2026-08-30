# skill-evolution-system-activity Specification

## Purpose
Defines system-owned read-only Skill-evolution activity sessions and deterministic projections that make committed background outcomes visible, localized, rebuildable, and safely navigable without becoming an execution or governance channel.

## Requirements

### Requirement: Stable system activity session identity
The system SHALL maintain at most one Skill Evolution activity session for each canonical workspace and one global activity session for global Skill scope. Identity SHALL derive from scope kind and canonical scope id rather than an Agent id or display title.

#### Scenario: Workspace activity first appears
- **WHEN** the first project-scoped evolution event commits for a canonical workspace
- **THEN** the system lazily creates or resolves that workspace's stable activity session

#### Scenario: Equivalent workspace aliases are used
- **WHEN** different path aliases resolve to the same canonical workspace
- **THEN** their activity projects into the same system session

### Requirement: System sessions are not Agent sessions
A Skill Evolution system session SHALL have system-activity kind, no Agent id, no seats, no interaction mode, no provider runtime id, no terminal, and no effect on active Agent workflow state.

#### Scenario: User opens a system session
- **WHEN** the system activity view is selected
- **THEN** no Agent becomes active, available, launched, resumed, stopped, or reconfigured

#### Scenario: Agent session counts are displayed
- **WHEN** active or archived interactive-session totals are calculated
- **THEN** system activity sessions are excluded from those totals

### Requirement: Lazy and durable system-session lifecycle
System sessions SHALL be created only after committed eligible activity, SHALL persist their stable identity and preferences, and SHALL remain separately listable even when detailed retained activity becomes empty. Users MUST NOT create arbitrary system sessions.

#### Scenario: Workspace has no evolution activity
- **WHEN** system sessions are listed
- **THEN** no empty workspace system session is fabricated unless policy explicitly keeps a previously created session visible

#### Scenario: User attempts creation
- **WHEN** a client submits a normal create-session request with system-activity kind
- **THEN** the system rejects it

### Requirement: System-authored append-only content
Only the native result projector SHALL append activity items. Users and model runtimes MUST NOT create, edit, replace, reorder, or delete individual activity items.

#### Scenario: User attempts to send a message
- **WHEN** a send-message command targets a system activity session
- **THEN** the service rejects it before any Agent or provider invocation

#### Scenario: Source result is superseded
- **WHEN** an authoritative evolution result changes through a new committed revision
- **THEN** the projector appends a supersession item rather than rewriting the earlier activity item

### Requirement: Canonical safe activity envelope
Every projected result SHALL use a versioned locale-neutral envelope containing event id and code, source domain/id/revision, canonical scope, occurred and committed times, severity, status, safe identities, counts, reason codes, navigation descriptor, supersession relation, payload schema, and content hash.

#### Scenario: Domain event is projected
- **WHEN** a supported committed source record becomes eligible
- **THEN** the projector validates and persists one canonical safe envelope before target-specific rendering

#### Scenario: Unknown envelope version is encountered
- **WHEN** a projector receives an unsupported event or payload version
- **THEN** it records a projection-health failure and does not guess display data

### Requirement: Supported evolution event taxonomy
The projector SHALL support events for orchestration runs and stages; evidence signals and seed readiness; assessment selection and routing; dossier and generation jobs; Curator candidate lifecycle and decisions; Overlay previews and applications; automatic eligibility and applications; probation and circuit breakers; generated Skill creation; recovery and reconciliation; and retention or purge outcomes.

#### Scenario: Routine internal retry occurs
- **WHEN** a subsystem retries without a new committed user-relevant state
- **THEN** the projector updates health counters or suppresses the duplicate rather than adding repetitive activity

### Requirement: Committed-source-only projection
The system SHALL project only authoritative committed source revisions and SHALL never derive product activity by parsing unified log files, terminal scrollback, model transcripts, notification text, or frontend state.

#### Scenario: Source transaction rolls back
- **WHEN** an evolution operation emits transient diagnostics but does not commit
- **THEN** no successful activity result is projected

### Requirement: Privacy-safe projection
Activity envelopes and payloads MUST exclude raw prompts, messages, correction bodies, terminal output, tool arguments, credentials, paths, diffs, model prompts/responses, generated draft content, evidence excerpts, and unsafe notes. Free text SHALL be sanitized and bounded before envelope persistence or delivery.

#### Scenario: Curator rejection has a user note
- **WHEN** a rejection event is projected
- **THEN** the activity includes its safe category but not the optional note body

#### Scenario: Overlay application succeeds
- **WHEN** an application event is projected
- **THEN** it includes safe Skill identity, scope, revision references, provenance, and navigation without instruction diff content

### Requirement: Idempotent ordered projection
Projection SHALL be idempotent by source domain, source id, source revision, event code, projection version, and target. Items SHALL have a deterministic order by committed time, source sequence, and event id.

#### Scenario: Source event is replayed
- **WHEN** startup catch-up receives an event already projected
- **THEN** no duplicate timeline item, unread increment, dashboard count, or notification is produced

#### Scenario: Events share a timestamp
- **WHEN** multiple committed events have equal timestamps
- **THEN** source sequence and event id produce stable ordering

### Requirement: Locale-neutral storage and localized rendering
Stored activity SHALL use event and reason codes plus safe parameters. Titles, summaries, statuses, and accessible labels SHALL be localized at render time in every supported application locale with a documented fallback.

#### Scenario: User changes locale
- **WHEN** an existing system timeline is reopened after a locale change
- **THEN** historical items render in the new locale without rewriting persisted envelopes

#### Scenario: Translation key is unavailable
- **WHEN** a projection code lacks the active-locale string
- **THEN** the UI uses the shared fallback and preserves safe code visibility for diagnosis

### Requirement: Structured system activity items
The system SHALL render each activity item from its envelope as a bounded timeline entry with optional supported read-only Rich Blocks for status card, checklist, stage timeline, metrics, or navigation links. It MUST NOT render executable HTML, interactive mutation controls, or raw diffs.

#### Scenario: Evolution run completes partially
- **WHEN** a partial run event is displayed
- **THEN** the item shows stage results, counts, budget reason, and navigation using read-only structured blocks

### Requirement: Multi-target result projection
One canonical envelope MAY project to the system session, Skill Evolution dashboard summary, unread badge, and notification service. Each target SHALL have an independent idempotent delivery receipt and target-specific filtering without changing the canonical envelope.

#### Scenario: Notification policy suppresses routine event
- **WHEN** an informational event is eligible for timeline and dashboard but below notification threshold
- **THEN** timeline and dashboard deliveries commit while notification is recorded as policy-suppressed

### Requirement: Read cursor and unread counts
Unread state SHALL be maintained separately from append-only activity using a per-session user read cursor. Marking read or unread MUST NOT mutate source events or activity envelopes.

#### Scenario: User opens newest activity
- **WHEN** the user marks the visible timeline through its newest sequence as read
- **THEN** unread counts update without changing event history

#### Scenario: Older event is rebuilt
- **WHEN** rebuild restores an event at or before the read cursor
- **THEN** it does not become newly unread solely because of rebuild

### Requirement: Activity filtering and safe search
The system SHALL provide paginated filtering by time, severity, event domain, status, Skill, run, Curator state, and attention requirement, plus search over localized codes and safe identity fields. It MUST NOT search or reveal unprojected sensitive source content.

#### Scenario: Search matches a Skill id
- **WHEN** the user searches a safe Skill identity
- **THEN** matching activity items are returned in stable timeline order

### Requirement: Navigation-only result links
Activity navigation descriptors SHALL use allowlisted target kinds and stable ids for run, evidence, assessment, dossier, generation job, Curator candidate, Overlay history, Skill, probation, or breaker detail. Activating a link MUST NOT perform a state-changing action.

#### Scenario: User opens Curator link
- **WHEN** an attention item is activated
- **THEN** the application opens the candidate review surface without approving, rejecting, deferring, or applying it

### Requirement: Projection preferences
The system SHALL provide versioned per-scope preferences for session visibility, minimum timeline severity, attention notification threshold, digest cadence, read-state retention, detailed activity retention, and export limits. Preference changes MUST NOT delete authoritative evolution records.

#### Scenario: User hides routine activity session
- **WHEN** visibility is disabled
- **THEN** projection may continue under retention policy while normal navigation hides the session and attention notifications follow their independent setting

### Requirement: Projection health and lag
The system SHALL expose projector state, lease owner, last successful source cursor by domain, pending count, oldest pending age, failed event categories, gaps, rebuild state, and last completed projection time. The maintenance UI SHALL present lease state, per-domain cursor sequence, pending count, oldest pending time, gap and failure codes, and a bounded recent rebuild history using locale-aware labels without exposing source payloads.

#### Scenario: One source domain is delayed
- **WHEN** generation events lag while other domains project normally
- **THEN** health identifies the affected domain without marking unrelated source outcomes failed

#### Scenario: Operator inspects unhealthy projection state
- **WHEN** a domain has pending work, a source gap, or a projection failure
- **THEN** the maintenance UI identifies that domain and displays its safe cursor, backlog, gap, and failure diagnostics

#### Scenario: Recent rebuild evidence is available
- **WHEN** projection health includes rebuild records
- **THEN** the maintenance UI shows a bounded recent history with scope identity, status, and processed-item totals

### Requirement: Gap detection and bounded catch-up
The projector SHALL detect missing source sequences or invalid cursors, stop advancement past an unsafe gap for that domain, and perform bounded catch-up after startup without blocking application readiness.

#### Scenario: Startup has a large backlog
- **WHEN** retained committed events exceed one catch-up budget
- **THEN** the projector checkpoints progress and continues later while showing lag

### Requirement: Deterministic projection rebuild
The system SHALL support scoped rebuild from retained authoritative audit records into a new projection generation, validate counts and hashes, and atomically activate the rebuilt generation. Rebuild MUST NOT call models, rerun assessments, modify Skills or Overlays, resend already delivered notifications, or change governance decisions. While a user-requested rebuild is active, the maintenance UI SHALL display its current phase and processed-item progress, prevent a duplicate start, and provide cancellation through the system-activity service boundary. Cancelling MUST leave the previous valid generation available.

#### Scenario: User requests workspace rebuild
- **WHEN** projection health reports corruption or a version upgrade requires rebuild
- **THEN** the system creates a bounded rebuild attempt and keeps the last valid generation readable until replacement validates

#### Scenario: Rebuild output differs unexpectedly
- **WHEN** source receipts and projection version predict a different count or hash
- **THEN** activation fails and the previous valid generation remains active

#### Scenario: Rebuild advances through maintenance phases
- **WHEN** a rebuild processes, validates, catches up, or activates a shadow generation
- **THEN** the maintenance UI reports the current phase and processed items and does not allow another rebuild to start concurrently

#### Scenario: User cancels an active rebuild
- **WHEN** the user requests cancellation while a rebuild is in progress
- **THEN** the system cancels through the service boundary, reports completion of cancellation, and keeps the previous projection available

### Requirement: Activity retention and purge
Detailed activity SHALL follow bounded preference and underlying evolution retention. Source purge SHALL remove or redact derived detail and SHALL append or preserve a non-sensitive purge tombstone where required to explain committed Skill or Overlay history.

#### Scenario: Evidence is purged
- **WHEN** detailed source evidence is deleted
- **THEN** linked activity loses sensitive drill-down eligibility while safe committed outcome references remain consistent

### Requirement: Sanitized activity export
The system SHALL export selected activity as deterministic JSON or Markdown with scope, filters, projection version, completeness, redaction, and hash metadata through the normal export boundary.

#### Scenario: User exports filtered timeline
- **WHEN** a time and severity filter is active
- **THEN** the export contains only matching safe projected items and declares the filter and completeness

### Requirement: Projection failure isolation
Projection, timeline, read-state, notification, rebuild, or export failure MUST NOT alter authoritative evolution state, retry a mutation, call a model, or affect Agent execution and Skill loading.

#### Scenario: Projection database write fails
- **WHEN** an evolution operation already committed
- **THEN** its source result remains authoritative and the projector retries only safe projection work

### Requirement: Desktop and Web projection parity
Desktop SHALL provide durable SQLite projection and background catch-up while the process lives. Web/mock SHALL provide behaviorally equivalent in-memory sessions, timeline, read state, preferences, lag, rebuild simulation, and export, while explicitly not claiming durable native background processing.

#### Scenario: Web page reloads
- **WHEN** the mock runtime resets its in-memory activity
- **THEN** the UI preserves contract behavior but does not claim desktop durability
