## MODIFIED Requirements

### Requirement: Normalized response usage records
The system SHALL persist at most one normalized usage record per VaneHub assistant response without storing prompt or response content in that record.

#### Scenario: Persist reported tokens
- **WHEN** a supported CLI reports valid, non-zero usage for an assistant response
- **THEN** the system SHALL persist non-negative normalized token categories with accounting kind `reported`, unit `tokens`, stable Agent id, source, and occurrence time

#### Scenario: Persist reported tokens for an interactive terminal session
- **WHEN** any of the four supported CLIs (claude-code, opencode, codex-cli, or gemini-cli) runs as an interactive embedded-terminal session rather than through VaneHub's managed invocation pipeline
- **THEN** the system SHALL read that CLI's own persisted session log or database to obtain reported usage
- **AND** it SHALL bind provider-native data to the exact provider runtime session when that identity is available
- **AND** repeated polling or reopening the same VaneHub session SHALL update one stable terminal usage record instead of adding prior cumulative usage again
- **AND** persistence failure SHALL be returned to the caller and recorded through unified logging

#### Scenario: Materialize provider log revisions
- **WHEN** a provider session log contains multiple revisions for the same provider message id or a snapshot replacing prior messages
- **THEN** the system SHALL materialize the provider's latest message state before aggregating usage
- **AND** it SHALL NOT count superseded revisions as additional responses

#### Scenario: Preserve cache-only reported usage
- **WHEN** a supported CLI reports a positive cache-read or cache-creation count while input and output counts are zero
- **THEN** the system SHALL persist the non-zero reported usage rather than treating the observation as empty

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

#### Scenario: Treat degenerate zero usage as unreported
- **WHEN** a supported CLI's completion signal for an assistant response carries a usage payload whose token categories are all zero
- **THEN** the system SHALL treat that response as without valid reported usage
- **AND** the system SHALL follow the successful fallback estimate scenario instead of persisting a reported record

#### Scenario: Fold reasoning tokens into reported output
- **WHEN** a supported CLI reports reasoning or thinking tokens separately from its output tokens for an assistant response
- **THEN** the system SHALL include those reasoning tokens in the persisted reported output token count
- **AND** the system SHALL NOT persist reasoning tokens as a distinct tracked category
