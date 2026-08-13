## MODIFIED Requirements

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
