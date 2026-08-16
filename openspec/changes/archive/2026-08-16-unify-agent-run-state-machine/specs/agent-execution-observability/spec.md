## ADDED Requirements

### Requirement: Canonical lifecycle correlation
Execution observability SHALL reuse the canonical Run id for lifecycle correlation while retaining independent trace/span identity and telemetry status. Canonical transition persistence SHALL NOT depend on OTLP exporter or timeline availability.

#### Scenario: Canonical Run transitions
- **WHEN** a correlated Run waits, retries, verifies, or terminates
- **THEN** observability records a bounded safe lifecycle event without replacing the canonical Run state or inventing unavailable child detail
