## ADDED Requirements

### Requirement: Performance evidence correlates with execution without blocking it
Performance evidence for an Agent Run SHALL use existing Run, operation, span, and dataset correlations and SHALL remain metadata-only, bounded, and non-blocking to the owning execution.

#### Scenario: Evidence recording fails
- **WHEN** a performance measurement cannot be persisted or exported
- **THEN** the Run SHALL continue according to its canonical outcome
- **AND** unified logging SHALL receive a bounded redacted failure classification without recursive telemetry

#### Scenario: Run performance result is exported
- **WHEN** dedicated benchmark evidence is produced for a Run
- **THEN** it SHALL identify commit, platform, profile, dataset, metric, baseline, delta, and correlation ids without captured execution content

