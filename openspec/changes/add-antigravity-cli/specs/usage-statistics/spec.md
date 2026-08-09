## ADDED Requirements

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
