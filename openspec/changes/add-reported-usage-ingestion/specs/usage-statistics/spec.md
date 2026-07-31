## MODIFIED Requirements

### Requirement: Normalized response usage records
The system SHALL persist at most one normalized usage record per VaneHub assistant response without storing prompt or response content in that record.

#### Scenario: Persist reported tokens
- **WHEN** a supported CLI reports valid, non-zero usage for an assistant response
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

#### Scenario: Treat degenerate zero usage as unreported
- **WHEN** a supported CLI's completion signal for an assistant response carries a usage payload whose token categories are all zero
- **THEN** the system SHALL treat that response as without valid reported usage
- **AND** the system SHALL follow the successful fallback estimate scenario instead of persisting a reported record

#### Scenario: Fold reasoning tokens into reported output
- **WHEN** a supported CLI reports reasoning or thinking tokens separately from its output tokens for an assistant response
- **THEN** the system SHALL include those reasoning tokens in the persisted reported output token count
- **AND** the system SHALL NOT persist reasoning tokens as a distinct tracked category
