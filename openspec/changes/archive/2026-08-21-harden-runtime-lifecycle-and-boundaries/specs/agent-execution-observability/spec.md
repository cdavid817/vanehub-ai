## ADDED Requirements

### Requirement: Observable telemetry persistence failure
Execution telemetry failures MUST remain non-blocking to the owning operation and SHALL produce bounded, redacted local diagnostics without recursively using the failing telemetry path.

#### Scenario: Span start or finish fails
- **WHEN** local persistence or export rejects a span or run transition
- **THEN** the Agent operation SHALL continue according to its own outcome and the unified log SHALL receive a safe failure classification

