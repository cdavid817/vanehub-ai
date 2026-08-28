# usage-statistics Specification

## Purpose
Defines the first-version usage statistics capability for summarizing persisted VaneHub chat message token usage in the settings center, including supported ranges, aggregation semantics, and documented accounting constraints.
## Requirements
### Requirement: Usage statistics summary
The system SHALL provide separated reported, reported-derived, and estimated usage statistics projected from invocation-grained accounting observations.

#### Scenario: Display reported token usage
- **WHEN** usage statistics are requested for a supported time range containing provider-reported usage
- **THEN** the system SHALL return authoritative total Tokens and available input, output, cached-input, cache-write-input, and reasoning-output dimensions
- **AND** it SHALL respect each provider adapter's declared overlap semantics instead of unconditionally summing every dimension

#### Scenario: Keep estimated activity separate
- **WHEN** the selected range contains cumulative-snapshot deltas or character estimates
- **THEN** the system SHALL return reported-derived Tokens and estimated characters separately from directly reported Tokens
- **AND** it SHALL NOT add estimated characters to a Token total

#### Scenario: Display coverage and breakdowns
- **WHEN** usage statistics are requested for a supported time range
- **THEN** the system SHALL return quality coverage, counted calls, generations and sessions, daily trend points, and breakdowns by stable Agent id, provider, model, purpose, and invocation status
- **AND** internal-purpose consumption SHALL remain distinguishable from user-response consumption

#### Scenario: Handle no usage data
- **WHEN** no persisted usage observations exist in the selected range
- **THEN** the system SHALL return zero-valued totals and coverage with empty trend and breakdown arrays instead of failing the page

### Requirement: Usage time ranges
The system SHALL support usage time ranges for today, last seven days, last thirty days, and all time using the active runtime's user-local calendar.

#### Scenario: Filter by bounded local-calendar range
- **WHEN** a user selects today, last seven days, or last thirty days
- **THEN** the system SHALL include usage whose occurrence time falls within that many local calendar dates including the current local date
- **AND** desktop and Web/mock runtimes SHALL apply equivalent local-calendar semantics

#### Scenario: Include all persisted usage
- **WHEN** a user selects all time
- **THEN** the system SHALL include all persisted VaneHub usage records without a lower date boundary

### Requirement: First-version accounting constraints
The system SHALL document and display that statistics cover VaneHub-observed invocations, preserve measurement quality and provider semantics, and are not provider billing reconciliation.

#### Scenario: Show accounting limitation
- **WHEN** the Usage Statistics page renders
- **THEN** it SHALL identify reported, reported-derived, and estimated sources
- **AND** it SHALL explain that unsupported external history, delayed provider records, unknown field semantics, and provider invoice adjustments can differ from VaneHub totals

#### Scenario: Filter without implying billing precision
- **WHEN** a user filters by Agent, provider, model, purpose, quality, or status
- **THEN** the page SHALL apply the selected dimensions to VaneHub observations
- **AND** it SHALL continue displaying the accounting limitation

### Requirement: Normalized response usage records
The system SHALL project zero or more normalized invocation observations to a VaneHub assistant response without storing prompt or response content in accounting records.

#### Scenario: Persist reported tokens
- **WHEN** a supported runtime reports valid non-zero usage for one assistant response invocation
- **THEN** the system SHALL project the invocation's normalized Token usage to that response

#### Scenario: Persist reported tokens for an interactive terminal session
- **WHEN** a supported CLI runs as an interactive embedded-terminal session
- **THEN** the system SHALL ingest the finest verified provider-native event or cumulative source available
- **AND** repeated polling or reopening SHALL remain idempotent through stable source keys and persisted cursors

#### Scenario: Materialize provider log revisions
- **WHEN** a provider session log contains multiple revisions for one provider message or a snapshot replacing prior messages
- **THEN** the system SHALL materialize the latest provider state before projecting usage
- **AND** superseded revisions SHALL remain auditable without contributing twice

#### Scenario: Preserve cache-only reported usage
- **WHEN** a provider reports positive cached-input or cache-write-input usage while input and output counts are zero
- **THEN** the system SHALL preserve the valid non-zero observation according to provider semantics

#### Scenario: Project multiple invocations
- **WHEN** a response requires multiple tool or retry invocations
- **THEN** the response projection SHALL aggregate each unique invocation exactly once
- **AND** request-level detail SHALL remain queryable

#### Scenario: Persist successful fallback estimate
- **WHEN** a visible assistant response completes successfully without valid reported usage
- **THEN** the system SHALL persist its input and output character counts as estimated activity

#### Scenario: Avoid incomplete fabricated estimate
- **WHEN** an assistant response fails or is cancelled without reported usage
- **THEN** the system SHALL NOT create a completed-response character estimate
- **AND** any valid usage reported by failed or cancelled provider invocations SHALL remain accounted

#### Scenario: Upgrade estimate to reported data
- **WHEN** reported usage later becomes available for an invocation represented by an estimate
- **THEN** the reported observation SHALL supersede that estimate in projections
- **AND** both records SHALL remain auditable without contributing twice

#### Scenario: Treat degenerate zero usage as unreported
- **WHEN** all provider usage categories and authoritative total are zero
- **THEN** the system SHALL treat that payload as no valid reported observation unless its provider contract explicitly permits a meaningful zero

#### Scenario: Fold reasoning tokens into reported output
- **WHEN** a provider reports reasoning separately from output
- **THEN** the first-version query contract SHALL preserve reasoning as its own dimension and SHALL NOT fold it into output
- **AND** totals SHALL respect the adapter's overlap semantics without folding reasoning into output

### Requirement: Historical usage quality preservation
The system SHALL preserve positive legacy message usage as estimated character history during migration.

#### Scenario: Backfill legacy message usage
- **WHEN** the usage-record migration runs on an existing database
- **THEN** each assistant message with positive legacy input or output values SHALL produce an idempotent estimated-character usage record attributed to its owning Agent and original creation time

#### Scenario: Preserve empty legacy rows
- **WHEN** an existing assistant message has no positive legacy usage value
- **THEN** the migration SHALL NOT create a synthetic usage record for that message

### Requirement: Session usage summary
The system SHALL provide a session-scoped summary and invocation breakdown without changing global range behavior.

#### Scenario: Return reported session totals
- **WHEN** session usage is requested
- **THEN** the system SHALL return reported, reported-derived, and estimated totals for only that session
- **AND** it SHALL return user-response and internal-purpose call counts and consumption separately

#### Scenario: Keep estimated session activity separate
- **WHEN** a session contains estimated character activity
- **THEN** the session summary SHALL keep it separate from reported and reported-derived Token totals

#### Scenario: Prefer reported tokens in compact panel
- **WHEN** a session contains more than one accounting quality
- **THEN** the compact panel SHALL prioritize provider-reported Token totals while preserving access to derived and estimated coverage

#### Scenario: Return session dimensions
- **WHEN** a session contains multiple Agents, providers, models, purposes, qualities, or statuses
- **THEN** the summary SHALL expose bounded breakdowns for those observed dimensions

#### Scenario: Handle session with no usage
- **WHEN** session usage is requested for a session with no observations
- **THEN** the system SHALL return zero-valued totals, coverage and call counts instead of failing

#### Scenario: Reject or isolate unknown session usage
- **WHEN** session usage is requested for an unknown, deleted, or inaccessible session
- **THEN** the system SHALL return a bounded service error or zero isolated result according to session policy
- **AND** it SHALL NOT expose another session's accounting data

#### Scenario: Refresh a running session's summary periodically
- **WHEN** an interactive terminal remains open after its usage was last observed
- **THEN** periodic ingestion SHALL update the session projection from new event-level usage or cumulative deltas
- **AND** previously projected historical intervals SHALL not move or duplicate

### Requirement: Persist reported tokens for managed Antigravity CLI invocations
The system SHALL persist provider-reported token usage for `antigravity-cli` managed (non-interactive) chat invocations from the terminal `result` event's usage object, mapping input tokens to fresh input, cache-read tokens to cache read, and folding reasoning (`thinking`) tokens into the output count, consistent with how reasoning tokens are already folded for `codex-cli`, `opencode`, and `gemini-cli`.

#### Scenario: Reported usage recorded from a completed invocation
- **WHEN** a managed `antigravity-cli` invocation completes and its `result` event carries a usage object
- **THEN** the system SHALL persist a reported-usage record for that response with fresh-input, output, and cache-read token counts derived from that object
- **AND** the response SHALL be counted as reported rather than estimated

#### Scenario: Reasoning tokens fold into output
- **WHEN** a `result` event's usage object reports a non-zero thinking-token count
- **THEN** the persisted output token count SHALL include those thinking tokens
- **AND** they SHALL NOT be persisted as a separate reported category

#### Scenario: Missing usage falls back to estimation
- **WHEN** a managed `antigravity-cli` invocation completes without a usage object
- **THEN** the system SHALL fall back to character-based estimation for that response
- **AND** the response SHALL NOT be counted as reported

### Requirement: Session-run report usage projection

The sessions-owned usage read model SHALL provide bounded usage summaries for a session-run report using the same reported, reported-derived, estimated, purpose, provider, model, status, and overlap semantics as existing session usage statistics.

#### Scenario: Report one or more runs

- **WHEN** a session-run report requests usage for retained run ids
- **THEN** the usage read model SHALL return observations correlated to those runs and the selected session
- **AND** it SHALL keep user-response and internal-purpose consumption distinguishable

#### Scenario: Run correlation is unavailable

- **WHEN** a session usage observation cannot be safely attributed to one requested run
- **THEN** the report usage section SHALL mark its run-level coverage partial or unavailable
- **AND** it SHALL not assign the observation to a run solely by timestamp proximity

#### Scenario: Report has estimated activity

- **WHEN** a report scope contains estimated character activity
- **THEN** the report SHALL display it separately from reported and reported-derived Tokens
- **AND** it SHALL not add characters to a Token total

#### Scenario: Chat messages are partly loaded

- **WHEN** the frontend has loaded only a subset of session messages
- **THEN** the report usage summary SHALL still come from persisted usage observations through the sessions service
- **AND** it SHALL not be recalculated from mounted React messages

### Requirement: Usage coverage without fabricated cost

Session-run report usage SHALL expose measurement quality and coverage and SHALL not claim monetary billing precision without an explicitly versioned pricing observation.

#### Scenario: Provider-reported usage exists

- **WHEN** the report scope contains provider-reported usage
- **THEN** the report SHALL expose authoritative reported dimensions and the adapter's declared overlap semantics
- **AND** it SHALL identify any report calls or generations not covered by reported usage

#### Scenario: Pricing metadata is absent

- **WHEN** Token usage exists but no versioned provider-pricing observation is available
- **THEN** monetary cost SHALL be absent or identified as unavailable
- **AND** the report SHALL retain the existing explanation that VaneHub observations are not provider invoice reconciliation

#### Scenario: Pricing metadata is introduced later

- **WHEN** a future approved capability provides versioned pricing observations
- **THEN** usage reporting MAY consume them through a published contract
- **AND** this capability SHALL not infer or fetch mutable prices implicitly during a report query

