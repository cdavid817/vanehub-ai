## MODIFIED Requirements

### Requirement: Summarization compaction
When compaction triggers, the system SHALL keep a fixed number of the most recent turns verbatim and SHALL replace all older turns with a single synthetic turn containing a model-generated summary of them.

#### Scenario: Older turns replaced by a summary
- **WHEN** compaction triggers
- **THEN** the system SHALL call the configured provider once to summarize the turns older than the kept window
- **AND** it SHALL replace those turns with one turn carrying the summary text
- **AND** the kept recent turns SHALL remain unchanged

#### Scenario: Summarization call does not declare tools
- **WHEN** the system makes the summarization call
- **THEN** the request SHALL NOT declare any tools
