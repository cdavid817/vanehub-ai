## ADDED Requirements

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
