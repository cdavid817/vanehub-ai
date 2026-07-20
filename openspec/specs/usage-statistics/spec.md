# usage-statistics Specification

## Purpose
Defines the first-version usage statistics capability for summarizing persisted VaneHub chat message token usage in the settings center, including supported ranges, aggregation semantics, and documented accounting constraints.
## Requirements
### Requirement: Usage statistics summary
The system SHALL provide separated reported-token and estimated-character usage statistics for VaneHub-managed assistant responses.

#### Scenario: Display reported token usage
- **WHEN** usage statistics are requested for a supported time range containing provider-reported usage
- **THEN** the system SHALL return reported fresh-input, output, cache-read, cache-creation, and total token counts
- **AND** reported total tokens SHALL equal the sum of those four token categories

#### Scenario: Keep estimated activity separate
- **WHEN** the selected range contains assistant responses whose usage is derived from character counting
- **THEN** the system SHALL return estimated input, output, and total character counts separately from reported token counts
- **AND** estimated characters SHALL NOT be added to any reported token total

#### Scenario: Display coverage and breakdowns
- **WHEN** usage statistics are requested for a supported time range
- **THEN** the system SHALL return reported, estimated, and total counted response counts, counted sessions, daily trend points, and per-Agent breakdown rows keyed by stable Agent id
- **AND** it SHALL return the percentage of counted responses backed by reported usage

#### Scenario: Handle no usage data
- **WHEN** no persisted usage records exist in the selected range
- **THEN** the system SHALL return zero-valued reported, estimated, coverage, response, and session totals with empty trend and Agent breakdown arrays instead of failing the page

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
The system SHALL document and display that usage statistics cover VaneHub-managed sessions, distinguish reported tokens from estimated characters, and are not provider billing records.

#### Scenario: Show accounting limitation
- **WHEN** the Usage Statistics page renders
- **THEN** it SHALL show localized explanatory text describing reported and estimated sources
- **AND** it SHALL state that external-terminal history, billing reconciliation, monetary cost estimation, request-detail logs, and provider/model filtering are not included in this version

### Requirement: Normalized response usage records
The system SHALL persist at most one normalized usage record per VaneHub assistant response without storing prompt or response content in that record.

#### Scenario: Persist reported tokens
- **WHEN** a supported CLI reports valid usage for an assistant response
- **THEN** the system SHALL persist non-negative normalized token categories with accounting kind `reported`, unit `tokens`, stable Agent id, source, and occurrence time

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

### Requirement: Historical usage quality preservation
The system SHALL preserve positive legacy message usage as estimated character history during migration.

#### Scenario: Backfill legacy message usage
- **WHEN** the usage-record migration runs on an existing database
- **THEN** each assistant message with positive legacy input or output values SHALL produce an idempotent estimated-character usage record attributed to its owning Agent and original creation time

#### Scenario: Preserve empty legacy rows
- **WHEN** an existing assistant message has no positive legacy usage value
- **THEN** the migration SHALL NOT create a synthetic usage record for that message

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

