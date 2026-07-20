## ADDED Requirements

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
