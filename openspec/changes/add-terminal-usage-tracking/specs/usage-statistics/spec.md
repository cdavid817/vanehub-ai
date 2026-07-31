## MODIFIED Requirements

### Requirement: Normalized response usage records
The system SHALL persist at most one normalized usage record per VaneHub assistant response without storing prompt or response content in that record.

#### Scenario: Persist reported tokens
- **WHEN** a supported CLI reports valid usage for an assistant response
- **THEN** the system SHALL persist non-negative normalized token categories with accounting kind `reported`, unit `tokens`, stable Agent id, source, and occurrence time

#### Scenario: Persist reported tokens for an interactive terminal session
- **WHEN** a supported CLI runs as an interactive embedded-terminal session rather than through VaneHub's managed invocation pipeline
- **THEN** the system SHALL read that CLI's own persisted session log or database to obtain reported usage
- **AND** it SHALL persist that usage the same way as managed-pipeline usage, with accounting kind `reported` and unit `tokens`

#### Scenario: Persist successful fallback estimate
- **WHEN** a VaneHub assistant response completes successfully without valid reported usage
- **THEN** the system SHALL persist its input and output character counts with accounting kind `estimated` and unit `characters`

#### Scenario: Avoid incomplete fabricated estimate
- **WHEN** an assistant response fails or is cancelled without reported usage
- **THEN** the system SHALL NOT create an estimated usage record for that incomplete response

#### Scenario: Upgrade estimate to reported data
- **WHEN** reported usage later becomes available for a response that has an estimated record
- **THEN** the reported record SHALL replace the estimate
- **AND** an estimated observation SHALL NOT overwrite reported data

### Requirement: Session usage summary
The system SHALL provide a session-scoped usage summary for a single VaneHub-managed session without changing global usage statistics range behavior.

#### Scenario: Return reported session totals
- **WHEN** session usage is requested for a session with provider-reported usage records
- **THEN** the system SHALL return reported fresh-input, output, cache-read, cache-creation, and total token counts for only that session
- **AND** reported total tokens SHALL equal the sum of those four token categories

#### Scenario: Keep estimated session activity separate
- **WHEN** session usage is requested for a session with estimated usage records
- **THEN** the system SHALL return estimated input, output, and total character counts separately from reported token counts
- **AND** estimated characters SHALL NOT be added to any reported token total

#### Scenario: Prefer reported tokens in compact panel
- **WHEN** a session has both reported and estimated usage records
- **THEN** the session usage summary SHALL preserve both accounting kinds
- **AND** UI consumers SHALL be able to use reported token totals as the primary displayed usage value

#### Scenario: Handle session with no usage
- **WHEN** session usage is requested for a session with no persisted usage records
- **THEN** the system SHALL return zero-valued reported, estimated, coverage, and response totals instead of failing

#### Scenario: Reject or isolate unknown session usage
- **WHEN** session usage is requested for an unknown, deleted, or inaccessible session
- **THEN** the system SHALL return a bounded service error or zero isolated result according to the existing session service error policy
- **AND** it SHALL NOT expose usage data from other sessions

#### Scenario: Refresh a running session's summary periodically
- **WHEN** a session's interactive terminal remains open after its usage was last read
- **THEN** the system SHALL periodically re-read the CLI's own reported usage and refresh the persisted usage record without waiting for the session to stop
- **AND** the session usage summary SHALL reflect the refreshed values on its next request
