## ADDED Requirements

### Requirement: Usage statistics summary
The system SHALL provide a usage statistics summary based on persisted VaneHub chat message usage fields.

#### Scenario: Display aggregate token usage
- **WHEN** usage statistics are requested for a supported time range
- **THEN** the system SHALL return total token usage, input token usage, output token usage, counted assistant messages, and counted sessions
- **AND** the total token usage SHALL equal input token usage plus output token usage

#### Scenario: Handle no usage data
- **WHEN** no persisted messages with usage exist in the selected range
- **THEN** the system SHALL return zero-valued usage totals instead of failing the page

### Requirement: Usage time ranges
The system SHALL support bounded first-version usage time ranges for today, last seven days, last thirty days, and all time.

#### Scenario: Filter by time range
- **WHEN** a user selects a usage time range
- **THEN** the system SHALL aggregate only messages whose creation time is within that range, except all time which SHALL include all persisted messages

### Requirement: First-version accounting constraints
The system SHALL document and display that first-version usage statistics are based on stored VaneHub message usage fields and are not provider billing records.

#### Scenario: Show accounting limitation
- **WHEN** the Usage Statistics page renders
- **THEN** it SHALL show localized explanatory text that real tokenizer accounting, provider/model breakdowns, cache tokens, request logs, and cost estimation are not included in the first version
