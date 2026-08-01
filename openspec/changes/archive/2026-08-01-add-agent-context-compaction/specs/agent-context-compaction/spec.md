## ADDED Requirements

### Requirement: Character-count compaction trigger
The system SHALL measure a generation's accumulated turns by summed character count and SHALL trigger compaction when that count exceeds a fixed threshold. The system SHALL NOT depend on real provider-reported token counts to make this determination.

#### Scenario: Below threshold, no compaction
- **WHEN** a generation's accumulated turns are below the character-count threshold
- **THEN** the system SHALL send the request unmodified

#### Scenario: Threshold crossed by session history
- **WHEN** a session's conversation history alone exceeds the character-count threshold
- **THEN** the system SHALL compact before sending the first request of that generation

#### Scenario: Threshold crossed during a tool-use loop
- **WHEN** turns accumulated during a generation's tool-use loop (tool call results) cause the running total to exceed the threshold
- **THEN** the system SHALL compact before sending the loop's next request

### Requirement: Summarization compaction
When compaction triggers, the system SHALL keep a fixed number of the most recent turns verbatim and SHALL replace all older turns with a single synthetic turn containing a model-generated summary of them.

#### Scenario: Older turns replaced by a summary
- **WHEN** compaction triggers
- **THEN** the system SHALL call the configured provider once to summarize the turns older than the kept window
- **AND** it SHALL replace those turns with one turn carrying the summary text
- **AND** the kept recent turns SHALL remain unchanged

#### Scenario: Summarization call does not declare tools
- **WHEN** the system makes the summarization call
- **THEN** the request SHALL NOT declare the shell or file tools

### Requirement: Visible compaction notice
The system SHALL insert a visible, distinctly-rendered notice into the chat transcript whenever compaction happens, reusing the existing rich-block mechanism.

#### Scenario: Notice appears in the transcript
- **WHEN** compaction happens during a generation
- **THEN** the system SHALL emit a rich block noting that earlier turns were condensed
- **AND** it SHALL persist and render through the same path existing rich blocks already use

### Requirement: Web runtime compaction parity
The Web/mock runtime SHALL simulate a deterministic compaction notice for sufficiently long mock sessions without making a real provider call.

#### Scenario: Web mock compaction notice
- **WHEN** a mock session's simulated history is long enough to exceed the mock threshold
- **THEN** the Web adapter SHALL emit a deterministic simulated compaction notice through the same event contract the desktop runtime uses
- **AND** it SHALL NOT call a real provider to produce it
