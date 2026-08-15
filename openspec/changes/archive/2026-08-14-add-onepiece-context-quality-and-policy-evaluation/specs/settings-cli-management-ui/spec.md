## ADDED Requirements

### Requirement: OnePiece context policy health panel
The OnePiece settings surface SHALL present localized context-policy health using service-provided history and aggregates, including evaluated decisions, compacted and bypassed outcomes, savings, measurement-quality coverage, fallback and failure distributions, active policy versions, range controls, and retention controls.

#### Scenario: User opens context health
- **WHEN** the OnePiece parameter page loads successfully
- **THEN** the panel SHALL show bounded aggregate values and recent assessment outcomes without rendering raw prompt, summary, tool, or secret content

#### Scenario: History is empty or unavailable
- **WHEN** no assessments exist or the service request fails
- **THEN** the panel SHALL show a distinct localized empty or error state without fabricating metrics or disabling unrelated settings

#### Scenario: User changes time range or retention
- **WHEN** the user selects a supported history range or retention option
- **THEN** the panel SHALL refresh through the settings and agent service boundaries with accessible loading and failure feedback

#### Scenario: Measurement quality is incomplete
- **WHEN** some assessments contain character-only or estimated measurements
- **THEN** the panel SHALL disclose quality coverage and SHALL state that savings are operational context metrics rather than provider billing reconciliation or proof of semantic answer quality

